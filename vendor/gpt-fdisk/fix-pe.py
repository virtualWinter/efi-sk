"""Move the upstream EFI binary's dummy relocation section out of its PE headers."""

import hashlib
import struct
import sys
from pathlib import Path

data = bytearray(Path(sys.argv[1]).read_bytes())
if hashlib.sha256(data).hexdigest() != (
    "28c5deb0b7f6675d3b32f9339002028aa7cfc8ae2719241da32c7bd201e7f274"
):
    raise SystemExit("Expected the upstream x64 GPT fdisk 1.0.4 EFI binary")

pe = struct.unpack_from("<I", data, 60)[0]
optional = pe + 24
sections = optional + 240
directories = optional + 112
certificate, _ = struct.unpack_from("<II", data, directories + 4 * 8)
image_size = struct.unpack_from("<I", data, optional + 56)[0]

# The original dummy .reloc block starts at RVA 0, overlapping the PE headers.
# GNU-EFI still performs the real ELF relocations using the untouched .rela.
reloc = bytearray(data[sections : sections + 40])
struct.pack_into("<IIII", reloc, 8, 12, image_size, 512, certificate)
data[sections : sections + 280] = data[sections + 40 : sections + 280] + reloc
data[certificate:] = struct.pack("<IIHH", 0, 12, 0, 0).ljust(512, b"\0")
struct.pack_into("<II", data, directories + 4 * 8, 0, 0)  # Signature no longer valid.
struct.pack_into("<II", data, directories + 5 * 8, image_size, 12)
struct.pack_into("<II", data, optional + 56, image_size + 4096, 1024)
struct.pack_into("<I", data, optional + 64, 0)  # Clear the stale PE checksum.
Path(sys.argv[2]).write_bytes(data)
