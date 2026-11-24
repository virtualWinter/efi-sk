# Linux ISO boot compatibility

- Arch loads the kernel and initramfs under `arch/boot/x86_64/` and
  identifies the ISO partition by its ext4 filesystem UUID.
- Ubuntu expects `casper/vmlinuz` and `casper/initrd` and passes the ISO
  path to Casper.
- Fedora netinst expects `images/pxeboot/vmlinuz`, `images/pxeboot/initrd.img`,
  and `images/install.img`. The picker passes the ISO path and ext4
  filesystem UUID to Anaconda for its installer runtime.
- Debian amd64 netinst can boot directly from the ISO when it includes
  matching ext4 and loop driver packages. Older layouts can still use the
  matching Debian Installer `hd-media` kernel and initramfs.
- Proxmox VE has an experimental drop-in profile as of `v0.2`. It loads the kernel and
  initramfs from the ISO and adds a small early-boot helper. The helper
  reopens the read-only USB partition by UUID and points `/proxmox.iso`
  to the original file, which the installer uses through its existing
  iPXE media path. It does not copy the ISO into memory.
  Proxmox VE 9.2-1 was verified in QEMU through the installer license screen.

Each Linux boot layout expects an x86-64 kernel built with the EFI stub and
an initramfs that can reopen the ISO from ext4. The ISO reader uses
ISO9660 directory records and inline Rock Ridge names. ISO booting is
profile-based. Other Linux layouts and Windows installer ISOs require
additional support.

## Experimental EFI-loader fallback

If no Linux profile matches, the picker checks for an EFI
boot entry in the ISO's El Torito catalog. It shows matching images as
**experimental EFI** entries. Selecting one exposes the ISO as a
read-only UEFI virtual CD and tries to start `\EFI\BOOT\BOOTX64.EFI` from
its El Torito boot filesystem. Reads are served from the USB on demand;
the firmware does not need to load the whole ISO into RAM. Existing Linux
profiles take precedence for now.

This path still requires firmware drivers for El Torito and FAT. It does
not preserve access to the virtual CD after the OS exits UEFI boot
services. Reaching an ISO's boot menu does not mean its installer can
find its media. Windows is not yet verified, and raw disk images such as
OpenWrt are not handled.

### Router images

- **VyOS rolling (experimental):** the picker can match the kernel and
  initramfs under `live/` by their shared version suffix. It passes the
  original ISO path to Debian live-boot and preserves the normal VyOS
  live boot options. The 2026.11.22 rolling ISO reached the live login
  prompt in QEMU with no companion files. Login and disk installation
  have not been verified.
- **OPNsense DVD (download only):** the installer catalog verifies the
  upstream `.iso.bz2`. The firmware can try its EFI loader on the
  decompressed ISO. For `OPNSENSE_INSTALL` media, an experimental
  read-only overlay asks the FreeBSD loader to preload the ISO as a
  memory disk. It leaves the USB file unchanged but requires enough
  RAM for the whole ISO plus the installer. Do not assume root mounting
  works until the installation environment has been verified.

### Replacing compatibility profiles

The intended next layer hands off the USB partition UUID and ISO path to
early userspace. Linux can reopen the USB partition and expose the ISO
through a read-only loop device. The loader must arrange for that helper
to run; merely launching an ISO's EFI executable cannot do this for
arbitrary installers. Where an image cannot reopen its source filesystem,
an extent-based block mapping can be evaluated separately.

Retire each existing profile only after its replacement boots an untouched
ISO from `iso/` without companion files and reaches a live system or an
installer that can read its packages. Keep firmware-only EFI launch
distinct from verified full boots.

## Debian netinst

Try copying just the ISO into `iso/` first. The picker can use the netinst
kernel and initramfs and load the storage drivers from the same ISO. For
images that lack those driver packages in the expected layout, use the
following companion-file method.

Debian 13.8 amd64 netinst was verified in QEMU through media detection
and successful network configuration with only the ISO on the USB.

Download `vmlinuz` and `initrd.gz` from Debian's
[amd64 hd-media directory for Debian 13 (trixie)](https://deb.debian.org/debian/dists/trixie/main/installer-amd64/current/images/hd-media/).
Use the pair built for your ISO's release and kernel ABI; if using an
older point release, obtain its matching installer files rather than
mixing releases.

Put all three files together. For example:

```text
debian-13.8.0-amd64-netinst.iso
debian-13.8.0-amd64-netinst.iso.vmlinuz
debian-13.8.0-amd64-netinst.iso.initrd.gz
```

Use `iso/` on the USB's `ISOS` partition, or `isos/` in the project for
the QEMU test. A top-level ISO is detected automatically when both
companion files are present.
