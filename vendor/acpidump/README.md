# ACPICA AcpiDump for x64 EFI

Built from [ACPICA 20260930](https://github.com/open-acpica/acpica/tree/20260930)
using GNU-EFI 4.0.4 (Arch package `4.0.4-1`). No ACPICA source changes.
From the ACPICA checkout, with GNU-EFI installed:

```sh
mkdir -p generate/efi/acpidump/obj
make -C generate/efi/acpidump OPT_CFLAGS=-O2
```

The result is `generate/efi/acpidump/obj/acpidump.efi`.
