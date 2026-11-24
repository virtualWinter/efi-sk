# Releases and ISO catalog

Push a version tag such as `v0.1.0` to publish `BOOTX64.EFI`, the bundled
EFI tools, and `SHA256SUMS` on GitHub Releases. The workflow builds from
that tag's lockfile. It does not package the desktop app.

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
