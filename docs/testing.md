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

The script builds the picker and creates a temporary GPT USB image with
the new EFI binary and all top-level `.iso` files from `isos/`, including
any matching Debian installer companion files. The picker
lists [supported ISOs](profiles.md) in its boot menu. The script does not
need root or write to physical drives. It removes the test image when
QEMU exits.

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
