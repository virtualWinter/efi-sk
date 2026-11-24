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

## Test the opt-in disk-backed handoff

Place the unmodified DVD ISO on its own in a test directory and run:

```sh
ISO_DIR=/path/to/test-isos QEMU_CACHE_DIR=build/qemu-cache-geom \
  OPNSENSE_GEOM_MODULE="$PWD/build/opnsense-geom/geom_ventoy.ko" \
  tools/test-qemu.sh
```

To reproduce the QEMU test without a FreeBSD build host, the
[pinned reference module](https://github.com/ventoy/Ventoy/blob/26980f9d59192657b66c57d85157f44a08b24cc6/Unix/ventoy_unix/FreeBSD/geom_ventoy_ko/15.x/64/geom_ventoy.ko.xz)
has compressed SHA-256
`392ac5d2a3dab6d410ecc5a95608a559e4fefd396a3b96dfedc27c32a3e0bce5`.
Verify it, decompress it with `xz -d`, then point `OPNSENSE_GEOM_MODULE`
at the `.ko`. Only provide a module you trust to run in the guest kernel.

The helper stamps an identity on the **temporary virtual USB disk only**
and stages the module on its EFI partition. The picker collects physical
blocks from a clean ext4 filesystem, merges adjacent ranges, and embeds
the bounded map in the module. Sparse blocks map to a verified zero-filled
area in the virtual disk's partition gap. The picker exposes the module
through the virtual CD without modifying the original ISO. This is opt-in and is not
installed on physical USB drives by the desktop installer.

In QEMU, the pinned Ventoy revision's prebuilt FreeBSD 15.x module
mounted the OPNsense 26.7 DVD root and reached interface assignment
without a separate CD/DVD or a RAM copy of the ISO. The prebuilt module
is not bundled. A full installation and physical USB boot have not been
tested.

The acceptance test is an unmodified OPNsense 26.7 DVD ISO copied into
`iso/` on a QEMU USB disk, with no separately attached optical media.
The live environment must reach its login prompt and read the media from
the disk-backed provider. A loader menu or an EFI-only boot is not a pass.
