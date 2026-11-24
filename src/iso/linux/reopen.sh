#!/bin/sh
# Runs before the original init. The source stays mounted for the installer.
export PATH=/sbin:/bin:/usr/sbin:/usr/bin
exec 3>&1
exec >/efi-sk-reopen.log 2>&1

mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev
IFS= read -r uuid < /efi-sk-source-uuid
IFS= read -r image < /efi-sk-source-path

if [ -f /efi-sk-ext4.udeb ]; then
    for package in /efi-sk-ext4.udeb /efi-sk-loop.udeb; do
        if ! ar p "$package" data.tar.xz | unxz | tar -xf - -C /; then
            echo "[EFI SK] Could not unpack storage drivers from $package"
        fi
    done
    depmod -a
    modprobe -q isofs
    modprobe -q loop
fi

# Load the storage drivers advertised by the machine, then USB mass storage.
for node in /sys/bus/pci/devices/*/modalias; do
    [ -r "$node" ] || continue
    read -r alias < "$node"
    modprobe -q "$alias"
done
for module in sd_mod usb-storage uas ext4; do
    modprobe -q "$module"
done

mkdir -p /efi-sk-source
mounted=no
attempt=0
while [ "$attempt" -lt 15 ]; do
    for node in /sys/class/block/*; do
        [ -f "$node/partition" ] || continue
        dev="/dev/${node##*/}"
        [ -b "$dev" ] || continue
        blkid "$dev" | grep -Fq " UUID=\"$uuid\"" || continue
        if mount -t ext4 -o ro,noload,nosuid,nodev,noexec "$dev" /efi-sk-source; then
            if [ -f "/efi-sk-source$image" ]; then
                if [ -f /efi-sk-ext4.udeb ]; then
                    mkdir -p /cdrom
                    mount -t iso9660 -o loop,ro "/efi-sk-source$image" /cdrom && mounted=yes
                else
                    ln -s "/efi-sk-source$image" /proxmox.iso && mounted=yes
                fi
                if [ "$mounted" = yes ]; then
                    echo "[EFI SK] Reopened installation media from $dev" >&3
                    break 2
                fi
            fi
            umount /efi-sk-source
        fi
    done
    attempt=$((attempt + 1))
    sleep 1
done

[ "$mounted" = yes ] || {
    cat /efi-sk-reopen.log >&3
    echo "[EFI SK] Could not reopen installation media; trying native discovery" >&3
}
umount /dev
umount /sys
umount /proc
exec /init >&3 2>&3 3>&-
