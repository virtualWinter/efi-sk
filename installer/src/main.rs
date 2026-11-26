use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use qmetaobject::{QObject, QObjectPinned, QString, QmlEngine};

mod usb;

qmetaobject::qrc!(register_iso_icons,
    "icons" as "iso-icons" {
        "debian.png", "opensuse.png", "fedora.png", "ubuntu.png",
        "archlinux.png", "rockylinux.png", "almalinux.png",
        "systemrescue.png", "kalilinux.png", "proxmox.png", "opnsense.png",
        "memory-stick.png", "terminal.png", "key-round.png", "network.png",
        "file-search.png", "hard-drive.png", "clipboard-check.png",
    }
);

const ISOS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "VyOS rolling: Experimental",
        "https://github.com/vyos/vyos-nightly-build/releases/download/2026.11.22-0018-rolling/vyos-2026.11.22-0018-rolling-generic-amd64.iso",
        "sha256:9c7915e8c113ed371e23115610d9d1d6e642e466844eed10cd74ed2c207e8123",
        "",
        "debian",
    ),
    (
        "OPNsense 26.7 DVD",
        "https://pkg.opnsense.org/releases/mirror/OPNsense-26.7-dvd-amd64.iso.bz2",
        "sha256:95cafedda6d5b22ce832e249dc2309110fbee19f813ad78cf28bb3d387186bfb",
        "",
        "opnsense",
    ),
    (
        "Proxmox VE 9.2",
        "https://enterprise.proxmox.com/iso/proxmox-ve_9.2-1.iso",
        "sha256:4e88fe416df9b527624a175f24c9aa07c714d3332afb1ee3dbf3879573ef2c6c",
        "",
        "proxmox",
    ),
    (
        "OpenWrt 25.12.6: Download only",
        "https://downloads.openwrt.org/releases/25.12.6/targets/x86/64/openwrt-25.12.6-x86-64-generic-ext4-combined-efi.img.gz",
        "https://downloads.openwrt.org/releases/25.12.6/targets/x86/64/sha256sums",
        "openwrt-25\\.12\\.6-x86-64-generic-ext4-combined-efi\\.img\\.gz$",
        "systemrescue",
    ),
    (
        "Xubuntu 24.04 LTS",
        "https://cdimage.ubuntu.com/xubuntu/releases/24.04/release/xubuntu-24.04.5-desktop-amd64.iso",
        "https://cdimage.ubuntu.com/xubuntu/releases/24.04/release/SHA256SUMS",
        "xubuntu-24\\.04\\.5-desktop-amd64\\.iso$",
        "ubuntu",
    ),
    (
        "Lubuntu 24.04 LTS",
        "https://cdimage.ubuntu.com/lubuntu/releases/24.04/release/lubuntu-24.04.5-desktop-amd64.iso",
        "https://cdimage.ubuntu.com/lubuntu/releases/24.04/release/SHA256SUMS",
        "lubuntu-24\\.04\\.5-desktop-amd64\\.iso$",
        "ubuntu",
    ),
    (
        "Kubuntu 24.04 LTS",
        "https://cdimage.ubuntu.com/kubuntu/releases/24.04/release/kubuntu-24.04.5-desktop-amd64.iso",
        "https://cdimage.ubuntu.com/kubuntu/releases/24.04/release/SHA256SUMS",
        "kubuntu-24\\.04\\.5-desktop-amd64\\.iso$",
        "ubuntu",
    ),
    (
        "Ubuntu MATE 24.04 LTS",
        "https://cdimage.ubuntu.com/ubuntu-mate/releases/24.04/release/ubuntu-mate-24.04.5-desktop-amd64.iso",
        "https://cdimage.ubuntu.com/ubuntu-mate/releases/24.04/release/SHA256SUMS",
        "ubuntu-mate-24\\.04\\.5-desktop-amd64\\.iso$",
        "ubuntu",
    ),
    (
        "Ubuntu Studio 24.04 LTS",
        "https://cdimage.ubuntu.com/ubuntustudio/releases/24.04/release/ubuntustudio-24.04.5-dvd-amd64.iso",
        "https://cdimage.ubuntu.com/ubuntustudio/releases/24.04/release/SHA256SUMS",
        "ubuntustudio-24\\.04\\.5-dvd-amd64\\.iso$",
        "ubuntu",
    ),
    (
        "Ubuntu Kylin 24.04 LTS",
        "https://cdimage.ubuntu.com/ubuntukylin/releases/24.04/release/ubuntukylin-24.04.5-desktop-amd64.iso",
        "https://cdimage.ubuntu.com/ubuntukylin/releases/24.04/release/SHA256SUMS",
        "ubuntukylin-24\\.04\\.5-desktop-amd64\\.iso$",
        "ubuntu",
    ),
    (
        "Ubuntu Unity 24.04 LTS",
        "https://cdimage.ubuntu.com/ubuntu-unity/releases/24.04/release/ubuntu-unity-24.04.5-desktop-amd64.iso",
        "https://cdimage.ubuntu.com/ubuntu-unity/releases/24.04/release/SHA256SUMS",
        "ubuntu-unity-24\\.04\\.5-desktop-amd64\\.iso$",
        "ubuntu",
    ),
    (
        "Edubuntu 24.04 LTS",
        "https://cdimage.ubuntu.com/edubuntu/releases/24.04/release/edubuntu-24.04.5-desktop-amd64.iso",
        "https://cdimage.ubuntu.com/edubuntu/releases/24.04/release/SHA256SUMS",
        "edubuntu-24\\.04\\.5-desktop-amd64\\.iso$",
        "ubuntu",
    ),
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
        "Ubuntu 24.04 LTS: Desktop",
        "https://releases.ubuntu.com/24.04.3/ubuntu-24.04.3-desktop-amd64.iso",
        "https://releases.ubuntu.com/24.04.3/SHA256SUMS",
        "ubuntu-24\\.04\\.3-desktop-amd64\\.iso$",
        "ubuntu",
    ),
    (
        "Ubuntu 24.04 LTS: Server",
        "https://releases.ubuntu.com/24.04.3/ubuntu-24.04.3-live-server-amd64.iso",
        "https://releases.ubuntu.com/24.04.3/SHA256SUMS",
        "ubuntu-24\\.04\\.3-live-server-amd64\\.iso$",
        "ubuntu",
    ),
    (
        "Debian 13: Network installer",
        "https://deb.debian.org/debian/dists/trixie/main/installer-amd64/current/images/netboot/mini.iso",
        "https://cdimage.debian.org/cdimage/archive/latest-oldstable/amd64/iso-cd/SHA256SUMS",
        "mini\\.iso$",
        "debian",
    ),
    (
        "Fedora Server 45: Network installer",
        "https://download.fedoraproject.org/pub/fedora/linux/releases/45/Server/x86_64/iso/Fedora-Server-netinst-x86_64-45-1.4.iso",
        "https://download.fedoraproject.org/pub/fedora/linux/releases/45/Server/x86_64/iso/Fedora-Server-45-1.4-x86_64-CHECKSUM",
        "Fedora-Server-netinst-x86_64-45-1\\.4\\.iso",
        "fedora",
    ),
    (
        "Arch Linux: Live installer",
        "https://geo.mirror.pkgbuild.com/iso/latest/archlinux-x86_64.iso",
        "https://geo.mirror.pkgbuild.com/iso/latest/sha256sums.txt",
        "archlinux-x86_64\\.iso$",
        "archlinux",
    ),
    (
        "Rocky Linux 10: Minimal",
        "https://download.rockylinux.org/pub/rocky/10/isos/x86_64/Rocky-10.0-x86_64-minimal.iso",
        "https://download.rockylinux.org/pub/rocky/10/isos/x86_64/CHECKSUM",
        "Rocky-10\\.0-x86_64-minimal\\.iso$",
        "rockylinux",
    ),
    (
        "AlmaLinux 10: Minimal",
        "https://repo.almalinux.org/almalinux/10/isos/x86_64/AlmaLinux-10.0-x86_64-minimal.iso",
        "https://repo.almalinux.org/almalinux/10/isos/x86_64/CHECKSUM",
        "AlmaLinux-10\\.0-x86_64-minimal\\.iso$",
        "almalinux",
    ),
    (
        "SystemRescue: Rescue toolkit",
        "https://sourceforge.net/projects/systemrescuecd/files/sysresccd-x86/12.01/systemrescue-12.01-amd64.iso/download",
        "https://www.system-rescue.org/downloads/SHA256SUMS",
        "systemrescue-12\\.01-amd64\\.iso$",
        "systemrescue",
    ),
    (
        "Kali Linux: Installer",
        "https://cdimage.kali.org/kali-2025.4/kali-linux-2025.4-installer-amd64.iso",
        "https://cdimage.kali.org/kali-2025.4/SHA256SUMS",
        "kali-linux-2025\\.4-installer-amd64\\.iso$",
        "kalilinux",
    ),
];

