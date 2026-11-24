# OPNsense GEOM module

`geom_ventoy.ko` is the unmodified, decompressed FreeBSD 15.x x86-64
[reference binary](https://github.com/ventoy/Ventoy/blob/26980f9d59192657b66c57d85157f44a08b24cc6/Unix/ventoy_unix/FreeBSD/geom_ventoy_ko/15.x/64/geom_ventoy.ko.xz)
from Ventoy commit `26980f9d59192657b66c57d85157f44a08b24cc6`. The matching
BSD-licensed GEOM source and [license](LICENSE) are included.

Decompressed SHA-256:
`aa8b2721b6adacd7d083821de69394debe3c9f50b2cf94144b658b3c30ccbb3b`.
The firmware enables it for the OPNsense 26.7 DVD kernel revision tested
in QEMU and fills its disk map in memory before FreeBSD loads it.
