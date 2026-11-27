#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

die() { printf 'Error: %s\n' "$*" >&2; exit 1; }

usage() {
    cat <<'EOF'
Usage: tools/boot-image-qemu.sh [--read-write] [--disk] [QEMU options...]
       IMAGE [QEMU options...]

Boots a raw disk image under OVMF, for example the file written by
tools/test-installer-qemu.sh or build/qemu-cache/usb.img.

  --read-write  let the guest write to IMAGE; by default writes go to a
                temporary overlay that is discarded when QEMU exits
  --disk        attach IMAGE as a SATA disk instead of USB storage
  -h, --help    show this help

Every other argument is passed to qemu-system-x86_64. The default
display is GTK; pass -display none, -nographic, or -vnc to run without
a window, in which case the serial console goes to the terminal.
Networking uses QEMU's default user-mode backend; pass -nic none to
disable it. Set OVMF_FIRMWARE to use a specific firmware file.
EOF
}

read_write=0
attach=usb
args=()
while (($#)); do
    case $1 in
        -h|--help) usage; exit 0 ;;
        --read-write) read_write=1; shift ;;
        --disk) attach=disk; shift ;;
        -display|-vnc) args+=("$1" "${2-}"); shift 2 ;;
        -*) args+=("$1"); shift ;;
        *) break ;;
    esac
done

(( $# >= 1 )) || { usage >&2; exit 2; }
image=$1
shift
args+=("$@")
[[ -f $image ]] || die "No such image: $image"
command -v qemu-system-x86_64 >/dev/null || die 'Missing command: qemu-system-x86_64'

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

display=(-display gtk)
for arg in "${args[@]}"; do
    case $arg in
        -display|-display=*|-nographic|-vnc) display=(); break ;;
    esac
done
if (( ${#display[@]} )); then
    backends=$(qemu-system-x86_64 -display help)
    grep -qx gtk <<< "$backends" ||
        die 'QEMU GTK backend is missing. On Arch, install qemu-ui-gtk, or pass -display none.'
fi

# Without a window there is no virtual console left for the firmware's
# serial output, so give it the terminal instead.
serial=()
if (( ${#display[@]} == 0 )); then
    serial=(-serial mon:stdio)
    for arg in "${args[@]}"; do
        case $arg in
            -serial|-serial=*|-nographic) serial=(); break ;;
        esac
    done
fi

snapshot=on
if (( read_write )); then
    [[ -w $image ]] || die "Cannot write to $image"
    snapshot=off
fi

drive=(-drive "if=none,id=img,format=raw,file=$image,snapshot=$snapshot"
       -device qemu-xhci -device 'usb-storage,drive=img')
if [[ $attach == disk ]]; then
    drive=(-drive "file=$image,format=raw,snapshot=$snapshot")
fi

printf 'Booting %s under %s (%s, %s)\n' "$image" "$firmware" "$attach" \
    "$([[ $snapshot == on ]] && printf 'temporary overlay' || printf 'written back')"
exec qemu-system-x86_64 -machine 'q35,accel=kvm:tcg' -m "${QEMU_MEMORY:-4G}" \
    -bios "$firmware" \
    "${drive[@]}" \
    -device qemu-xhci -device usb-mouse \
    "${display[@]}" "${serial[@]}" "${args[@]}"