#[derive(Default, qmetaobject::QObject)]
struct Installer {
    base: qmetaobject::qt_base_class!(trait QObject),
    status: qmetaobject::qt_property!(QString; NOTIFY status_changed),
    status_changed: qmetaobject::qt_signal!(),
    busy: qmetaobject::qt_property!(bool; NOTIFY status_changed),
    installing: qmetaobject::qt_property!(bool; NOTIFY status_changed),
    skip_iso_checksum: bool,
    updates: Option<std::sync::mpsc::Receiver<String>>,
    poll_updates: qmetaobject::qt_method!(fn(&mut self)),
    iso_names: qmetaobject::qt_property!(QString),
    tool_names: qmetaobject::qt_property!(QString),
    release_mode: qmetaobject::qt_property!(bool),
    iso_icons: qmetaobject::qt_property!(QString),
    iso_paths: qmetaobject::qt_property!(QString),
    devices: qmetaobject::qt_property!(QString; NOTIFY devices_changed),
    devices_changed: qmetaobject::qt_signal!(),
    refresh: qmetaobject::qt_method!(fn(&mut self)),
    download_iso: qmetaobject::qt_method!(fn(&mut self, index: i32)),
    create_usb: qmetaobject::qt_method!(
        fn(
            &mut self,
            iso_indices: QString,
            disk_index: i32,
            excluded_tools: QString,
            efi_version: QString,
        )
    ),
}

