#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

die() { printf 'Error: %s\n' "$*" >&2; exit 1; }
cancel() { printf '\nCancelled. No disk changes made.\n'; exit 0; }

(( EUID == 0 )) || die 'Run this script with sudo.'
for command in lsblk sfdisk mkfs.fat mkfs.ext4 mount umount mountpoint realpath stat; do
    command -v "$command" >/dev/null || die "Missing command: $command"
done

project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
efi="$project_dir/build/BOOTX64.EFI"
if [[ ! -s "$efi" ]]; then
    efi="$project_dir/build/x86_64-unknown-uefi/release/iso-boot-picker.efi"
fi
[[ -f "$efi" && -s "$efi" ]] || die 'Build ISO Boot Picker before creating the USB.'
memtest="$project_dir/vendor/memtest86plus/memtest86plus.efi"
[[ -s "$memtest" && -r "$memtest" ]] || die 'The bundled Memtest86+ executable is missing.'
shell="$project_dir/vendor/edk2-shell/shell.efi"
[[ -s "$shell" && -r "$shell" ]] || die 'The bundled UEFI Shell executable is missing.'
keytool="$project_dir/vendor/keytool/KeyTool.efi"
ipxe="$project_dir/vendor/ipxe/ipxe.efi"
acpidump="$project_dir/vendor/acpidump/acpidump.efi"
[[ -s "$acpidump" && -r "$acpidump" ]] || die 'The bundled AcpiDump executable is missing.'
[[ -s "$ipxe" && -r "$ipxe" ]] || die 'The bundled iPXE executable is missing.'
[[ -s "$keytool" && -r "$keytool" ]] || die 'The bundled KeyTool executable is missing.'
if [[ "${INCLUDE_UEFI_SCT:-1}" == 1 ]]; then
    command -v bsdtar >/dev/null || die 'Install bsdtar for UEFI SCT, or set INCLUDE_UEFI_SCT=0.'
    bsdtar -tf "$project_dir/vendor/uefi-sct/sct-x64.7z" >/dev/null || die 'Cannot read the UEFI SCT archive.'
fi
if [[ "${INCLUDE_GPT_FDISK:-1}" == 1 ]]; then
    [[ -s "$project_dir/vendor/gpt-fdisk/gdisk_x64.efi" ]] || die 'The bundled GPT fdisk executable is missing.'
fi

