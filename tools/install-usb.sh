#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

die() { printf 'Error: %s\n' "$*" >&2; exit 1; }
disk= project=
isos_to_copy=()
while (($#)); do
    case "$1" in
        --disk) disk=${2-}; shift 2 ;;
        --iso) isos_to_copy+=("${2-}"); shift 2 ;;
        --project) project=${2-}; shift 2 ;;
        *) die "unknown argument: $1" ;;
    esac
done
[[ $disk == /dev/* && -b $disk ]] || die "invalid disk"
for iso in "${isos_to_copy[@]}"; do
    [[ -f $iso && $iso == "$project"/isos/* ]] || die "ISO must be inside project isos/"
done
[[ -d $project && -x $(command -v lsblk) ]] || die "invalid project"
read -r type transport readonly < <(lsblk -dnro TYPE,TRAN,RO -- "$disk")
[[ $type == disk && $transport == usb && $readonly == 0 ]] || die "not a writable USB disk"
[[ -z $(lsblk -nrpo MOUNTPOINTS -- "$disk" | tr -d '[:space:]') ]] || die "unmount USB partitions first"

efi="$project/build/BOOTX64.EFI"
[[ -s $efi ]] || efi="$project/build/x86_64-unknown-uefi/release/efi-sk.efi"
[[ -s $efi ]] || die "build the UEFI picker first"
for file in memtest86plus/memtest86plus.efi edk2-shell/shell.efi keytool/KeyTool.efi \
    ipxe/ipxe.efi acpidump/acpidump.efi; do
    [[ -s $project/vendor/$file ]] || die "missing vendor/$file"
done

work=$(mktemp -d)
cleanup() { umount "$work/efi" "$work/isos" 2>/dev/null || true; rm -rf "$work"; }
trap cleanup EXIT
mkdir "$work/efi" "$work/isos"
sfdisk --wipe always --wipe-partitions always "$disk" <<'PARTITIONS'
label: gpt
start=1MiB, size=512MiB, type=U
type=L
PARTITIONS
udevadm settle
mapfile -t parts < <(lsblk -nrpo PATH,PARTN,TYPE -- "$disk" | awk '$3=="part"{print $1 " " $2}')
esp=$(printf '%s\n' "${parts[@]}" | awk '$2==1{print $1}')
isos=$(printf '%s\n' "${parts[@]}" | awk '$2==2{print $1}')
[[ -b $esp && -b $isos ]] || die "partitions did not appear"
mkfs.fat -F 32 -n PICKEREFI "$esp"
mkfs.ext4 -q -F -m 0 -L ISOS "$isos"
mount -o nosuid,nodev,noexec "$esp" "$work/efi"
mount -o nosuid,nodev,noexec "$isos" "$work/isos"
mkdir -p "$work/efi/EFI/BOOT" "$work/isos/iso"
cp "$efi" "$work/efi/EFI/BOOT/BOOTX64.EFI"
cp "$project/vendor/memtest86plus/memtest86plus.efi" "$work/efi/EFI/BOOT/"
cp "$project/vendor/edk2-shell/shell.efi" "$work/efi/EFI/BOOT/"
cp "$project/vendor/keytool/KeyTool.efi" "$work/efi/EFI/BOOT/"
cp "$project/vendor/ipxe/ipxe.efi" "$work/efi/EFI/BOOT/"
cp "$project/vendor/acpidump/acpidump.efi" "$work/efi/EFI/BOOT/"
for iso in "${isos_to_copy[@]}"; do
    cp "$iso" "$work/isos/iso/"
done
sync
printf 'USB prepared successfully\n'
