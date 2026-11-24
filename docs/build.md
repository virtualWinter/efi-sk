# Build

## Repository layout

This repository contains two independent Cargo packages, not a Cargo
workspace. They share a repository because the installer deploys the picker
and the bundled EFI tools together.

| Path | Purpose |
| --- | --- |
| `Cargo.toml`, `src/` | `no_std` picker for x86-64 UEFI |
| `installer/` | Linux desktop installer using Qt 6 |
| `tools/` | Build, USB creation, lint, and QEMU scripts |
| `assets/`, `vendor/` | Firmware assets and bundled third-party components |
| `build/`, `isos/` | Ignored build outputs and local test images |

Each package has its own lockfile. Building the picker does not require Qt.
To check the desktop installer from the repository root:

```sh
cargo check --locked --manifest-path installer/Cargo.toml
```

See the [installer README](../installer/README.md) for desktop dependencies
and launch instructions.

## Toolchain

The picker builds with a current stable Rust toolchain. Install Rust with
[rustup](https://rustup.rs) on any distro and add the UEFI target:

```sh
rustup target add x86_64-unknown-uefi
```

Distro packages are alternatives, though distro Rust tends to be older
than `current stable`: Debian/Ubuntu `sudo apt install rustc cargo`,
Fedora `sudo dnf install rust cargo`, Arch `sudo pacman -S rust`.

## Building

From the source directory, with current stable Rust and Cargo:

```sh
rustup target add x86_64-unknown-uefi
tools/build-efi.sh
```

The result is `build/BOOTX64.EFI`, ready to copy to `EFI/BOOT/` on the USB.
The desktop installer and USB creation scripts use it automatically.
A fresh checkout does not contain a prebuilt picker.

## Font

The graphical menu embeds Geist from Vercel's v1.9.2 release as
pre-rendered glyphs under the SIL Open Font License in
`assets/OFL-Geist.txt`. The font sources and atlases are in `assets/`.
To regenerate the atlases, run `python3 tools/build_font_atlas.py` with
Pillow installed on the host.
