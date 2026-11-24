# Testing with QEMU

Install QEMU, OVMF, and the tools for creating a USB image:

- Debian/Ubuntu: `sudo apt install qemu-system-x86 ovmf fdisk dosfstools e2fsprogs mtools`
- Fedora: `sudo dnf install qemu-system-x86 edk2-ovmf util-linux dosfstools e2fsprogs mtools`
- Arch: `sudo pacman -S qemu-desktop edk2-ovmf util-linux dosfstools e2fsprogs mtools`

Set up the [Rust toolchain](build.md), then run from the project root:

```sh
mkdir -p isos
# Copy your ISO files into isos/, then:
tools/test-qemu.sh
```

The script builds the picker and prepares a cached GPT USB image with
the new EFI binary and all top-level `.iso` files from `isos/`, including
any matching Debian installer companion files. The picker
lists [supported ISOs](profiles.md) in its boot menu. The script does not
need root or write to physical drives. When the ISO files, companions,
or bundled tools change, it rebuilds the image; otherwise it only
updates `BOOTX64.EFI` on the cached EFI partition. QEMU runs with
`-snapshot`, so guest writes don't affect the cache. A second run using
the same cache exits with a message rather than waiting. Set
`QEMU_CACHE_DIR` to a different directory under `build/` to run tests
concurrently. Delete a cache directory when no test is using it to
reclaim its space.

The test image includes all bundled tools by default, even if `isos/` is
empty. UEFI SCT needs `bsdtar` (`libarchive-tools` on Debian/Ubuntu,
`bsdtar` on Fedora, or `libarchive` on Arch). Set `INCLUDE_UEFI_SCT=0`
or `INCLUDE_GPT_FDISK=0` to omit either tool.

If OVMF is installed in a custom location, set `OVMF_FIRMWARE` to the
path of a non-Secure-Boot firmware file.

The script opens a GTK window by default. If the GTK backend is missing,
install `qemu-ui-gtk` on Arch. To run headless, pass `-display none`.

Set `ISO_DIR` to test a different directory of ISOs, or `QEMU_MEMORY`
to change the default 4 GiB of guest RAM.

### Drop-in ISO smoke test

Set `ISO_DIR` to a directory containing just the checksum-verified,
unmodified ISO. Do not attach another copy as a virtual DVD or put
companion files beside it. Select the ISO in the picker and continue
past language and keyboard selection where applicable. Verify that the
installer reads its media before proceeding to disk partitioning:

- **Debian netinst:** advance past media detection and loading installer
  components, or use an installer shell to read a package from
  `/cdrom/pool/`.
- **Proxmox VE:** reach the installer UI after it mounts its base system.
  `[EFI SK] Reopened installation media` alone confirms the USB handoff,
  but not that the installer payload was readable.

## Mouse input

The regular test uses installed OVMF and never builds firmware. Stock OVMF
may lack a mouse driver; keyboard input still works.

For an optional mouse test, run `tools/test-qemu-mouse.sh`. Its first run
downloads EDK II from GitHub, builds OVMF with `UsbMouseDxe`, and caches it
under `build/`. Install the extra build tools before running it:

- Debian/Ubuntu: `sudo apt install build-essential git nasm acpica-tools python3 uuid-dev`
- Fedora: `sudo dnf install gcc gcc-c++ make git nasm acpica-tools python3 libuuid-devel`
- Arch: `sudo pacman -S base-devel git nasm acpica python util-linux-libs`

Set `OVMF_FIRMWARE` to use an existing firmware file with either script
and skip the optional build.
