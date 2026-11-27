#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

die() { printf 'Error: %s\n' "$*" >&2; exit 1; }

usage() {
    cat <<'EOF'
Usage: tools/smoke-test-qemu.sh

Boots a text-mode build of the picker headlessly and fails unless the
menu shows up on the serial console. The run needs no ISO files, no
display server, and no root.

Environment:
  OVMF_FIRMWARE  path to a non-Secure-Boot OVMF firmware file
  QEMU_MEMORY    guest RAM, default 1G
  SMOKE_TIMEOUT  seconds to wait for the menu, default 120

The serial log of the last run is kept in build/qemu-smoke/serial.log.
EOF
}

case ${1:-} in
    -h|--help) usage; exit 0 ;;
    '') ;;
    *) usage >&2; exit 2 ;;
esac

for command in cargo qemu-system-x86_64 truncate mkfs.fat mmd mcopy; do
    command -v "$command" >/dev/null || die "Missing command: $command"
done

firmware=${OVMF_FIRMWARE:-}
if [[ -z $firmware ]]; then
    for candidate in /usr/share/{edk2,ovmf,OVMF}/{,x64/,ovmf/}OVMF{,_CODE}{,.4m,_4M}.fd; do
        if [[ -r $candidate ]]; then
            firmware=$candidate
            break
        fi
    done
fi
[[ -r $firmware ]] || die 'OVMF not found; set OVMF_FIRMWARE to a non-Secure-Boot firmware file.'

target=x86_64-unknown-uefi
# The graphics build only writes its menu to GOP, which no headless run can
# observe, so the assertion below uses the text-mode build instead.
printf 'Building the text-mode picker...\n'
cargo build --locked --release --target "$target" --target-dir build/text-mode \
    --features text-mode

output=build/qemu-smoke
mkdir -p -- "$output"
log=$output/serial.log
: > "$log"

work=$(mktemp -d build/qemu-smoke-run.XXXXXXXX)
qemu_pid=
cleanup() {
    if [[ -n $qemu_pid ]]; then
        kill "$qemu_pid" 2>/dev/null || true
        wait "$qemu_pid" 2>/dev/null || true
    fi
    rm -rf -- "$work"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# A minimal EFI system partition: the picker is the only file on it.
image=$work/esp.img
truncate -s 64M "$image"
mkfs.fat -F 32 -s 1 -n SMOKE -- "$image" >/dev/null
mmd -i "$image" ::/EFI ::/EFI/BOOT
mcopy -i "$image" "build/text-mode/$target/release/efi-sk.efi" ::/EFI/BOOT/BOOTX64.EFI

patterns=(
    'EFI Swiss Knife'
    'Tools'
    'System Info'
    'Up/Down: select'
)
timeout=${SMOKE_TIMEOUT:-120}

printf 'Booting the picker headlessly for up to %ss...\n' "$timeout"
qemu-system-x86_64 -machine 'q35,accel=kvm:tcg' -m "${QEMU_MEMORY:-1G}" \
    -bios "$firmware" \
    -drive "if=none,id=esp,format=raw,file=$image" \
    -device qemu-xhci -device 'usb-storage,drive=esp' \
    -display none -serial "file:$log" -nic none &
qemu_pid=$!

deadline=$((SECONDS + timeout))
missing=("${patterns[@]}")
while (( SECONDS < deadline )); do
    missing=()
    for pattern in "${patterns[@]}"; do
        if ! grep -qF -- "$pattern" "$log" 2>/dev/null; then
            missing+=("$pattern")
        fi
    done
    if (( ${#missing[@]} == 0 )); then
        break
    fi
    if ! kill -0 "$qemu_pid" 2>/dev/null; then
        break
    fi
    sleep 1
done

kill "$qemu_pid" 2>/dev/null || true
wait "$qemu_pid" 2>/dev/null || true
qemu_pid=

if (( ${#missing[@]} )); then
    printf 'FAIL: the serial console did not show:\n' >&2
    for pattern in "${missing[@]}"; do
        printf '  - %s\n' "$pattern" >&2
    done
    printf '\n--- %s ---\n' "$log" >&2
    cat "$log" >&2 || true
    exit 1
fi

printf 'PASS: the text-mode picker rendered its menu on the serial console.\n'
printf 'Serial log: %s\n' "$log"