validate_disk() {
    [[ -b "$disk" && "$disk" == /dev/* ]] || die 'Select a block device.'
    local type transport readonly mounts
    read -r type transport readonly < <(lsblk -dnro TYPE,TRAN,RO -- "$disk") || die "Cannot inspect $disk."
    [[ "$type" == disk && "$transport" == usb && "$readonly" == 0 ]] ||
        die 'Select a writable whole USB disk.'
    mounts=$(lsblk -nr -o MOUNTPOINTS -- "$disk") || die "Cannot check mounts on $disk."
    [[ ! "$mounts" =~ [^[:space:]] ]] || die 'Unmount all partitions on this USB and disable any swap on it.'
    local topology
    topology=$(lsblk -nr -o TYPE -- "$disk") || die "Cannot check partitions on $disk."
    if grep -qvxE 'disk|part' <<< "$topology"; then
        die 'Deactivate device mappings on this USB first.'
    fi
}

printf 'Available USB disks\n'
lsblk -dnpr -o PATH,TYPE,TRAN,SIZE,RO,MODEL | awk '$2 == "disk" && $3 == "usb" && $5 == 0 { print "  " $0; found = 1 } END { exit !found }' ||
    die 'No writable USB disk found.'

read -erp 'USB device (for example /dev/sdb): ' device || cancel
[[ -n "$device" ]] || cancel
disk=$(realpath -e -- "$device") || die 'Device path does not exist.'
validate_disk
sequence_file="/sys/class/block/${disk##*/}/diskseq"
disk_sequence=$(cat -- "$sequence_file") || die 'Cannot identify the USB device.'
disk_bytes=$(lsblk -bdnro SIZE -- "$disk") || die 'Cannot determine USB capacity.'
[[ "$disk_bytes" =~ ^[0-9]+$ ]] || die 'Invalid USB capacity.'

(( disk_bytes >= 768 * 1024 * 1024 )) || die 'Use a USB drive with at least 768 MiB.'

printf '\nUSB to erase\n'
lsblk -p -o NAME,SIZE,MODEL,FSTYPE,LABEL,MOUNTPOINTS -- "$disk"
read -rp "Type ERASE $disk to erase this USB and continue: " confirmation || cancel
[[ "$confirmation" == "ERASE $disk" ]] || cancel

validate_disk
[[ $(cat -- "$sequence_file") == "$disk_sequence" ]] || die 'The selected USB changed.'

workdir=$(mktemp -d)
mkdir -- "$workdir/efi" "$workdir/isos"
cleanup() {
    local status=$?
    trap - EXIT
    for directory in "$workdir/isos" "$workdir/efi"; do
        if mountpoint -q -- "$directory"; then
            umount -- "$directory" || status=1
        fi
    done
    rmdir -- "$workdir/isos" "$workdir/efi" "$workdir" 2>/dev/null || true
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

printf 'Creating GPT partitions on %s...\n' "$disk"
sfdisk --wipe always --wipe-partitions always -- "$disk" <<'PARTITIONS'
label: gpt
start=1MiB, size=512MiB, type=U
type=L
PARTITIONS

esp=''
isos=''
for (( attempt=0; attempt<20; attempt++ )); do
    mapfile -t partitions < <(lsblk -nrpo PATH,PARTN,TYPE -- "$disk")
    for partition in "${partitions[@]}"; do
        read -r path number type <<< "$partition"
        case "$number $type" in
            '1 part') esp=$path ;;
            '2 part') isos=$path ;;
        esac
    done
    [[ -b "$esp" && -b "$isos" ]] && break
    sleep 0.5
done
[[ -b "$esp" && -b "$isos" ]] || die 'The new USB partitions did not appear.'
[[ $(cat -- "$sequence_file") == "$disk_sequence" ]] || die 'The selected USB changed.'

mkfs.fat -F 32 -n PICKEREFI -- "$esp"
mkfs.ext4 -q -F -m 0 -L ISOS -- "$isos"
mount -o nosuid,nodev,noexec -- "$esp" "$workdir/efi"
mount -o nosuid,nodev,noexec -- "$isos" "$workdir/isos"
mkdir -p -- "$workdir/efi/EFI/BOOT" "$workdir/isos/iso"
cp -- "$efi" "$workdir/efi/EFI/BOOT/BOOTX64.EFI"
cp -- "$memtest" "$workdir/efi/EFI/BOOT/memtest86plus.efi"
cp -- "$shell" "$workdir/efi/EFI/BOOT/shell.efi"
cp -- "$keytool" "$workdir/efi/EFI/BOOT/KeyTool.efi"
cp -- "$ipxe" "$workdir/efi/EFI/BOOT/ipxe.efi"
cp -- "$acpidump" "$workdir/efi/EFI/BOOT/acpidump.efi"
if [[ "${INCLUDE_GPT_FDISK:-1}" == 1 ]]; then
    cp -- "$project_dir/vendor/gpt-fdisk/gdisk_x64.efi" "$workdir/efi/EFI/BOOT/"
fi
if [[ "${INCLUDE_UEFI_SCT:-1}" == 1 ]]; then
    bsdtar -xf "$project_dir/vendor/uefi-sct/sct-x64.7z" -C "$workdir/efi"
fi
if [[ ${SUDO_UID:-} =~ ^[0-9]+$ && ${SUDO_GID:-} =~ ^[0-9]+$ ]]; then
    chown -- "$SUDO_UID:$SUDO_GID" "$workdir/isos/iso"
fi


umount -- "$workdir/isos"
umount -- "$workdir/efi"
printf 'USB prepared. Copy your ISOs into iso/ on the ISOS partition.\n'
