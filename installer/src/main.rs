use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use qmetaobject::{QObject, QObjectPinned, QString, QmlEngine};

mod usb;

const ISOS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "Debian Live 13.8 (standard)",
        "https://cdimage.debian.org/debian-cd/current-live/amd64/iso-hybrid/debian-live-13.8.0-amd64-standard.iso",
        "https://cdimage.debian.org/debian-cd/current-live/amd64/iso-hybrid/SHA256SUMS",
        "debian-live-13\\.8\\.0-amd64-standard\\.iso$",
        "debian",
    ),
    (
        "openSUSE Tumbleweed NET",
        "https://download.opensuse.org/tumbleweed/iso/openSUSE-Tumbleweed-NET-x86_64-Current.iso",
        "https://download.opensuse.org/tumbleweed/iso/openSUSE-Tumbleweed-NET-x86_64-Current.iso.sha256",
        "openSUSE-Tumbleweed-NET.*\\.iso$",
        "opensuse",
    ),
    (
        "Fedora Workstation 45",
        "https://dl.fedoraproject.org/pub/fedora/linux/releases/45/Workstation/x86_64/iso/Fedora-Workstation-Live-45-1.4.x86_64.iso",
        "https://dl.fedoraproject.org/pub/fedora/linux/releases/45/Workstation/x86_64/iso/Fedora-Workstation-45-1.4.x86_64-CHECKSUM",
        "Fedora-Workstation-Live-45-1\\.4\\.x86_64\\.iso\\) =",
        "fedora",
    ),
    (
        "Ubuntu 24.04 LTS · Desktop",
        "https://releases.ubuntu.com/24.04.3/ubuntu-24.04.3-desktop-amd64.iso",
        "https://releases.ubuntu.com/24.04.3/SHA256SUMS",
        "ubuntu-24\\.04\\.3-desktop-amd64\\.iso$",
        "ubuntu",
    ),
    (
        "Ubuntu 24.04 LTS · Server",
        "https://releases.ubuntu.com/24.04.3/ubuntu-24.04.3-live-server-amd64.iso",
        "https://releases.ubuntu.com/24.04.3/SHA256SUMS",
        "ubuntu-24\\.04\\.3-live-server-amd64\\.iso$",
        "ubuntu",
    ),
    (
        "Debian 13 · Network installer",
        "https://deb.debian.org/debian/dists/trixie/main/installer-amd64/current/images/netboot/mini.iso",
        "https://cdimage.debian.org/cdimage/archive/latest-oldstable/amd64/iso-cd/SHA256SUMS",
        "mini\\.iso$",
        "debian",
    ),
    (
        "Fedora Server 45 · Network installer",
        "https://download.fedoraproject.org/pub/fedora/linux/releases/45/Server/x86_64/iso/Fedora-Server-netinst-x86_64-45-1.4.iso",
        "https://download.fedoraproject.org/pub/fedora/linux/releases/45/Server/x86_64/iso/Fedora-Server-45-1.4-x86_64-CHECKSUM",
        "Fedora-Server-netinst-x86_64-45-1\\.4\\.iso",
        "fedora",
    ),
    (
        "Arch Linux · Live installer",
        "https://geo.mirror.pkgbuild.com/iso/latest/archlinux-x86_64.iso",
        "https://geo.mirror.pkgbuild.com/iso/latest/sha256sums.txt",
        "archlinux-x86_64\\.iso$",
        "arch",
    ),
    (
        "Rocky Linux 10 · Minimal",
        "https://download.rockylinux.org/pub/rocky/10/isos/x86_64/Rocky-10.0-x86_64-minimal.iso",
        "https://download.rockylinux.org/pub/rocky/10/isos/x86_64/CHECKSUM",
        "Rocky-10\\.0-x86_64-minimal\\.iso$",
        "rocky",
    ),
    (
        "AlmaLinux 10 · Minimal",
        "https://repo.almalinux.org/almalinux/10/isos/x86_64/AlmaLinux-10.0-x86_64-minimal.iso",
        "https://repo.almalinux.org/almalinux/10/isos/x86_64/CHECKSUM",
        "AlmaLinux-10\\.0-x86_64-minimal\\.iso$",
        "almalinux",
    ),
    (
        "SystemRescue · Rescue toolkit",
        "https://sourceforge.net/projects/systemrescuecd/files/sysresccd-x86/12.01/systemrescue-12.01-amd64.iso/download",
        "https://www.system-rescue.org/downloads/SHA256SUMS",
        "systemrescue-12\\.01-amd64\\.iso$",
        "systemrescue",
    ),
    (
        "Kali Linux · Installer",
        "https://cdimage.kali.org/kali-2025.4/kali-linux-2025.4-installer-amd64.iso",
        "https://cdimage.kali.org/kali-2025.4/SHA256SUMS",
        "kali-linux-2025\\.4-installer-amd64\\.iso$",
        "kali",
    ),
];

