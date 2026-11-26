use std::{
    fs,
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub type Asset = (&'static str, &'static str, &'static str);
pub const EFI_ARCHIVE: &str = "efi-sk-efi-x86_64.tar.gz";
const PICKER: Asset = ("BOOTX64.EFI", "", "EFI Swiss Knife");
const SHELL: Asset = ("shell.efi", "edk2-shell", "EFI Shell");
const SCT: Asset = ("sct-x64.7z", "uefi-sct", "UEFI SCT (includes EFI Shell)");
pub const TOOLS: &[Asset] = &[
    ("memtest86plus.efi", "memtest86plus", "Memtest86+"),
    SHELL,
    ("KeyTool.efi", "keytool", "KeyTool"),
    ("ipxe.efi", "ipxe", "iPXE"),
    ("acpidump.efi", "acpidump", "ACPIdump"),
    ("gdisk_x64.efi", "gpt-fdisk", "GPT fdisk"),
    SCT,
];

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn selected_assets(excluded: &str) -> Result<Vec<Asset>> {
    let mut included = vec![true; TOOLS.len()];
    if !excluded.is_empty() {
        for index in excluded.split(',') {
            let index = index.parse::<usize>()?;
            *included
                .get_mut(index)
                .ok_or("Invalid EFI tool selection")? = false;
        }
    }
    let mut assets = vec![PICKER];
    assets.extend(
        TOOLS
            .iter()
            .copied()
            .zip(included)
            .filter_map(|(tool, include)| include.then_some(tool)),
    );
    if assets.contains(&SCT) && !assets.contains(&SHELL) {
        assets.push(SHELL);
    }
    Ok(assets)
}

pub struct Temp(pub PathBuf);

impl Temp {
    pub fn new() -> Result<Self> {
        Ok(Self(
            output(Command::new("mktemp").arg("-d"))?.trim().into(),
        ))
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        // Never remove the contents recursively: a failed unmount could
        // leave a filesystem mounted beneath this directory.
        let _ = fs::remove_dir(&self.0);
    }
}

fn output(command: &mut Command) -> Result<String> {
    let result = command.output()?;
    if !result.status.success() {
        return Err(format!("{command:?}: {}", String::from_utf8_lossy(&result.stderr)).into());
    }
    Ok(String::from_utf8(result.stdout)?)
}

fn run(command: &mut Command) -> Result<()> {
    output(command).map(drop)
}

fn release_path(version: &str) -> Result<String> {
    if version.is_empty() || version == "latest" {
        return Ok("latest/download".into());
    }
    if !version
        .strip_prefix('v')
        .or_else(|| version.strip_prefix("esk-"))
        .is_some_and(|suffix| {
            !suffix.is_empty()
                && suffix.bytes().next().is_some_and(|b| b.is_ascii_digit())
                && suffix
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
        })
    {
        return Err("Enter latest or an EFI release tag such as v0.1.1".into());
    }
    Ok(format!("download/{version}"))
}

pub fn download(repo: &str, version: &str, dir: &Path, assets: &[Asset]) -> Result<()> {
    let valid = |part: &str| {
        !part.is_empty()
            && part != "."
            && part != ".."
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    };
    if !repo
        .split_once('/')
        .is_some_and(|(owner, name)| valid(owner) && valid(name))
    {
        return Err("Expected a GitHub repository in OWNER/REPO form".into());
    }
    let release = release_path(version)?;
    let fetch = |name: &str| {
        run(Command::new("curl")
            .args([
                "--fail",
                "--silent",
                "--show-error",
                "--location",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--connect-timeout",
                "15",
                "--max-time",
                "180",
                "--retry",
                "2",
            ])
            .arg("--output")
            .arg(dir.join(name))
            .arg(format!(
                "https://github.com/{repo}/releases/{release}/{name}"
            )))
    };
    fetch("SHA256SUMS")?;
    let sums = fs::read_to_string(dir.join("SHA256SUMS"))?;
    let has_archive = sums.lines().any(|line| {
        line.split_once("  ")
            .is_some_and(|(_, file)| file == EFI_ARCHIVE)
    });
    let fetch_verified = |name: &str| -> Result<()> {
        let mut matches = sums.lines().filter_map(|line| {
            let (hash, file) = line.split_once("  ")?;
            (file == name && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
                .then_some(hash)
        });
        let expected = matches
            .next()
            .ok_or_else(|| format!("Missing release checksum: {name}"))?;
        if matches.next().is_some() {
            return Err(format!("Duplicate release checksum: {name}").into());
        }
        fetch(name)?;
        let actual = output(Command::new("sha256sum").arg(dir.join(name)))?;
        if !actual
            .split_whitespace()
            .next()
            .is_some_and(|hash| hash.eq_ignore_ascii_case(expected))
        {
            return Err(format!("Checksum mismatch: {name}").into());
        }
        Ok(())
    };
    if has_archive {
        fetch_verified(EFI_ARCHIVE)?;
        stage_archive(dir, assets)?;
    } else {
        // Older firmware releases published the EFI files individually.
        for (name, _, _) in assets {
            fetch_verified(name)?;
        }
    }
    Ok(())
}

fn stage_archive(dir: &Path, assets: &[Asset]) -> Result<()> {
    for (name, _, _) in assets {
        // Stream an exact member into a file we create, rather than honoring
        // archive paths, links, ownership, or modes.
        let result = Command::new("tar")
            .args(["-xOzf"])
            .arg(dir.join(EFI_ARCHIVE))
            .args(["--", name])
            .stdout(fs::File::create_new(dir.join(name))?)
            .output()?;
        if !result.status.success() || fs::metadata(dir.join(name))?.len() == 0 {
            return Err(format!(
                "Could not extract {name}: {}",
                String::from_utf8_lossy(&result.stderr)
            )
            .into());
        }
    }
    Ok(())
}

fn inspect(disk: &Path, columns: &str, flags: &str) -> Result<String> {
    output(Command::new("lsblk").args([flags, columns, "--"]).arg(disk))
}

pub fn sequence(disk: &Path) -> Result<String> {
    Ok(fs::read_to_string(
        Path::new("/sys/class/block")
            .join(disk.file_name().ok_or("Missing device name")?)
            .join("diskseq"),
    )?)
}

pub fn write(args: &[std::ffi::OsString]) -> Result<()> {
    use std::os::unix::fs::FileTypeExt;

    let [disk, expected_sequence, dir, excluded, images @ ..] = args else {
        return Err("Missing USB writer arguments".into());
    };
    println!("progress: Checking USB drive…");
    let assets = selected_assets(excluded.to_str().ok_or("Invalid EFI tool selection")?)?;
    let include_sct = assets.contains(&SCT);
    let disk = fs::canonicalize(disk)?;
    let dir = Path::new(dir);
    if !disk.starts_with("/dev") || !fs::metadata(&disk)?.file_type().is_block_device() {
        return Err("Select a whole USB disk".into());
    }
    let identity = inspect(&disk, "TYPE,TRAN,RO", "-dnro")?;
    if !identity.split_whitespace().eq(["disk", "usb", "0"])
        || !inspect(&disk, "MOUNTPOINTS", "-nro")?.trim().is_empty()
        || inspect(&disk, "TYPE", "-nro")?
            .lines()
            .any(|line| !matches!(line.trim(), "disk" | "part"))
        || sequence(&disk)?.trim() != expected_sequence.to_string_lossy()
    {
        return Err("The USB changed, is mounted, or is not a writable whole USB disk".into());
    }
    let mut total = 0u64;
    for image in images {
        let path = Path::new(image);
        let metadata = fs::metadata(path)?;
        if !metadata.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("iso") {
            return Err("Invalid ISO file".into());
        }
        total = total
            .checked_add(metadata.len())
            .ok_or("ISO size overflow")?;
    }
    let capacity: u64 = inspect(&disk, "SIZE", "-bdnro")?.trim().parse()?;
    if capacity < total.saturating_add(768 * 1024 * 1024) {
        return Err("Not enough space on the USB drive".into());
    }
    for (asset, _, _) in &assets {
        if !fs::metadata(dir.join(asset))?.is_file() {
            return Err(format!("Missing asset: {asset}").into());
        }
    }
    // Check command availability before touching the disk.
    for name in [
        "sfdisk",
        "udevadm",
        "mkfs.fat",
        "mkfs.ext4",
        "mount",
        "umount",
    ]
    .into_iter()
    .chain(include_sct.then_some("bsdtar"))
    {
        if !["/usr/sbin", "/usr/bin", "/sbin", "/bin"]
            .iter()
            .any(|p| Path::new(p).join(name).is_file())
        {
            return Err(format!("Missing utility: {name}").into());
        }
    }
    if include_sct {
        run(Command::new("bsdtar").arg("-tf").arg(dir.join(SCT.0)))?;
    }
    if sequence(&disk)?.trim() != expected_sequence.to_string_lossy() {
        return Err("The USB drive changed".into());
    }
    println!("progress: Partitioning USB drive…");
    let mut partition = Command::new("sfdisk")
        .args(["--wipe", "always", "--wipe-partitions", "always", "--"])
        .arg(&disk)
        .stdin(Stdio::piped())
        .spawn()?;
    partition
        .stdin
        .take()
        .ok_or("Missing partitioner stdin")?
        .write_all(b"label: gpt\nstart=1MiB, size=512MiB, type=U\ntype=L\n")?;
    if !partition.wait()?.success() {
        return Err("Partitioning failed".into());
    }
    run(Command::new("udevadm").arg("settle"))?;
    let entries = inspect(&disk, "PATH,PARTN,TYPE", "-nrpo")?;
    let part = |number| -> Result<&str> {
        entries
            .lines()
            .find_map(|line| {
                let fields: Vec<_> = line.split_whitespace().collect();
                match fields.as_slice() {
                    [path, n, "part"] if *n == number => Some(*path),
                    _ => None,
                }
            })
            .ok_or_else(|| "Partition did not appear".into())
    };
    let (esp, isos) = (part("1")?, part("2")?);
    if sequence(&disk)?.trim() != expected_sequence.to_string_lossy() {
        return Err("The USB drive changed".into());
    }
    println!("progress: Formatting partitions…");
    stamp_disk_identity(&disk)?;
    run(Command::new("mkfs.fat").args(["-F", "32", "-n", "PICKEREFI", "--", esp]))?;
    run(Command::new("mkfs.ext4").args(["-q", "-F", "-m", "0", "-L", "ISOS", "--", isos]))?;
    let efi = Mount::new(esp)?;
    let data = Mount::new(isos)?;
    let boot = efi.dir.0.join("EFI/BOOT");
    let iso = data.dir.0.join("iso");
    fs::create_dir_all(&boot)?;
    fs::create_dir(&iso)?;
    println!("progress: Installing EFI picker and tools…");
    for (asset, _, _) in assets.into_iter().filter(|asset| *asset != SCT) {
        fs::copy(dir.join(asset), boot.join(asset))?;
    }
    if include_sct {
        run(Command::new("bsdtar")
            .args(["-xf"])
            .arg(dir.join(SCT.0))
            .arg("-C")
            .arg(&efi.dir.0))?;
    }
    for image in images {
        let path = Path::new(image);
        println!(
            "progress: Copying {}…",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        fs::copy(
            path,
            iso.join(path.file_name().ok_or("Missing ISO filename")?),
        )?;
    }
    println!("progress: Flushing writes and unmounting…");
    data.unmount()?;
    efi.unmount()?;
    Ok(())
}

fn stamp_disk_identity(disk: &Path) -> Result<()> {
    let mut identity = *b"EFISKMAP\0\0\0\0\0\0\0\0";
    fs::File::open("/dev/urandom")?.read_exact(&mut identity[8..])?;
    let mut device = fs::OpenOptions::new().write(true).open(disk)?;
    // Sparse ISO blocks use this zero backing range, entirely before the ESP.
    device.seek(SeekFrom::Start(128 * 1024))?;
    for _ in 0..128 {
        device.write_all(&[0; 4096])?;
    }
    // The first GPT partition begins at 1 MiB; this is the unused PMBR
    // bootstrap area, not the disk signature or the partition entries.
    device.seek(SeekFrom::Start(384))?;
    device.write_all(&identity)?;
    device.sync_all()?;
    Ok(())
}

struct Mount {
    dir: Temp,
    mounted: bool,
}

impl Mount {
    fn new(device: &str) -> Result<Self> {
        let dir = Temp::new()?;
        run(Command::new("mount")
            .args(["-o", "nosuid,nodev,noexec", "--", device])
            .arg(&dir.0))?;
        Ok(Self { dir, mounted: true })
    }

    fn unmount(mut self) -> Result<()> {
        run(Command::new("umount").arg("--").arg(&self.dir.0))?;
        self.mounted = false;
        Ok(())
    }
}

impl Drop for Mount {
    fn drop(&mut self) {
        if self.mounted {
            let _: io::Result<_> = Command::new("umount").arg("--").arg(&self.dir.0).status();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_only_the_reserved_disk_ranges() -> Result<()> {
        let dir = Temp::new()?;
        let disk = dir.0.join("disk.img");
        let original = vec![0x5a; 1024 * 1024 + 1];
        fs::write(&disk, &original)?;
        let result = (|| {
            println!("progress: Formatting partitions…");
            stamp_disk_identity(&disk)?;
            let stamped = fs::read(&disk)?;
            assert_eq!(&stamped[..384], &original[..384]);
            assert_eq!(&stamped[384..392], b"EFISKMAP");
            assert_eq!(&stamped[400..128 * 1024], &original[400..128 * 1024]);
            assert!(
                stamped[128 * 1024..640 * 1024]
                    .iter()
                    .all(|&byte| byte == 0)
            );
            assert_eq!(&stamped[640 * 1024..], &original[640 * 1024..]);
            Ok(())
        })();
        let _ = fs::remove_file(&disk);
        result
    }

    #[test]
    fn defaults_include_every_tool_and_empty_selection_keeps_picker() {
        assert_eq!(selected_assets("").unwrap().len(), TOOLS.len() + 1);
        assert_eq!(selected_assets("0,1,2,3,4,5,6").unwrap(), [PICKER]);
    }

    #[test]
    fn sct_requires_shell() {
        assert_eq!(
            selected_assets("0,1,2,3,4,5").unwrap(),
            [PICKER, SCT, SHELL]
        );
    }

    #[test]
    fn rejects_unknown_exclusions() {
        assert!(selected_assets("999").is_err());
        assert!(selected_assets("invalid").is_err());
    }

    #[test]
    fn validates_release_versions() -> Result<()> {
        assert_eq!(release_path("")?, "latest/download");
        assert_eq!(release_path("latest")?, "latest/download");
        assert_eq!(release_path("v0.1.1")?, "download/v0.1.1");
        assert_eq!(release_path("esk-0.1.1")?, "download/esk-0.1.1");
        assert!(release_path("esk-1/../../other").is_err());
        assert!(release_path("installer-v0.1").is_err());
        Ok(())
    }

    #[test]
    fn archive_stages_only_selected_members() -> Result<()> {
        let dir = Temp::new()?;
        let result = (|| {
            fs::write(dir.0.join(PICKER.0), b"picker")?;
            fs::write(dir.0.join(SHELL.0), b"shell")?;
            run(Command::new("tar").current_dir(&dir.0).args([
                "-czf",
                EFI_ARCHIVE,
                PICKER.0,
                SHELL.0,
            ]))?;
            fs::remove_file(dir.0.join(PICKER.0))?;
            fs::remove_file(dir.0.join(SHELL.0))?;
            stage_archive(&dir.0, &[PICKER])?;
            assert_eq!(fs::read(dir.0.join(PICKER.0))?, b"picker");
            assert!(!dir.0.join(SHELL.0).exists());
            Ok(())
        })();
        for name in [PICKER.0, SHELL.0, EFI_ARCHIVE] {
            let _ = fs::remove_file(dir.0.join(name));
        }
        result
    }
}
