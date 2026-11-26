# Third-party notices

The project's ISC license does not replace the terms for bundled third-party
files or external dependencies.

- [ext4-view](../vendor/ext4-view/README.md): MIT or Apache-2.0, with a
  local read-only physical block iterator.
- [GEOM module for OPNsense](../vendor/opnsense-geom/README.md): BSD
  license, embedded in the firmware.

- [Geist glyph atlases](../assets/OFL-Geist.txt): SIL Open Font License.
  The original TTFs are not bundled. See the [build guide](build.md#font)
  for the upstream release and regeneration instructions.
- [Lucide icons and generated bitmaps](../assets/lucide/README.md): see the
  [bundled license](../assets/lucide/LICENSE).
- [Installer and firmware distro icons](../installer/icons/README.md): Simple Icons,
  with [CC0 license](../installer/icons/LICENSE.simple-icons) and applicable trademark policies.
- [Additional firmware distro artwork](../assets/distro/README.md):
  Simple Icons and font-logos, with sources and licenses documented there.

The bundled EFI tools have their own notices and upstream source information:
[ACPIdump](../vendor/acpidump/README.md),
[EDK II Shell](../vendor/edk2-shell/README.md),
[GPT fdisk](../vendor/gpt-fdisk/README.md),
[iPXE](../vendor/ipxe/README.md),
[KeyTool](../vendor/keytool/README.md),
[Memtest86+](../vendor/memtest86plus/README.md), and
[UEFI SCT](../vendor/uefi-sct/README.md).
Keep those notices and any required corresponding source when redistributing
the tools or USB images that contain them.

Linux ISOs are downloaded separately, not distributed by this repository.
Distro names and logos remain subject to their respective trademark policies.
