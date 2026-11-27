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
or bundled tools change, it rebuilds the image. Otherwise it only
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

### Headless smoke test

`tools/smoke-test-qemu.sh` builds the picker with the `text-mode`
feature, boots it from a minimal EFI system partition with no display
and no ISO files, and fails unless the menu reaches the serial console.
It needs QEMU, OVMF, mtools, and the Rust toolchain, but neither root
nor network access, so it is the quickest check that a change still
boots. The graphics build only draws to GOP, which no headless run can
observe, so the assertion uses a text-mode build instead. The serial
log of the last run is kept in `build/qemu-smoke/serial.log`. Set
`SMOKE_TIMEOUT` to change the 120 second deadline.

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

## Booting an ISO or a disk image directly

`tools/boot-iso-qemu.sh ISO` boots an ISO under OVMF without the
picker, which is the quickest way to check that an installer itself
starts. Pass `--usb` to attach it as USB storage instead of a SATA
CD-ROM.

`tools/boot-image-qemu.sh IMAGE` boots a raw disk image, for example
`build/qemu-cache/usb.img` or the file written by the test below. The
image is attached as USB storage; pass `--disk` to attach it as a SATA
disk. Guest writes go to a temporary overlay that is discarded when
QEMU exits unless you pass `--read-write`.

Both open a GTK window. Pass `-display none`, `-nographic`, or `-vnc`
to run without one, in which case the serial console goes to the
terminal. Every other argument is forwarded to `qemu-system-x86_64`, so
`-nic none` disables the default user-mode networking.

## Installer end-to-end test

`tools/test-installer-qemu.sh` exercises the real USB writer without a
physical drive. It loads the `dummy_hcd` and `g_mass_storage` kernel
modules so that a file appears to `lsblk` as a writable whole USB disk,
runs the installer's `--write-usb` writer against it, unloads the
modules, and boots the written image to assert the picker's menu on the
serial console.

The two modules must be available for the running kernel, and the
script asks for a sudo password to load them and to run the writer. It
never touches a physical drive. Without arguments it uses the smallest
ISO in `isos/`, which must be one the picker lists
([supported ISOs](profiles.md)). The virtual disk is the size of the
ISOs plus 1 GiB, and the work directory is removed when the run ends.

A desktop environment that automounts new USB devices can make the
writer refuse the disk; unmount it or turn automounting off first. The
written disk carries a text-mode picker so the final assertion can read
the menu from serial.

Set `SKIP_BOOT=1` to stop after the writer finishes, `EXPECT_ISO=0` if
the picker is not expected to list the ISO, `EXCLUDE_TOOLS` to a
comma-separated list of `usb::TOOLS` indices to leave out (UEFI SCT is
index 6, and `bsdtar` is then not required), `KEEP_IMAGE=1` to keep the
work directory for inspection, and `BOOT_TIMEOUT` to change the 120
second deadline.

## Mouse input

The regular test uses the installed OVMF firmware. Stock OVMF
may lack a mouse driver. Keyboard input still works.

For an optional mouse test, run `tools/test-qemu-mouse.sh`. Its first run
downloads EDK II from GitHub, builds OVMF with `UsbMouseDxe`, and caches it
under `build/`. Install the extra build tools before running it:

- Debian/Ubuntu: `sudo apt install build-essential git nasm acpica-tools python3 uuid-dev`
- Fedora: `sudo dnf install gcc gcc-c++ make git nasm acpica-tools python3 libuuid-devel`
- Arch: `sudo pacman -S base-devel git nasm acpica python util-linux-libs`

Set `OVMF_FIRMWARE` to use an existing firmware file with either script
and skip the optional build.
