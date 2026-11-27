#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

die() { printf 'Error: %s\n' "$*" >&2; exit 1; }

usage() {
    cat <<'EOF'
Usage: tools/test-installer-qemu.sh [ISO...]

End-to-end test of the installer's USB writer. It loads the dummy_hcd
and g_mass_storage kernel modules so a file looks like a real USB disk
to lsblk, runs the real --write-usb writer against it, then boots the
result in QEMU and checks the picker's menu on the serial console.

Root is required for the kernel modules and the writer, so the script
asks for a sudo password. It never touches a physical drive. The picker
on the written disk is a text-mode build, because the graphics build
only draws to GOP and cannot be asserted headlessly.

Without arguments the smallest ISO in isos/ (ISO_DIR) is used. The ISO
must be one the picker lists, see docs/profiles.md.

Environment:
  EXCLUDE_TOOLS  comma-separated usb::TOOLS indices to leave out; the
                 default is to install every tool, UEFI SCT included
  EXPECT_ISO     0 to skip the "Boot images" assertion (default 1)
  SKIP_BOOT      1 to stop after the writer finishes
  KEEP_IMAGE     1 to keep the work directory instead of deleting it
  BOOT_TIMEOUT   seconds to wait for the menu, default 120
  OVMF_FIRMWARE  path to a non-Secure-Boot OVMF firmware file
  QEMU_MEMORY    guest RAM for the final boot, default 1G
EOF
}

case ${1:-} in
    -h|--help) usage; exit 0 ;;
esac

for command in cargo qemu-system-x86_64 lsblk sfdisk udevadm mkfs.fat mkfs.ext4 \
    mount umount truncate modprobe rmmod modinfo awk comm tee sudo; do
    command -v "$command" >/dev/null || die "Missing command: $command"
done