impl Installer {
    fn set_status(&mut self, message: impl Into<QString>) {
        self.status = message.into();
        eprintln!("[efi-sk] {}", self.status);
        self.status_changed();
    }

    fn refresh(&mut self) {
        eprintln!("[efi-sk] Refreshing USB drives");
        self.devices = list_usb_disks();
        self.devices_changed();
    }

    fn download_iso(&mut self, index: i32) {
        let Some((label, url, checksum_url, pattern, _icon)) = ISOS.get(index as usize) else {
            self.set_status("Invalid ISO selection");
            return;
        };
        if self.busy {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.updates = Some(rx);
        self.busy = true;
        self.set_status(format!("Preparing to download {label}…"));
        let skip = self.skip_iso_checksum;
        std::thread::spawn(move || {
            let report = |message| {
                let _ = tx.send(message);
            };
            match fetch_iso(label, url, checksum_url, pattern, skip, report) {
                Ok(_) if skip => report(format!("{label} downloaded (checksum skipped)")),
                Ok(_) => report(format!("{label} downloaded and verified")),
                Err(error) => report(error),
            }
        });
    }

    fn poll_updates(&mut self) {
        while let Some(rx) = &self.updates {
            match rx.try_recv() {
                Ok(message) => self.set_status(message),
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.updates = None;
                    self.busy = false;
                    self.installing = false;
                    self.status_changed();
                }
            }
        }
    }

    fn create_usb(
        &mut self,
        iso_indices: QString,
        disk_index: i32,
        excluded_tools: QString,
        efi_version: QString,
    ) {
        if self.busy {
            return;
        }
        let device_list = self.devices.to_string();
        let selected = device_list
            .lines()
            .nth(usize::try_from(disk_index).unwrap_or(usize::MAX))
            .and_then(|line| line.split_whitespace().next())
            .map(str::to_owned);
        let Some(disk) = selected else {
            self.set_status("Select a USB drive");
            return;
        };
        let mut selected_images = Vec::new();
        let selection = iso_indices.to_string();
        for value in selection.split(',').filter(|_| !selection.is_empty()) {
            let Some(index) = value.parse::<usize>().ok().filter(|i| *i < ISOS.len()) else {
                self.set_status("Invalid ISO selection");
                return;
            };
            if ISOS[index].0.ends_with(": Download only") {
                self.set_status(format!("{} is available to download only. Deselect it to create the USB.", ISOS[index].0.trim_end_matches(": Download only")));
                return;
            }
            selected_images.push(index);
        }
        let excluded = excluded_tools.to_string();
        let version = efi_version.to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        self.updates = Some(rx);
        self.busy = true;
        self.installing = true;
        self.set_status(format!("Preparing {disk}…"));
        let skip = self.skip_iso_checksum;
        std::thread::spawn(move || {
            let report = |message| {
                let _ = tx.send(message);
            };
            let result = Self::prepare_usb(
                &disk,
                &selected_images,
                &excluded,
                version.trim(),
                skip,
                report,
            );
            report(match result {
                Ok(()) => format!("Bootable USB ready: {disk}"),
                Err(error) => format!("USB creation failed: {error}"),
            });
        });
    }

