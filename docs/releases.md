# Releases and ISO catalog

Push a version tag such as `v0.1.1`, or publish a GitHub release for a
`v`-prefixed tag, to build and attach:

- `efi-sk-efi-x86_64.tar.gz`: `BOOTX64.EFI` and the bundled EFI tools.
- `efi-sk-installer-linux-x86_64.tar.gz`: the desktop installer, without Qt.
- `catalog.json` and `distros.json`: the catalog and distro artwork index.
- `SHA256SUMS`: SHA-256 digests for the archives and JSON files.

The workflow builds from the tagged lockfiles. Manual runs build and
package without publishing. Use the installer from the combined release:
older installer binaries expect individually published EFI assets.

The tagged source tree includes the bundled tools' [license notices and
source information](third-party.md), including the corresponding source
archives kept in `vendor/`. Keep the notices and source available alongside
any redistribution of the release binaries.

The ISO catalog workflow runs weekly and can be dispatched manually on
the default branch. It reads `catalog/sources.json`, resolves download
checksums from upstream HTTPS lists or pinned upstream digests, and uploads `catalog.json`
to the non-latest `iso-catalog` release. Its download path is
`releases/download/iso-catalog/catalog.json` under the GitHub repository.
If generation fails, the workflow does not upload a replacement.

To preview a refresh locally:

```sh
python3 tools/update-catalog.py
```

`catalog.json` has three separate lists:

- `distros`: all entries in the embedded artwork index, including
  filename-independent recognition aliases and whether the artwork is
  a project logo, a related project's mark, or generic.
- `images`: specific downloads with checksums, linked to `distros` by
  distro ID. `qemu-verified` means a live boot or installer startup was
  verified, not a full installation; `layout-supported` covers a known
  layout, and `untested` or `download-only` do not claim boot support.
- `downloads`: one entry for every artwork distro. `verified-images`
  entries link to checksum-backed records in `images`. `reference-only`
  entries link to a project, archive, or DistroWatch page, not a verified
  direct download. Some old or restricted projects may not offer a
  public image.

The same rolling release also publishes `distros.json`, containing
`schema_version` and the `distros` list from `catalog.json`.

The desktop installer still uses its built-in list. The published
catalog does not bundle image files or verify upstream signatures.
