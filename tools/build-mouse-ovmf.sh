#!/usr/bin/env bash
set -eo pipefail
export LC_ALL=C
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

release=edk2-stable202502
cache="$PWD/build"
firmware="$cache/OVMF-mouse-$release.fd"
if [[ -s "$firmware" ]]; then
    printf '%s\n' "$firmware"
    exit
fi

for command in git make gcc g++ nasm iasl python3; do
    command -v "$command" >/dev/null || {
        printf 'Missing %s for mouse-enabled OVMF. See docs/testing.md for build dependencies, or set OVMF_FIRMWARE.\n' "$command" >&2
        exit 1
    }
done

mkdir -p -- "$cache"
work=$(mktemp -d "$cache/ovmf.XXXXXXXX")
trap 'rm -rf -- "$work"' EXIT
(
    printf 'Building and caching mouse-enabled OVMF (first run only)...\n'
    git clone --depth 1 --branch "$release" --recurse-submodules --shallow-submodules \
        https://github.com/tianocore/edk2.git "$work/edk2"
    cd -- "$work/edk2"
    driver=MdeModulePkg/Bus/Usb/UsbMouseDxe/UsbMouseDxe.inf
    sed -i "/^\[Components\]/a\\  $driver" OvmfPkg/OvmfPkgX64.dsc
    sed -i "/^INF .*UsbKbDxe\.inf/a\\INF  $driver" OvmfPkg/OvmfPkgX64.fdf
    grep -Fq "$driver" OvmfPkg/OvmfPkgX64.dsc
    grep -Fq "$driver" OvmfPkg/OvmfPkgX64.fdf
    make -C BaseTools -j "$(nproc)"
    source edksetup.sh
    build -a X64 -t GCC5 -b RELEASE -p OvmfPkg/OvmfPkgX64.dsc
    cp Build/OvmfX64/RELEASE_GCC5/FV/OVMF.fd "$work/OVMF.fd"
    mv -- "$work/OVMF.fd" "$firmware"
) >&2
printf '%s\n' "$firmware"