excluded=${EXCLUDE_TOOLS:-}
sct_included=1
for index in ${excluded//,/ }; do
    if [[ $index == 6 ]]; then
        sct_included=0
    fi
done
if (( sct_included )); then
    command -v bsdtar >/dev/null || die 'Install bsdtar for UEFI SCT, or set EXCLUDE_TOOLS=6.'
fi
if [[ ! -d /lib/modules/$(uname -r) ]]; then
    die "No kernel modules are installed for the running kernel $(uname -r).
After a kernel upgrade the old module directory is removed; reboot into
the installed kernel before running this test."
fi
modinfo dummy_hcd >/dev/null 2>&1 ||
    die 'The dummy_hcd kernel module is unavailable for this kernel; it ships with the kernel.'
modinfo g_mass_storage >/dev/null 2>&1 ||
    die 'The g_mass_storage kernel module is unavailable for this kernel; it ships with the kernel.'

isos=()
if (( $# )); then
    isos=("$@")
else
    shopt -s nullglob nocaseglob
    for iso in "${ISO_DIR:-isos}"/*.iso; do
        if [[ -f $iso ]]; then
            isos+=("$iso")
        fi
    done
    shopt -u nullglob nocaseglob
    if (( ${#isos[@]} )); then
        smallest=${isos[0]}
        for iso in "${isos[@]}"; do
            if (( $(stat -Lc %s -- "$iso") < $(stat -Lc %s -- "$smallest") )); then
                smallest=$iso
            fi
        done
        isos=("$smallest")
    fi
fi
(( ${#isos[@]} )) || die 'No ISO found. Copy one into isos/ or pass its path.'
total=0
for iso in "${isos[@]}"; do
    [[ -f $iso ]] || die "No such ISO: $iso"
    total=$((total + $(stat -Lc %s -- "$iso")))
done

target=x86_64-unknown-uefi
printf 'Building the text-mode picker and the installer...\n'
cargo build --locked --release --target "$target" --target-dir build/text-mode \
    --features text-mode
cargo build --locked --release --manifest-path installer/Cargo.toml --locked
installer=$PWD/installer/target/release/efi-sk-installer
[[ -x $installer ]] || die "Missing installer binary: $installer"

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

unmount_disk() {
    local target=$1 mountpoint
    local -a mounts=()
    mapfile -t mounts < <(lsblk -nrpo MOUNTPOINTS -- "$target" 2>/dev/null || true)
    for mountpoint in "${mounts[@]}"; do
        [[ -n ${mountpoint//[[:space:]]/} ]] || continue
        sudo umount -- "$mountpoint" 2>/dev/null || true
    done
}

usb_disks() {
    lsblk -dnpo PATH,TYPE,TRAN,RO |
        awk '$2 == "disk" && $3 == "usb" && $4 == "0" { print $1 }' | sort
}

work=$(mktemp -d "$PWD/build/installer-e2e.XXXXXXXX")
image=$work/usb.img
stage=$work/stage
writer_log=$work/writer.log
serial_log=$work/serial.log
mkdir -p -- "$stage"

privileged=0
gadget_loaded=0
dummy_loaded=0
disk=
qemu_pid=
cleanup() {
    local status=$?
    trap - EXIT
    if [[ -n $qemu_pid ]]; then
        kill "$qemu_pid" 2>/dev/null || true
        wait "$qemu_pid" 2>/dev/null || true
    fi
    if (( privileged )); then
        if [[ -n $disk ]]; then
            unmount_disk "$disk"
        fi
        if (( gadget_loaded )); then
            sudo rmmod g_mass_storage 2>/dev/null || true
        fi
        if (( dummy_loaded )); then
            sudo rmmod dummy_hcd 2>/dev/null || true
        fi
    fi
    if [[ ${KEEP_IMAGE:-0} == 1 ]]; then
        printf 'Keeping the test data in %s\n' "$work"
    else
        rm -rf -- "$work"
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# Stage what prepare_usb() puts in its temporary directory, but with the
# text-mode picker so the final boot can be asserted on serial.
cp -- "build/text-mode/$target/release/efi-sk.efi" "$stage/BOOTX64.EFI"
# The (name, vendor) pairs of usb::TOOLS, in order.
assets=(
    'memtest86plus.efi memtest86plus'
    'shell.efi edk2-shell'
    'KeyTool.efi keytool'
    'ipxe.efi ipxe'
    'acpidump.efi acpidump'
    'gdisk_x64.efi gpt-fdisk'
    'sct-x64.7z uefi-sct'
)
for asset in "${assets[@]}"; do
    read -r name vendor <<< "$asset"
    [[ -s vendor/$vendor/$name ]] || die "Missing vendor/$vendor/$name"
    cp -- "vendor/$vendor/$name" "$stage/$name"
done

# The writer wants the ISOs plus 768 MiB of headroom.
capacity=$((total + 1024 * 1024 * 1024))
truncate -s "$capacity" "$image"
printf 'Virtual USB disk: %s (%s MiB)\n' "$image" "$((capacity / 1048576))"

printf 'Authorizing sudo for the kernel modules and the USB writer...\n'
sudo -v || die 'sudo is required.'
privileged=1

usb_disks > "$work/usb-disks.before"
sudo modprobe dummy_hcd || die 'Could not load dummy_hcd.'
dummy_loaded=1
shopt -s nullglob
udcs=(/sys/class/udc/*)
shopt -u nullglob
(( ${#udcs[@]} )) || die 'dummy_hcd did not register a USB gadget controller.'
for udc in "${udcs[@]}"; do
    controller=${udc##*/}
    if [[ $controller != dummy_udc* ]]; then
        die "USB gadget controller $controller did not come from dummy_hcd; unload it first."
    fi
done

printf 'Backing the virtual USB disk with %s\n' "$image"
sudo modprobe g_mass_storage "file=$image" || die 'Could not load g_mass_storage.'
gadget_loaded=1

disk=
for _ in $(seq 1 50); do
    usb_disks > "$work/usb-disks.now"
    disk=$(comm -13 "$work/usb-disks.before" "$work/usb-disks.now" | head -n 1 || true)
    if [[ -n $disk ]]; then
        break
    fi
    sleep 0.2
done
if [[ -z $disk || ! -b $disk ]]; then
    printf 'Block devices now:\n' >&2
    lsblk -dnpo PATH,TYPE,TRAN,SIZE,MODEL,RO >&2 || true
    die 'The virtual USB disk did not appear as a writable USB disk.'
fi
printf 'Virtual USB disk: %s\n' "$disk"
lsblk -p -o NAME,SIZE,MODEL,TRAN,RO,MOUNTPOINTS -- "$disk" || true
unmount_disk "$disk"

name=${disk##*/}
[[ -r /sys/class/block/$name/diskseq ]] || die "No diskseq for $disk."
sequence=$(< "/sys/class/block/$name/diskseq")

printf 'Running the privileged USB writer on %s...\n' "$disk"
if ! sudo env "PKEXEC_UID=$(id -u)" "PATH=$PATH" "$installer" \
        --write-usb "$disk" "$sequence" "$stage" "$excluded" "${isos[@]}" 2>&1 |
        tee "$writer_log"; then
    die 'The USB writer failed.'
fi
for expected in \
    'progress: Partitioning USB drive' \
    'progress: Formatting partitions' \
    'progress: Installing EFI picker and tools' \
    'progress: Copying' \
    'progress: Flushing writes and unmounting'; do
    grep -qF -- "$expected" "$writer_log" || die "The writer never reported: $expected"
done

unmount_disk "$disk"
sudo rmmod g_mass_storage || die 'g_mass_storage is still busy; unmount its filesystems first.'
gadget_loaded=0
sudo rmmod dummy_hcd || true
dummy_loaded=0
disk=

if [[ ${SKIP_BOOT:-0} == 1 ]]; then
    printf 'SKIP_BOOT=1: not booting the written image.\n'
    printf 'PASS: the installer wrote a bootable USB image.\n'
    exit 0
fi

patterns=('EFI Swiss Knife' 'Tools' 'Up/Down: select')
if [[ ${EXPECT_ISO:-1} == 1 ]]; then
    patterns+=('Boot images')
fi
timeout=${BOOT_TIMEOUT:-120}
: > "$serial_log"

printf 'Booting the written image headlessly for up to %ss...\n' "$timeout"
qemu-system-x86_64 -machine 'q35,accel=kvm:tcg' -m "${QEMU_MEMORY:-1G}" \
    -bios "$firmware" \
    -drive "if=none,id=usb,format=raw,file=$image,snapshot=on" \
    -device qemu-xhci -device 'usb-storage,drive=usb' \
    -display none -serial "file:$serial_log" -nic none &
qemu_pid=$!

deadline=$((SECONDS + timeout))
missing=("${patterns[@]}")
while (( SECONDS < deadline )); do
    missing=()
    for pattern in "${patterns[@]}"; do
        if ! grep -qF -- "$pattern" "$serial_log" 2>/dev/null; then
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
    if [[ ' ${missing[*]} ' == *' Boot images '* ]]; then
        printf 'The writer succeeded, but the picker listed no ISO. Use an ISO the\n' >&2
        printf 'picker supports (docs/profiles.md), or set EXPECT_ISO=0.\n' >&2
    fi
    printf '\n--- %s ---\n' "$serial_log" >&2
    cat "$serial_log" >&2 || true
    exit 1
fi

printf 'PASS: the installer wrote a USB image that boots into the picker.\n'
if [[ ${KEEP_IMAGE:-0} == 1 ]]; then
    printf 'Writer log: %s\nSerial log: %s\n' "$writer_log" "$serial_log"
fi
