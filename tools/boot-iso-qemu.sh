#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

die() { printf 'Error: %s\n' "$*" >&2; exit 1; }

usage() {
    cat <<'EOF'
Usage: tools/boot-iso-qemu.sh [--usb] [QEMU options...] ISO [QEMU options...]

Boots an ISO directly under OVMF, without the picker, so an installer
can be checked on its own.

  --usb       attach the ISO as USB storage instead of a SATA CD-ROM
  -h, --help  show this help

Every other argument is passed to qemu-system-x86_64. The default
display is GTK; pass -display none, -nographic, or -vnc to run without
a window, in which case the serial console goes to the terminal.
Networking uses QEMU's default user-mode backend; pass -nic none to
disable it. Set OVMF_FIRMWARE to use a specific firmware file.
EOF
}

usb=0
args=()
while (($#)); do
    case $1 in
        -h|--help) usage; exit 0 ;;
        --usb) usb=1; shift ;;
        -display|-vnc) args+=("$1" "${2-}"); shift 2 ;;
        -*) args+=("$1"); shift ;;
        *) break ;;
    esac
done

(( $# >= 1 )) || { usage >&2; exit 2; }
iso=$1
shift
args+=("$@")
[[ -f $iso ]] || die "No such ISO: $iso"
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

media=(-drive "file=$iso,media=cdrom,readonly=on")
if (( usb )); then
    media=(-drive "if=none,id=iso,format=raw,file=$iso,readonly=on"
           -device 'usb-storage,drive=iso,removable=on')
fi

printf 'Booting %s directly under %s\n' "$iso" "$firmware"
exec qemu-system-x86_64 -machine 'q35,accel=kvm:tcg' -m "${QEMU_MEMORY:-4G}" \
    -bios "$firmware" \
    -device qemu-xhci -device usb-mouse \
    "${media[@]}" \
    "${display[@]}" "${serial[@]}" "${args[@]}"
