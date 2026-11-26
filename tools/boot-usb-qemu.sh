#!/usr/bin/env bash
set -euo pipefail

usage() { echo "Usage: $0 [--read-write] /dev/USB_DISK"; }
if [[ ${1:-} == --help ]]; then usage; exit 0; fi
readonly_disk=on
if [[ ${1:-} == --read-write ]]; then readonly_disk=off; shift; fi
if (( $# != 1 )); then usage >&2; exit 2; fi
disk=$(realpath -e -- "$1")
if [[ ! -b $disk || $(lsblk -dnro TYPE,TRAN -- "$disk" | xargs) != 'disk usb' ]]; then
    echo "$disk is not a whole USB disk" >&2
    exit 1
fi
if [[ $readonly_disk == off ]]; then
    mounts=$(lsblk -nr -o MOUNTPOINTS -- "$disk")
    types=$(lsblk -nr -o TYPE -- "$disk")
    if [[ $mounts =~ [^[:space:]] ]] || grep -qvxE 'disk|part' <<< "$types"; then
        echo "Unmount all partitions and deactivate any mapped devices on $disk first." >&2
        exit 1
    fi
fi

firmware=${OVMF_FIRMWARE:-}
if [[ -z $firmware ]]; then
    for path in /usr/share/{OVMF/x64,edk2/x64}/OVMF.4m.fd /usr/share/OVMF/OVMF_CODE_4M.fd; do
        if [[ -r $path ]]; then firmware=$path; break; fi
    done
fi
[[ -r $firmware ]] || { echo 'Set OVMF_FIRMWARE to an x86-64 OVMF firmware file.' >&2; exit 1; }

qemu=(qemu-system-x86_64 -machine 'q35,accel=kvm:tcg' -m 2G -bios "$firmware"
      -add-fd 'fd=3,set=1'
      -blockdev "driver=host_device,node-name=stick,filename=/dev/fdset/1,read-only=$readonly_disk"
      -device qemu-xhci -device 'usb-storage,drive=stick' -nic none -display gtk)
if [[ $readonly_disk == off ]]; then
    echo "Booting $disk read-write. Guest writes go directly to the USB drive."
    if [[ -r $disk && -w $disk ]]; then exec "${qemu[@]}" 3<>"$disk"; fi
else
    echo "Booting $disk read-only. Close the QEMU window to exit."
fi
if [[ $readonly_disk == on && -r $disk ]]; then
    exec "${qemu[@]}" 3<"$disk"
fi

# Open only the disk as root, then drop privileges before starting the GUI.
gui_env=("HOME=$HOME" "PATH=$PATH")
for key in DISPLAY WAYLAND_DISPLAY XAUTHORITY XDG_RUNTIME_DIR DBUS_SESSION_BUS_ADDRESS; do
    if [[ -v $key ]]; then gui_env+=("$key=${!key}"); fi
done
exec sudo -- /bin/bash -c '
    if [[ $2 == on ]]; then exec 3<"$1"; else exec 3<>"$1"; fi || exit 1
    shift 2
    exec /usr/bin/setpriv --reuid="$SUDO_UID" --regid="$SUDO_GID" --init-groups \
        --no-new-privs /usr/bin/env -i "$@"
' bash "$disk" "$readonly_disk" "${gui_env[@]}" "${qemu[@]}"
