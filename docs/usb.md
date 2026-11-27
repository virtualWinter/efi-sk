# Set up the USB

## Interactive setup

Install the partitioning and filesystem tools:

- Debian/Ubuntu: `sudo apt install fdisk util-linux dosfstools e2fsprogs`
- Fedora: `sudo dnf install util-linux dosfstools e2fsprogs`
- Arch: `sudo pacman -S util-linux dosfstools e2fsprogs`

The script needs `lsblk`, `sfdisk`, `mkfs.fat`, `mkfs.ext4`, `mount`,
`umount`, `mountpoint`, `realpath` and `stat` on the PATH.

Then run the interactive script from the project directory:

```sh
sudo bash tools/create-usb.sh
```

The script creates both partitions, installs the built EFI binary and
the bundled EFI tools,
and leaves an empty `iso/` directory
on the `ISOS` partition. Copy your ISO files there afterward. The picker
detects supported images automatically. Unmount the USB first and back up
its contents. The script asks for an exact confirmation before erasing
the selected disk.

## Manual setup

Create a GPT partition table on the USB with the following two partitions.
Creating a new partition table erases the USB's current layout, so back up
its contents first.

| Partition | Format | Size | Contents |
| --- | --- | --- | --- |
| EFI System Partition | FAT32, GPT type EFI System Partition | 512 MiB | Picker and bundled EFI tools |
| ISO data | ext4, filesystem label `ISOS` | Remaining space | ISO files in the `iso/` directory |

### Use Linux `fdisk` for manual setup

Use a Linux system or a live ISO such as Arch. Back up the USB and
unmount its partitions first. Identify the USB by model, size, and
transport with `lsblk -o NAME,SIZE,MODEL,TRAN,MOUNTPOINTS`. The commands
below use `/dev/sdX` as a placeholder. Replace it with the verified USB
device, not your system disk.

1. Run `sudo fdisk /dev/sdX`. Use `g` for a new GPT, then `n` twice:
   accept the default first sector each time, enter `+512M` for the first
   partition's last sector, and accept the default last sector for the
   second partition. Use `t` to set partition 1 to **EFI System** (`L`
   lists the types). Use `p` to review, then `w` to write or `q` to quit
   without saving.
2. Verify the newly created partition paths with `lsblk`, then format
   them: `sudo mkfs.fat -F 32 /dev/sdX1` and
   `sudo mkfs.ext4 -L ISOS /dev/sdX2`.

`fdisk` and GPT fdisk only edit the partition table. Create the
filesystems with the `mkfs` commands above.

### Copy the picker and tools

Copy `build/BOOTX64.EFI` to `EFI/BOOT/BOOTX64.EFI` on the FAT32 partition.
The picker has a built-in tool list and shows the tools installed at
the paths below. Use UEFI Shell to launch other EFI applications.

For the optional **Memory Test (Memtest86+)** entry, also copy
`vendor/memtest86plus/memtest86plus.efi` to `EFI/BOOT/memtest86plus.efi`.
The picker adds it automatically if there is a free entry slot. It runs
the stock Memtest86+ interface. With Secure Boot enabled, the firmware
must also trust the [Memtest86+ signing CA](https://github.com/memtest86plus/memtest86plus/tree/v9.00/secureboot).

The USB script also installs the **UEFI Shell**. For manual setup, copy
`vendor/edk2-shell/shell.efi` to `EFI/BOOT/shell.efi`. It appears under
**Tools** when there is a free entry slot. Type `exit` to return to the
picker. With Secure Boot enabled, the firmware must trust the shell too.

**System Info** is built in and read-only. **Secure Boot Keys (KeyTool)**
is also installed by the USB script. For manual setup, copy
`vendor/keytool/KeyTool.efi` to `EFI/BOOT/KeyTool.efi`. KeyTool can change
firmware trust databases on supported systems. Back up existing keys
and understand the consequences before changing anything.
The firmware must expose the Secure Boot `SetupMode` variable, even if
Secure Boot is off. Stock non-Secure-Boot OVMF does not expose it.

**UEFI SCT** is installed by default. Install
`bsdtar` (`libarchive-tools` on Debian/Ubuntu, `bsdtar` on Fedora, or
`libarchive` on Arch). Set `INCLUDE_UEFI_SCT=0` to skip it in either
script. For manual setup, extract
`vendor/uefi-sct/sct-x64.7z` to the root of the FAT32 partition.
**UEFI SCT Installer** runs through the bundled UEFI Shell, which provides
the commands it needs to install the harness. It does not start the tests.
Use a disposable VM for conformance testing, since tests can change
firmware state or reboot the machine.

Run `acpiview -l` in **UEFI Shell** to list the firmware's ACPI tables.
**System Info** shows the current display resolution.

**Partition Editor (GPT fdisk)** is installed by default. It can rewrite
partition tables. Set `INCLUDE_GPT_FDISK=0` to skip it. For manual setup,
copy `vendor/gpt-fdisk/gdisk_x64.efi` to `EFI/BOOT/gdisk_x64.efi`.
The upstream EFI build is older. Use a disposable disk to test it.

A 64 GB USB stick is plenty for these tools. The USB script uses a
512 MiB EFI partition and leaves the rest for ISOs.

**Network Boot (iPXE)** is installed by default. It needs a supported
network adapter and a boot server or URL. For manual setup, copy
`vendor/ipxe/ipxe.efi` to `EFI/BOOT/ipxe.efi`.

Press Ctrl+B in iPXE for its command prompt. `iwlist` scans on supported
wireless adapters. Otherwise use Ethernet or boot Linux for Wi-Fi.

**Save ACPI Report (AcpiDump)** writes `acpidump.txt` to the root of
the picker's EFI partition. For manual setup, copy
`vendor/acpidump/acpidump.efi` to `EFI/BOOT/acpidump.efi`.

Copy [supported ISO files](profiles.md) directly into `iso/` on `ISOS`.
At boot the picker checks each image's kernel and initramfs paths and
lists recognized distros. The graphical menu shows the ISO filename
in gray parentheses. It scans the top level of `iso/`.
Unreadable and unrecognized images are skipped.

## Automatic entries and artwork

The menu reads the image's name from its media metadata or ISO volume
label and uses that name to pick a [bundled logo or
fallback](../assets/distro/README.md). Images without usable name metadata
appear as **Unknown ISO**. Renaming a file does not affect its menu
identity. All menu artwork comes from the picker.

## Menu input

No configuration file is read. The picker lists up to nine supported
ISOs from the top level of `iso/`, separately from the tools.
ISO filenames can contain ASCII letters, digits, dots, underscores, and
hyphens. The ISO itself can exceed FAT32's per-file limit.

Safely eject the drive and select its UEFI entry in the target PC's boot
menu. Tab cycles through Boot images, Tools, and the bottom action bar.
Use Up/Down in a list, Left/Right in the action bar, and Enter to activate
the selection. Number shortcuts launch boot images directly. Esc exits
the loader. In the graphical menu,
move the mouse to highlight an entry and left-click to boot it. Mouse
input depends on the firmware exposing a UEFI pointer device.

**Boot local drive** appears when the picker finds an enabled firmware
boot option on a non-USB EFI filesystem other than the picker's own.
It selects that option for the next boot and reboots. **Firmware setup**
appears only if the firmware advertises that capability. **Exit to firmware**
does the same thing as Esc. **Reboot** and **Shut down** restart or power
off the machine. The graphical action buttons are icon-only. Focus a button
to see its name.

The prebuilt application is unsigned. For Secure Boot, the firmware must
trust the loader and each EFI program it launches.