#[derive(Default, qmetaobject::QObject)]
struct Installer {
    base: qmetaobject::qt_base_class!(trait QObject),
    status: qmetaobject::qt_property!(QString; NOTIFY status_changed),
    status_changed: qmetaobject::qt_signal!(),
    iso_names: qmetaobject::qt_property!(QString),
    tool_names: qmetaobject::qt_property!(QString),
    iso_icons: qmetaobject::qt_property!(QString),
    iso_paths: qmetaobject::qt_property!(QString),
    devices: qmetaobject::qt_property!(QString; NOTIFY devices_changed),
    devices_changed: qmetaobject::qt_signal!(),
    refresh: qmetaobject::qt_method!(fn(&mut self)),
    download_iso: qmetaobject::qt_method!(fn(&mut self, index: i32)),
    create_usb: qmetaobject::qt_method!(
        fn(&mut self, iso_indices: QString, disk_index: i32, excluded_tools: QString)
    ),
}

impl Installer {
    fn set_status(&mut self, message: impl Into<QString>) {
        self.status = message.into();
        self.status_changed();
    }

    fn refresh(&mut self) {
        self.devices = list_usb_disks();
        self.devices_changed();
        self.set_status("USB device list refreshed");
    }

    fn download_iso(&mut self, index: i32) {
        let Some((label, url, checksum_url, pattern, _icon)) = ISOS.get(index as usize) else {
            self.set_status("Invalid ISO selection");
            return;
        };
        let destination_dir = iso_directory();
        let directory = destination_dir.join(".downloads");
        if let Err(error) = fs::create_dir_all(&directory) {
            self.set_status(format!("Cannot create download directory: {error}"));
            return;
        }
        let name = url.rsplit('/').next().unwrap_or("image.iso");
        let partial = directory.join(format!("{name}.part"));
        self.set_status(format!("Downloading {label}…"));
        let result = Command::new("curl")
            .args(["--fail", "--location", "--retry", "3", "--continue-at", "-"])
            .arg("--output")
            .arg(&partial)
            .arg(url)
            .status();
        if !result.is_ok_and(|status| status.success()) {
            self.set_status("Download failed; the partial file was kept for retry");
            return;
        }
        self.set_status("Verifying checksum…");
        let sums = directory.join(format!("{name}.sums"));
        let fetched = Command::new("curl")
            .args([
                "--fail",
                "--location",
                "--retry",
                "3",
                "--silent",
                "--show-error",
            ])
            .arg("--output")
            .arg(&sums)
            .arg(checksum_url)
            .status();
        let expected = Command::new("grep")
            .args(["-E", "--"])
            .arg(pattern)
            .arg(&sums)
            .output();
        let actual = Command::new("sha256sum").arg(&partial).output();
        let digest = |output: std::io::Result<std::process::Output>| {
            let output = output.ok().filter(|o| o.status.success())?;
            String::from_utf8(output.stdout)
                .ok()?
                .split_whitespace()
                .find(|word| word.len() == 64 && word.bytes().all(|b| b.is_ascii_hexdigit()))
                .map(str::to_ascii_lowercase)
        };
        let expected = digest(expected);
        let valid =
            fetched.is_ok_and(|s| s.success()) && expected.is_some() && expected == digest(actual);
        if !valid {
            self.set_status("Checksum verification failed; retry the download");
            return;
        }
        let destination = destination_dir.join(name);
        if fs::rename(&partial, &destination).is_err() {
            self.set_status("Verified download could not be installed");
        } else {
            self.set_status(format!("{label} downloaded and verified"));
        }
    }

    fn create_usb(&mut self, iso_indices: QString, disk_index: i32, excluded_tools: QString) {
        let device_list = self.devices.to_string();
        let selected = device_list
            .lines()
            .nth(disk_index.max(0) as usize)
            .and_then(|line| line.split_whitespace().next())
            .map(str::to_owned);
        let Some(disk) = selected else {
            self.set_status("Select a USB disk first");
            return;
        };
        let indexes: Vec<usize> = iso_indices
            .to_string()
            .split(',')
            .filter_map(|value| value.parse().ok())
            .filter(|index| *index < ISOS.len())
            .collect();
        if indexes.is_empty() {
            self.set_status("Select an ISO first");
            return;
        }
        let mut images = Vec::new();
        for index in indexes {
            let name = ISOS[index].1.rsplit('/').next().unwrap();
            let iso = iso_directory().join(name);
            if !iso.is_file() {
                self.set_status(format!("Download and verify {} first", ISOS[index].0));
                return;
            }
            images.push(iso);
        }
        let result = self.prepare_usb(&disk, &images, &excluded_tools.to_string());
        self.set_status(match result {
            Ok(()) => "USB created successfully".to_owned(),
            Err(error) => format!("USB creation failed: {error}"),
        });
    }

