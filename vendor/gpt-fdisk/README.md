# GPT fdisk 1.0.4 for x64 EFI

`gdisk_x64.efi` is from the [upstream EFI release](https://sourceforge.net/projects/gptfdisk/files/gptfdisk/1.0.4/gdisk-binaries/).
The upstream GPT fdisk and UEFI port source archives and GPL license are included.
This older EFI build may not work with every firmware or storage device.

The upstream binary places its dummy PE relocation section at RVA zero,
overlapping the image headers. `fix-pe.py` moves that section to a separate
page for newer firmware loaders. The original signature is removed because
it no longer matches; the executable code and GNU-EFI relocations are
unchanged.

To reproduce the bundled binary, extract `gdisk_x64.efi` from the upstream
archive and run `python3 fix-pe.py /path/to/original.efi gdisk_x64.efi`.
