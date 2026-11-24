#!/usr/bin/env bash
set -euo pipefail

project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
command -v cargo >/dev/null || {
    printf 'Error: Rust Cargo is not installed.\n' >&2
    exit 1
}

exec cargo run --manifest-path "$project_dir/installer/Cargo.toml" -- "$@"
