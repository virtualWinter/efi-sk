#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

die() { printf 'Error: %s\n' "$*" >&2; exit 1; }
shopt -s nullglob nocaseglob
isos=()
for iso in "${ISO_DIR:-isos}"/*.iso; do
    [[ -f "$iso" ]] && isos+=("$iso")
done

for command in cargo qemu-system-x86_64 sfdisk mkfs.ext4 mkfs.fat mmd mcopy flock; do
    command -v "$command" >/dev/null || die "Missing command: $command"
done

if [[ "${INCLUDE_UEFI_SCT:-1}" == 1 ]]; then
    command -v bsdtar >/dev/null || die 'Install bsdtar for UEFI SCT, or set INCLUDE_UEFI_SCT=0.'
fi

display=(-display gtk)
for arg in "$@"; do
    case "$arg" in
        -display|-display=*|-nographic|-vnc) display=(); break ;;
    esac
done
if (( ${#display[@]} )); then
    backends=$(qemu-system-x86_64 -display help)
    grep -qx gtk <<< "$backends" ||
        die 'QEMU GTK backend is missing. On Arch, install qemu-ui-gtk, or pass -display none for headless testing.'
fi

firmware=${OVMF_FIRMWARE:-}
if [[ -z "$firmware" ]]; then
    for candidate in /usr/share/{edk2,ovmf,OVMF}/{,x64/,ovmf/}OVMF{,_CODE}{,.4m,_4M}.fd; do
        if [[ -r "$candidate" ]]; then
            firmware=$candidate
            break
        fi
    done
fi
[[ -r "$firmware" ]] || die 'OVMF not found; set OVMF_FIRMWARE to a non-Secure-Boot firmware file.'

cache=${QEMU_CACHE_DIR:-build/qemu-cache}
mkdir -p "$cache"
exec 9>"$cache/lock"
flock -nx 9 || die 'A test is using this cache. Close that VM or set QEMU_CACHE_DIR to run another.'
tools/build-efi.sh
work=$(mktemp -d build/qemu.XXXXXXXX)
trap 'rm -rf -- "$work"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
files=()
bytes=0
for iso in "${isos[@]}"; do
    for file in "$iso" "$iso.vmlinuz" "$iso.initrd.gz"; do
        [[ -f "$file" ]] || continue
        files+=("$file")
        bytes=$((bytes + $(stat -Lc %s -- "$file")))
    done
done

# All partition offsets and lengths below are in 512-byte sectors.
esp_start=2048
esp_size=$((64 * 2048))
if [[ "${INCLUDE_UEFI_SCT:-1}" == 1 ]]; then
    # The installer requires at least 100 MiB free after staging its files.
    esp_size=$((160 * 2048))
fi
data_start=$((esp_start + esp_size))
data_size=$(((bytes + bytes / 10 + 128 * 1024 * 1024 + 1048575) / 1048576 * 2048))
{
    printf '%s\0' 'qemu-image-v1' "$esp_size" "${INCLUDE_GPT_FDISK:-1}" "${INCLUDE_UEFI_SCT:-1}"
    sha256sum tools/test-qemu.sh
    for file in "${files[@]}"; do
        printf '%s\0' "$file"
        stat -Lc '%d %i %s %y %z' -- "$file"
    done
    sha256sum vendor/{memtest86plus/memtest86plus,edk2-shell/shell,keytool/KeyTool,ipxe/ipxe,acpidump/acpidump,gpt-fdisk/gdisk_x64}.efi vendor/uefi-sct/sct-x64.7z
} > "$work/manifest"

build_image() {
    local image="$work/usb.img" esp
    mkdir -p "$work/data/iso"
    if (( ${#files[@]} )); then
        cp --dereference --reflink=auto -- "${files[@]}" "$work/data/iso/"
    fi
    truncate -s "$(((data_start + data_size + 2048) * 512))" "$image"
    sfdisk --quiet "$image" <<EOF
label: gpt
start=$esp_start, size=$esp_size, type=U
start=$data_start, size=$data_size, type=L
EOF

    mkfs.fat -F 32 -s 1 -h "$esp_start" -n PICKEREFI \
        --offset="$esp_start" "$image" "$((esp_size / 2))" >/dev/null
    esp="$image@@$((esp_start * 512))"
    mmd -i "$esp" ::/EFI ::/EFI/BOOT
    mcopy -i "$esp" vendor/memtest86plus/memtest86plus.efi ::/EFI/BOOT/memtest86plus.efi
    mcopy -i "$esp" vendor/edk2-shell/shell.efi ::/EFI/BOOT/shell.efi
    mcopy -i "$esp" vendor/keytool/KeyTool.efi ::/EFI/BOOT/KeyTool.efi
    mcopy -i "$esp" vendor/ipxe/ipxe.efi ::/EFI/BOOT/ipxe.efi
    mcopy -i "$esp" vendor/acpidump/acpidump.efi ::/EFI/BOOT/acpidump.efi
    if [[ "${INCLUDE_GPT_FDISK:-1}" == 1 ]]; then
        mcopy -i "$esp" vendor/gpt-fdisk/gdisk_x64.efi ::/EFI/BOOT/gdisk_x64.efi
    fi
    if [[ "${INCLUDE_UEFI_SCT:-1}" == 1 ]]; then
        bsdtar -xf vendor/uefi-sct/sct-x64.7z -C "$work"
        mcopy -s -i "$esp" "$work/SctPackageX64" ::/
    fi
    mkfs.ext4 -q -F -m 0 -b 4096 -L ISOS -d "$work/data" \
        -E "offset=$((data_start * 512))" "$image" "$((data_size / 8))"
}

image="$cache/usb.img"
if [[ ! -s $image ]] || ! cmp -s "$work/manifest" "$cache/manifest"; then
    printf 'Building QEMU USB image (cache miss).\n'
    build_image
    rm -f -- "$cache/manifest"
    mv -f -- "$work/usb.img" "$image"
    mv -f -- "$work/manifest" "$cache/manifest"
else
    printf 'Reusing cached QEMU USB image.\n'
fi
esp="$image@@$((esp_start * 512))"
mcopy -o -i "$esp" build/BOOTX64.EFI ::/EFI/BOOT/BOOTX64.EFI

printf 'Booting QEMU with %s ISO(s).\n' "${#isos[@]}"
qemu-system-x86_64 -machine q35,accel=kvm:tcg -m "${QEMU_MEMORY:-4G}" \
    -bios "$firmware" -snapshot \
    -drive "if=none,id=usb,format=raw,file=$image" \
    -device qemu-xhci -device usb-storage,drive=usb -device usb-mouse \
    "${display[@]}" "$@"
