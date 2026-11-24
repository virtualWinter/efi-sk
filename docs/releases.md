# Releases and ISO catalog

Push an `esk-*` tag to publish one GitHub release with:

- `efi-sk-efi-x86_64.tar.gz`: `BOOTX64.EFI` and the bundled EFI tools.
- `efi-sk-installer-linux-x86_64.tar.gz`: the desktop installer, without Qt.
- `SHA256SUMS`: SHA-256 digests for both archives.

The workflow builds from the tagged lockfiles. Manual runs build and
package without publishing. Use the installer from the combined release:
older installer binaries expect individually published EFI assets.

The tagged source tree includes the bundled tools' [license notices and
source information](third-party.md), including the corresponding source
archives kept in `vendor/`. Keep the notices and source available alongside
any redistribution of the release binaries.

The ISO catalog workflow runs weekly and can be dispatched manually on
the default branch. It reads `catalog/sources.json`, resolves each entry
against the distributor's HTTPS SHA-256 list, and uploads `catalog.json`
to the non-latest `iso-catalog` release. Its download path is
`releases/download/iso-catalog/catalog.json` under the GitHub repository.
If generation fails, the workflow does not upload a replacement.

To preview a refresh locally:

```sh
python3 tools/update-catalog.py
```

This is a starter catalog, separate from the installer’s built-in list.
It does not yet include official logo assets or verify upstream signatures.
The installer does not fetch this catalog yet.
