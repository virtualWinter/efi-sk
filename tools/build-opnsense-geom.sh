#!/usr/bin/env bash
# Build a reference GEOM module for the OPNsense 26.7 kernel ABI.
set -euo pipefail

die() { printf 'Error: %s\n' "$*" >&2; exit 1; }
[[ $(uname -s) == FreeBSD && $(uname -m) == amd64 ]] ||
    die 'Run this on an amd64 FreeBSD 15.x build host.'
[[ $# == 2 ]] || die "Usage: $0 /path/to/opnsense-src /path/to/output-directory"
src=$(cd -- "$1" && pwd -P)
mkdir -p -- "$2"
out=$(cd -- "$2" && pwd -P)

kernel_rev=12334a596709d195acb73f0f069da82c570a3d84
ventoy_rev=26980f9d59192657b66c57d85157f44a08b24cc6
[[ $(git -C "$src" rev-parse HEAD) == "$kernel_rev" ]] ||
    die "Check out OPNsense src revision $kernel_rev first."
[[ -z $(git -C "$src" status --porcelain) ]] ||
    die 'The OPNsense source checkout must be clean.'

work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
base="https://raw.githubusercontent.com/ventoy/Ventoy/$ventoy_rev/Unix/ventoy_unix_src/FreeBSD/geom_ventoy_src/15.x/sys/geom/ventoy"
for file in g_ventoy.c g_ventoy.h; do
    fetch -q -o "$work/$file" "$base/$file"
done
[[ $(sha256 -q "$work/g_ventoy.c") == 5104c8efe3a3bc7414972187eef7ca25d19e2e332238dca9e11ee57fdca44f61 &&
   $(sha256 -q "$work/g_ventoy.h") == 4b664ec2ad191acbdb348a4bb703c2bc7fc3710a487f1039d464a65ec1003862 ]] ||
    die 'The reference module source did not match its pinned checksums.'
cat > "$work/Makefile" <<'EOF'
KMOD=geom_ventoy
SRCS=g_ventoy.c
.include <bsd.kmod.mk>
EOF

# Use the target source's GENERIC configuration, not the host kernel headers.
(cd "$src/sys/amd64/conf" && config -d "$work/kern" GENERIC)
make -C "$work" SYSDIR="$src/sys" KERNBUILDDIR="$work/kern"
install -m 0644 "$work/geom_ventoy.ko" "$out/geom_ventoy.ko"
install -m 0644 "$work/g_ventoy.c" "$work/g_ventoy.h" "$out/"
printf 'Built %s/geom_ventoy.ko; boot compatibility has not been verified.\n' "$out"