    fn prepare_usb(
        disk: &str,
        selected_images: &[usize],
        excluded_tools: &str,
        efi_version: &str,
        skip: bool,
        report: impl Fn(String),
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::io::{BufRead, BufReader};
        use std::process::Stdio;
        let assets = usb::selected_assets(excluded_tools)?;
        let sequence = usb::sequence(Path::new(disk))?;
        let mut images = Vec::new();
        for &index in selected_images {
            let (label, url, sums, pattern, _) = ISOS[index];
            let path = image_path(url, false);
            let unchecked = image_path(url, true);
            images.push(if path.is_file() {
                report(format!("Using local image: {}", path.display()));
                path
            } else if skip && unchecked.is_file() {
                report(format!(
                    "Using unchecked local image: {}",
                    unchecked.display()
                ));
                unchecked
            } else {
                report(format!("Fetching {label}…"));
                fetch_iso(label, url, sums, pattern, skip, &report)
                    .map_err(|error| format!("{label}: {error}"))?
            });
        }
        let stage = usb::Temp::new()?;
        let result = (|| {
            if let Some(repo) = release_repo() {
                report("Downloading and verifying EFI binaries…".into());
                usb::download(&repo, efi_version, &stage.0, &assets)?;
            } else {
                let root = project_root();
                let mut picker = root.join("build/BOOTX64.EFI");
                if !picker.is_file() {
                    picker = root.join("build/x86_64-unknown-uefi/release/efi-sk.efi");
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
            if images.iter().any(|image| {
                image
                    .file_name()
                    .is_some_and(|name| name == "OPNsense-26.7-dvd-amd64.iso")
            }) {
                let picker = fs::read(stage.0.join("BOOTX64.EFI"))?;
                // Older picker releases did not embed the GEOM module.
                let marker = b"g_ventoy_map_data\0";
                if !picker.windows(marker.len()).any(|part| part == marker) {
                    return Err(
                        "OPNsense requires an EFI build with the embedded GEOM module".into(),
                    );
                }
            }
            report(format!("Waiting for authorization to write {disk}…"));
            let errors = stage.0.join("writer.stderr");
            let mut child = Command::new("pkexec")
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
                .stdout(Stdio::piped())
                .stderr(fs::File::create_new(&errors)?)
                .spawn()?;
            for line in BufReader::new(child.stdout.take().unwrap()).lines() {
                if let Some(message) = line?.strip_prefix("progress: ") {
                    report(message.to_owned());
                }
            }
            let status = child.wait()?;
            if !status.success() {
                return Err(format!("{status}: {}", fs::read_to_string(errors)?.trim()).into());
            }
            Ok(())
        })();
        for name in assets.iter().map(|(name, _, _)| *name).chain([
            "SHA256SUMS",
            usb::EFI_ARCHIVE,
            "writer.stderr",
        ]) {
            let _ = fs::remove_file(stage.0.join(name));
        }
        result
    }
}

fn image_filename(url: &str) -> &str {
    url.strip_suffix("/download")
        .unwrap_or(url)
        .rsplit('/')
        .next()
        .unwrap()
}

fn image_path(url: &str, skip: bool) -> PathBuf {
    let name = image_filename(url);
    let mut dir = iso_directory();
    if skip {
        dir.push("unchecked");
    }
    dir.join(name.strip_suffix(".bz2").unwrap_or(name))
}

fn fetch_iso(
    label: &str,
    url: &str,
    checksum_url: &str,
    pattern: &str,
    skip: bool,
    report: impl Fn(String),
) -> Result<PathBuf, String> {
    let destination = image_path(url, skip);
    let directory = destination.parent().unwrap().join(".downloads");
    if let Err(error) = fs::create_dir_all(&directory) {
        return Err(format!("Cannot create download directory: {error}"));
    }
    let name = image_filename(url);
    let partial = directory.join(format!("{name}.part"));
    let result = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--fail",
            "--location",
            "--retry",
            "3",
            "--continue-at",
            "-",
        ])
        .arg("--output")
        .arg(&partial)
        .arg(url)
        .spawn()
        .and_then(|mut child| {
            loop {
                if let Some(status) = child.try_wait()? {
                    break Ok(status);
                }
                let bytes = fs::metadata(&partial).map_or(0, |meta| meta.len());
                report(format!(
                    "Downloading {label}: {:.1} MiB received",
                    bytes as f64 / 1048576.0
                ));
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        });
    if !result.is_ok_and(|status| status.success()) {
        return Err("Download failed; the partial file was kept for retry".into());
    }
    if !skip {
        report("Verifying checksum…".into());
        let sums = directory.join(format!("{name}.sums"));
        let fetched = (|| {
            if checksum_url.starts_with("sha256:") {
                return Ok(true);
            }
            Command::new("curl")
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
                .status()
                .map(|status| status.success())
        })();
        let actual = Command::new("sha256sum").arg(&partial).output();
        let digest = |output: std::io::Result<std::process::Output>| {
            let output = output.ok().filter(|o| o.status.success())?;
            String::from_utf8(output.stdout)
                .ok()?
                .split_whitespace()
                .find(|word| word.len() == 64 && word.bytes().all(|b| b.is_ascii_hexdigit()))
                .map(str::to_ascii_lowercase)
        };
        let expected = if let Some(pinned) = checksum_url.strip_prefix("sha256:") {
            Some(pinned.to_owned())
        } else {
            digest(
                Command::new("grep")
                    .args(["-E", "--"])
                    .arg(pattern)
                    .arg(&sums)
                    .output(),
            )
        };
        let valid = fetched.unwrap_or(false) && expected.is_some() && expected == digest(actual);
        if !valid {
            return Err("Checksum verification failed; retry the download".into());
        }
    }
    if name.ends_with(".iso.bz2") {
        report("Extracting ISO…".into());
        let extracted = directory.join(format!("{name}.{}.tmp", std::process::id()));
        let out = match fs::File::create_new(&extracted) {
            Ok(out) => out,
            Err(error) => {
                return Err(format!("Could not stage extracted ISO: {error}"));
            }
        };
        let result = (|| -> std::io::Result<()> {
            let status = Command::new("bzip2")
                .arg("-dc")
                .arg("--")
                .arg(&partial)
                .stdout(out)
                .status()?;
            if !status.success() {
                return Err(std::io::Error::other("bzip2 extraction failed"));
            }
            fs::rename(&extracted, &destination)?;
            let _ = fs::remove_file(&partial);
            Ok(())
        })();
        if let Err(error) = result {
            let _ = fs::remove_file(&extracted);
            return Err(format!("Could not extract ISO: {error}"));
        }
    } else {
        fs::rename(&partial, &destination).map_err(|e| format!("Could not store download: {e}"))?;
    }
    Ok(destination)
}

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("installer directory")
        .to_path_buf()
}

fn release_repo() -> Option<String> {
    std::env::var("EFI_SK_GITHUB_REPO")
        .ok()
        .or_else(|| option_env!("EFI_SK_GITHUB_REPO").map(str::to_owned))
}

fn iso_directory() -> PathBuf {
    if let Some(path) = std::env::var_os("EFI_SK_ISO_DIR") {
        return PathBuf::from(path);
    }
    if release_repo().is_some()
        && let Some(base) = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
    {
        return base.join("efiswissknife/isos");
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
    let skip_iso_checksum = args.iter().any(|arg| arg == "--skip-iso-checksum");
    eprintln!(
        "[efi-sk] Starting installer; ISO directory: {}; image checksum validation: {}",
        iso_directory().display(),
        if skip_iso_checksum { "off" } else { "on" }
    );
    let mut engine = QmlEngine::new();
    register_iso_icons();
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
        devices: list_usb_disks(),
        skip_iso_checksum,
        release_mode: release_repo().is_some(),
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
    drop(engine);
}
