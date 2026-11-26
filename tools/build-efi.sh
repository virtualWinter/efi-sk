#!/usr/bin/env bash
set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
target=x86_64-unknown-uefi
cargo build --locked --release --target "$target" --target-dir build "$@"
cp -- "build/$target/release/efi-sk.efi" build/BOOTX64.EFI
printf 'Built build/BOOTX64.EFI\n'
