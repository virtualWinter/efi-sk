# Releases and ISO catalog

The firmware and desktop installer have separate tags and release workflows:

- `esk-0.1` (pattern `esk-*`) publishes `BOOTX64.EFI`, the bundled EFI
  tools, and `SHA256SUMS`. Only firmware releases are marked latest, so
  the desktop installer can find the firmware and tools.
- `installer-v0.1` (pattern `installer-v*`) publishes the Linux x86-64
  installer archive and `SHA256SUMS`. It does not bundle firmware or Qt.

Both workflows build from the tagged lockfiles. Manual runs build and
package without publishing.

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
