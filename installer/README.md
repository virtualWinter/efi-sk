# EFI Swiss Knife Installer (experimental)

Use the Qt installer to prepare an EFI Swiss Knife USB drive on Linux. Run it as your regular user. **Creating a USB erases the selected drive.**

## Dependencies

- Rust
- Qt 6 development packages

On Debian/Ubuntu, install `qt6-base-dev` and `qt6-declarative-dev`.

On Linux, downloads use `curl` and `sha256sum`. USB creation requires
`pkexec`, util-linux, udev, `mkfs.fat`, and `mkfs.ext4`.
Downloading the OPNsense DVD also requires `bzip2`.

To use the latest published picker and EFI tools, run:

```sh
EFI_SK_GITHUB_REPO=OWNER/REPO tools/run-installer.sh
```

Replace `OWNER/REPO` with the GitHub repository that published the
[release](../docs/releases.md). Downloads are verified against the release
checksums before requesting administrator privileges. The repository can
also be set at compile time using the same variable. If it is unset, the
installer uses local tools and requires a [local picker build](../docs/build.md).

## Images and tools

Leave all boot images unchecked to create a drive without downloading any ISOs.
Otherwise, select the images you want and click **Create USB**. The installer
reuses local images and downloads and verifies missing ones before erasing the drive.

To skip checksum validation for image downloads in this session, launch
`efi-sk-installer --skip-iso-checksum` (or `tools/run-installer.sh --skip-iso-checksum`).
Unchecked downloads go in the ISO directory's `unchecked/` subfolder and are
not reused in normal mode. EFI release assets are still verified.
Copy ISO files into the `iso/` folder on the `ISOS` partition later.

The **Optional EFI tools** checklist defaults to all tools. Deselect any you
don't want, or use **Clear all** to install only the picker and your
images. Release mode downloads and verifies the EFI archive, then extracts
only the picker and selected tools using `tar`. Leave **EFI release version**
blank to use the latest release, or enter an exact tag such as `v0.1.1`
to use that version instead. Local mode copies those files
from the local build and `vendor/`. Selecting UEFI SCT also
includes the EFI Shell and requires `bsdtar`.
Release-mode ISO downloads are stored under
`${XDG_DATA_HOME:-$HOME/.local/share}/efiswissknife/isos`. Set
`EFI_SK_ISO_DIR` to override that location.

The installer binary downloads and verifies files as your regular user. It
restarts itself through `pkexec` in a non-GUI writer mode to prepare the USB.
Neither path invokes scripts under `tools/`.

## Run

On Ubuntu 24.04 x86-64 or a compatible Linux distribution, download the
installer archive and `SHA256SUMS` from a combined release such as `v0.1.1`. Verify it with
`sha256sum -c --ignore-missing SHA256SUMS` and extract it. Install the Qt 6 QML runtime
modules (on Ubuntu: `qml6-module-qtquick`,
`qml6-module-qtquick-controls`, `qml6-module-qtquick-layouts`,
`qml6-module-qtquick-templates`, `qml6-module-qtquick-window`,
and `qml6-module-qtqml-workerscript`).
Then run `./efi-sk-installer` from the extracted directory. The packaged
installer is configured to download EFI assets from the latest firmware release.

To run from a source checkout:

```sh
tools/run-installer.sh
```

The download catalog is
experimental. Proxmox VE 9.2 requires EFI Swiss Knife `v0.2` or newer.
OpenWrt is **download only**: it distributes a compressed EFI disk
image, not an ISO, and cannot be selected for USB installation. For other
entries, being listed does not guarantee that the picker
supports booting it. For the documented manual setup, see
[Setting up the USB drive](../docs/usb.md).

To lint the QML with the Qt declarative tools installed:

```sh
tools/lint-installer.sh
```
