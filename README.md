# EFI Swiss Knife

Project name: `efiswissknife`. Build target executable: `efi-sk.efi`.

A Rust boot menu for x86-64 UEFI. It automatically discovers supported
ISO files in `iso/` on the `ISOS` ext4 partition and shows a built-in
list of tools installed on the EFI partition. No configuration file
is needed. Each EFI tool receives its full device path, so it
can find files relative to its own location. The menu uses the firmware's
Graphics Output Protocol when available, falling back to the firmware
text console. The graphical menu embeds Geist from Vercel's v1.9.2
release as pre-rendered glyphs under the SIL Open Font License in
`assets/OFL-Geist.txt`.

Ubuntu's [Circle of Friends](https://discourse.ubuntu.com/t/ubuntu-merchandise-assets/67314)
(registered by Canonical Ltd.) is read from the ISO's live system at boot,
and Arch Linux artwork is subject to the
[Arch Linux trademark policy](https://archlinux.org/art/).

Documentation:

- [Building the picker](docs/build.md)
- [Setting up the USB drive](docs/usb.md)
- [Linux ISO profiles](docs/profiles.md)
- [Testing with QEMU](docs/testing.md)
- [Desktop installer (experimental)](installer/README.md)

## License

Original project code is licensed under the [ISC License](LICENSE).
Bundled third-party software, fonts, and artwork remain under their
respective terms; see [third-party notices](docs/third-party.md).