    fn prepare_usb(
        &mut self,
        disk: &str,
        images: &[PathBuf],
        excluded_tools: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let assets = usb::selected_assets(excluded_tools)?;
        let sequence = usb::sequence(Path::new(disk))?;
        let stage = usb::Temp::new()?;
        let result = (|| {
            if let Some(repo) = std::env::var("ISO_BOOT_PICKER_GITHUB_REPO")
                .ok()
                .or_else(|| option_env!("ISO_BOOT_PICKER_GITHUB_REPO").map(str::to_owned))
            {
                self.set_status("Downloading and verifying EFI binaries…");
                usb::download(&repo, &stage.0, &assets)?;
            } else {
                let root = project_root();
                let mut picker = root.join("build/BOOTX64.EFI");
                if !picker.is_file() {
                    picker = root.join("build/x86_64-unknown-uefi/release/iso-boot-picker.efi");
                }
                for &(name, vendor, _) in &assets {
                    let source = if vendor.is_empty() {
                        picker.clone()
                    } else {
                        root.join("vendor").join(vendor).join(name)
                    };
                    fs::copy(&source, stage.0.join(name))
                        .map_err(|error| format!("{}: {error}", source.display()))?;
                }
            }
            self.set_status(format!("Preparing {disk}…"));
            let output = Command::new("pkexec")
                .arg(std::env::current_exe()?)
                .args(["--write-usb", disk, sequence.trim()])
                .arg(&stage.0)
                .arg(excluded_tools)
                .args(
                    images
                        .iter()
                        .map(fs::canonicalize)
                        .collect::<std::io::Result<Vec<_>>>()?,
                )
                .output()?;
            if !output.status.success() {
                return Err(format!(
                    "{} {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
            Ok(())
        })();
        for name in assets
            .iter()
            .map(|(name, _, _)| *name)
            .chain(["SHA256SUMS"])
        {
            let _ = fs::remove_file(stage.0.join(name));
        }
        result
    }
}

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("installer directory")
        .to_path_buf()
}

fn iso_directory() -> PathBuf {
    if let Some(path) = std::env::var_os("ISO_BOOT_PICKER_ISO_DIR") {
        return PathBuf::from(path);
    }
    if std::env::var_os("ISO_BOOT_PICKER_GITHUB_REPO").is_some()
        || option_env!("ISO_BOOT_PICKER_GITHUB_REPO").is_some()
    {
        if let Some(base) = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
        {
            return base.join("iso-boot-picker/isos");
        }
    }
    project_root().join("isos")
}

fn list_usb_disks() -> QString {
    let output = Command::new("lsblk")
        .args(["-dnpo", "PATH,TYPE,TRAN,SIZE,MODEL,RO"])
        .output();
    let Ok(output) = output else {
        return "Unable to inspect USB devices".into();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .filter(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            fields.get(1) == Some(&"disk")
                && fields.get(2) == Some(&"usb")
                && fields.last() == Some(&"0")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .into()
}

fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if let Some(args) = args.strip_prefix(&[std::ffi::OsString::from("--write-usb")]) {
        let result = if std::env::var_os("PKEXEC_UID").is_none() {
            Err("Run the writer via the installer’s privilege prompt".into())
        } else {
            usb::write(args)
        };
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    let mut engine = QmlEngine::new();
    let names = ISOS
        .iter()
        .map(|entry| entry.0)
        .collect::<Vec<_>>()
        .join("\n");
    let icons = ISOS
        .iter()
        .map(|entry| entry.4)
        .collect::<Vec<_>>()
        .join("\n");
    let installer = RefCell::new(Installer {
        status: "Ready".into(),
        iso_names: names.into(),
        iso_icons: icons.into(),
        tool_names: usb::TOOLS
            .iter()
            .map(|tool| tool.2)
            .collect::<Vec<_>>()
            .join("\n")
            .into(),
        ..Default::default()
    });
    engine.set_object_property("installer".into(), unsafe {
        QObjectPinned::new(&installer)
    });
    engine.load_data(include_bytes!("ui.qml").as_slice().into());
    engine.exec();
}
