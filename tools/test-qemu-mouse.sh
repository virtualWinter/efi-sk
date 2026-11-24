#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

if [[ -z "${OVMF_FIRMWARE:-}" ]]; then
    OVMF_FIRMWARE=$(bash tools/build-mouse-ovmf.sh)
fi
export OVMF_FIRMWARE
exec tools/test-qemu.sh "$@"
