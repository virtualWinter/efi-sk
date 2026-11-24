# ISO Boot Picker Installer (experimental)

This is the desktop Qt application for preparing an ISO Boot Picker USB.
It is intentionally a separate crate because the firmware picker is a
`no_std` UEFI application.

## Dependencies

- Rust
- Qt 6 development packages
- KDE Kirigami 6

On Debian/Ubuntu, install `qt6-base-dev`, `qt6-declarative-dev`, and the
distribution's Kirigami 6 QML package (often named `kirigami2-dev` or
`qml6-module-org-kde-kirigami`).

On Linux, downloads use `curl` and `sha256sum`. USB creation requires
`pkexec`, util-linux, udev, `mkfs.fat`, and `mkfs.ext4`.

To use the latest published picker and EFI tools, run:

```sh
ISO_BOOT_PICKER_GITHUB_REPO=OWNER/REPO tools/run-installer.sh
```

Replace `OWNER/REPO` with the GitHub repository that published the
[release](../docs/releases.md). Downloads are verified against the release
checksums before requesting administrator privileges. The repository can
also be set at compile time using the same variable. If it is unset, the
installer uses local tools and requires a [local picker build](../docs/build.md).
The **Optional EFI tools** checklist defaults to all tools. Deselect any you
don't want, or use **Deselect all tools** to install only the picker and your
images. Release mode downloads only the picker and selected tools; local mode
copies those files from the local build and `vendor/`. Selecting UEFI SCT also
includes the EFI Shell and requires `bsdtar`.
Release-mode ISO downloads are stored under
`${XDG_DATA_HOME:-$HOME/.local/share}/iso-boot-picker/isos`. Set
`ISO_BOOT_PICKER_ISO_DIR` to override that location.

The installer binary downloads and verifies files as your regular user. It
restarts itself through `pkexec` in a non-GUI writer mode to prepare the USB.
Neither path invokes scripts under `tools/`.

## Run

```sh
tools/run-installer.sh
```

Run the GUI as your regular user. Creating a USB **erases the selected drive**
and requests elevated privileges through `pkexec`. The download catalog is
experimental: an image being listed does not guarantee that the picker
supports booting it. For the documented manual setup, see
[Setting up the USB drive](../docs/usb.md).

To lint the QML with the Qt declarative tools installed:

```sh
tools/lint-installer.sh
```

The interface uses KDE Kirigami controls and layout conventions so it
integrates with KDE Plasma themes, high-DPI settings, keyboard navigation,
and accessibility tooling.
