# OPNsense disk-backed handoff

The 26.7 amd64 DVD boots its EFI loader, but FreeBSD loses the
firmware-provided CD at `ExitBootServices`. Its installer expects to mount
`cd9660:/dev/iso9660/OPNSENSE_INSTALL`. The RAM-disk prototype failed on
QEMU because the loader could not allocate a large enough staging area.

## Reference module build

The DVD's `/boot/kernel/kernel` reports FreeBSD 15.1-RELEASE-p1,
`stable/26.7-n283674-12334a596709`. On an amd64 FreeBSD 15.x build host:

```sh
git clone https://github.com/opnsense/src.git opnsense-src
git -C opnsense-src checkout 12334a596709d195acb73f0f069da82c570a3d84
bash tools/build-opnsense-geom.sh opnsense-src build/opnsense-geom
```

This recipe builds Ventoy's BSD-licensed GEOM reference module against
the pinned OPNsense source configuration. The source files retain their
license notices. A successful build is not evidence that the module
loads in the DVD kernel. Do not ship the output until that is verified.

## Remaining integration and acceptance test

The picker must obtain the ISO's physical extents without modifying the
source filesystem, translate them from partition-relative offsets to the
USB disk, and pass a bounded, read-only map identifying the correct disk
to the preloaded GEOM module. The module must publish the ISO provider
before FreeBSD tries to mount root. A stock module also requires checking
its disk-identification assumptions against the EFI Swiss Knife layout.

The acceptance test is an unmodified OPNsense 26.7 DVD ISO copied into
`iso/` on a QEMU USB disk, with no separately attached optical media.
The live environment must reach its login prompt and read the media from
the disk-backed provider. A loader menu or an EFI-only boot is not a pass.
