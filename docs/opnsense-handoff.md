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
loads in the DVD kernel. Verify that the module loads before distributing it.

## Use and test the bundled handoff

New USBs created by the desktop installer include the disk identity and
zero-filled backing area needed for OPNsense. Recreate older USBs with
the current installer if you need OPNsense support. **Recreating a USB
erases it.** Copy the decompressed 26.7 DVD ISO into `iso/`, or let the
installer download, verify, and decompress it. The handoff currently
requires a USB drive with 512-byte logical sectors.

Place the unmodified DVD ISO on its own in a test directory and run:

```sh
ISO_DIR=/path/to/test-isos QEMU_CACHE_DIR=build/qemu-cache-geom \
  tools/test-qemu.sh
```

The test helper stamps an identity on its cached virtual USB disk.
The [pinned GEOM module](../vendor/opnsense-geom/README.md) is embedded
in `BOOTX64.EFI`, so no separate module download or build is needed.
The picker collects physical
blocks from a clean ext4 filesystem, merges adjacent ranges, and embeds
the bounded map in the module. Sparse blocks map to a verified zero-filled
area in the virtual disk's partition gap. The picker exposes the module
through the virtual CD without modifying the original ISO.

In QEMU, the pinned Ventoy revision's prebuilt FreeBSD 15.x module
mounted the OPNsense 26.7 DVD root and reached interface assignment
without a separate CD/DVD or a RAM copy of the ISO. A full installation
and physical USB boot have not been tested.

The acceptance test is an unmodified OPNsense 26.7 DVD ISO copied into
`iso/` on a QEMU USB disk, with no separately attached optical media.
The live environment must reach its login prompt and read the media from
the disk-backed provider. A loader menu or an EFI-only boot is not a pass.
