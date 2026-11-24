# Linux ISO profiles

- Arch loads the kernel and initramfs under `arch/boot/x86_64/` and
  identifies the ISO partition by its ext4 filesystem UUID.
- Ubuntu expects `casper/vmlinuz` and `casper/initrd` and passes the ISO
  path to Casper.
- Fedora netinst expects `images/pxeboot/vmlinuz`, `images/pxeboot/initrd.img`,
  and `images/install.img`. The picker passes the ISO path and ext4
  filesystem UUID to Anaconda for its installer runtime.
- Debian supports amd64 netinst ISOs with the matching Debian
  Installer `hd-media` kernel and initramfs. The installer scans the disk
  for Debian ISOs; if it offers a choice, select the intended ISO.

Each profile expects an x86-64 Linux kernel built with the EFI stub and
an initramfs that can reopen the ISO from ext4. The ISO reader uses
ISO9660 directory records and inline Rock Ridge names. ISO booting is
profile-based. Other Linux layouts and Windows installer ISOs require
additional support.

## Debian netinst

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

The graphical menu reads the plain Debian swirl from `pics/logo-50.jpg`
inside the ISO and makes its white background transparent. If that fails,
it tries `pics/openlogo-nd-50.png`, then uses a generic icon.

Ubuntu artwork comes from the live system itself: the picker lists
`casper/`, opens the largest SquashFS first and reads the Circle of
Friends from `usr/share/pixmaps/`. If none of the known artwork files
decode, the entry uses a generic icon.

Fedora artwork comes from Anaconda's stage2 image: the picker opens
`images/install.img` and reads `fedora-logo-icon.png` from the hicolor
icon theme at `usr/share/icons/hicolor/`.
