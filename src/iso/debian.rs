use alloc::{boxed::Box, format, string::String, vec, vec::Vec};
use ext4_view::Ext4;
use uefi::Status;

use super::{
    artwork::{Icon, Metadata, jpeg, png},
    filesystem::Iso,
};

pub const NAME: &str = "Debian Installer";
pub const KERNEL: &str = "install.amd/vmlinuz";
pub const INITRD: &str = "install.amd/initrd.gz";

/// Debian's `hd-media` boot uses kernel and initrd files beside the ISO
/// instead of loading them from the image itself.
const MAX_COMPANION: u64 = 256 * 1024 * 1024;

pub fn detect(iso: &mut Iso, partition: &Ext4, path: &str) -> bool {
    iso.has_boot_file(KERNEL) && iso.has_boot_file(INITRD) && companions(partition, path)
}

pub fn boot(
    iso: &mut Iso,
    partition: &Ext4,
    path: &str,
) -> Result<(Vec<u8>, Vec<u8>, String), Status> {
    if !iso.has_boot_file(KERNEL) || !iso.has_boot_file(INITRD) {
        return Err(Status::UNSUPPORTED);
    }
    let [kernel, initrd] = companion_paths(path);
    Ok((
        read_companion(partition, &kernel)?,
        read_companion(partition, &initrd)?,
        String::new(),
    ))
}

pub fn metadata(partition: &Ext4, path: &str, need_icon: bool) -> Option<Metadata> {
    need_icon.then(|| Metadata {
        label: None,
        icon: icon(partition, path),
    })
}

fn companion_paths(path: &str) -> [String; 2] {
    [format!("{path}.vmlinuz"), format!("{path}.initrd.gz")]
}

fn companion_size(partition: &Ext4, path: &str) -> Option<u64> {
    let file = partition.open(path).ok()?;
    let size = file.metadata().len();
    (1..=MAX_COMPANION).contains(&size).then_some(size)
}

fn companions(partition: &Ext4, path: &str) -> bool {
    companion_paths(path)
        .iter()
        .all(|file| companion_size(partition, file).is_some())
}

fn read_companion(partition: &Ext4, path: &str) -> Result<Vec<u8>, Status> {
    let size = companion_size(partition, path).ok_or(Status::NOT_FOUND)? as usize;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| Status::OUT_OF_RESOURCES)?;
    bytes.resize(size, 0);
    let mut file = partition.open(path).map_err(|_| Status::DEVICE_ERROR)?;
    let mut remaining = bytes.as_mut_slice();
    while !remaining.is_empty() {
        let count = file
            .read_bytes(remaining)
            .map_err(|_| Status::DEVICE_ERROR)?;
        if count == 0 {
            return Err(Status::END_OF_FILE);
        }
        remaining = &mut remaining[count..];
    }
    Ok(bytes)
}

fn icon(partition: &Ext4, path: &str) -> Option<Box<Icon>> {
    let mut iso = Iso::open(partition.open(path).ok()?).ok()?;
    read_logo(&mut iso, "pics/logo-50.jpg")
        .and_then(|bytes| jpeg::decode_debian_logo(&bytes))
        .or_else(|| png::decode_debian_logo(&read_logo(&mut iso, "pics/openlogo-nd-50.png")?))
}

fn read_logo(iso: &mut Iso, path: &str) -> Option<Vec<u8>> {
    let mut logo = iso.open_file(path).ok()?;
    if !(1..=128 * 1024).contains(&logo.len()) {
        return None;
    }
    let mut bytes = vec![0; logo.len() as usize];
    logo.read_at(0, &mut bytes).ok()?;
    Some(bytes)
}
