#!/usr/bin/env bash
set -euo pipefail

usage() { echo "Usage: $0 /dev/USB_DISK"; }
if [[ ${1:-} == --help ]]; then usage; exit 0; fi
if (( $# != 1 )); then usage >&2; exit 2; fi
disk=$(realpath -e -- "$1")
if [[ ! -b $disk || $(lsblk -dnro TYPE,TRAN -- "$disk" | xargs) != 'disk usb' ]]; then
    echo "$disk is not a whole USB disk" >&2
    exit 1
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
      -blockdev 'driver=host_device,node-name=stick,filename=/dev/fdset/1,read-only=on'
      -device qemu-xhci -device 'usb-storage,drive=stick' -nic none -display gtk)
echo "Booting $disk read-only. Close the QEMU window to exit."
if [[ -r $disk ]]; then
    exec "${qemu[@]}" 3<"$disk"
fi

# Open only the disk as root, then drop privileges before starting the GUI.
gui_env=("HOME=$HOME" "PATH=$PATH")
for key in DISPLAY WAYLAND_DISPLAY XAUTHORITY XDG_RUNTIME_DIR DBUS_SESSION_BUS_ADDRESS; do
    if [[ -v $key ]]; then gui_env+=("$key=${!key}"); fi
done
exec sudo -- /bin/bash -c '
    exec 3<"$1" || exit 1
    shift
    exec /usr/bin/setpriv --reuid="$SUDO_UID" --regid="$SUDO_GID" --init-groups \
        --no-new-privs /usr/bin/env -i "$@"
' bash "$disk" "${gui_env[@]}" "${qemu[@]}"
