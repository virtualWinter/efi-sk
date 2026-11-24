use alloc::{boxed::Box, format, string::String, vec::Vec};
use ext4_view::Ext4;
use uefi::Status;

use super::{
    artwork::{Icon, Metadata, png, squashfs::SquashFs},
    filesystem::Iso,
};

pub const NAME: &str = "Ubuntu";
pub const KERNEL: &str = "casper/vmlinuz";
pub const INITRD: &str = "casper/initrd";

pub fn detect(iso: &mut Iso, _partition: &Ext4, _path: &str) -> bool {
    iso.has_boot_file(KERNEL) && iso.has_boot_file(INITRD)
}

pub fn boot(
    iso: &mut Iso,
    _partition: &Ext4,
    path: &str,
) -> Result<(Vec<u8>, Vec<u8>, String), Status> {
    let kernel = iso.read_boot_file(KERNEL)?;
    let initrd = iso.read_boot_file(INITRD)?;
    Ok((
        kernel,
        initrd,
        format!("boot=casper iso-scan/filename={path}"),
    ))
}

pub fn metadata(partition: &Ext4, path: &str, need_icon: bool) -> Option<Metadata> {
    need_icon.then(|| Metadata {
        label: None,
        icon: icon(partition, path),
    })
}

/// SquashFS images inside `casper/`, largest first: the live system's root
/// filesystem carries the Ubuntu artwork.
const LOGOS: [&str; 3] = [
    "usr/share/pixmaps/ubuntu-logo.png",
    "usr/share/pixmaps/ubuntu-logo-text-dark.png",
    "usr/share/pixmaps/ubuntu-logo-text.png",
];

fn icon(partition: &Ext4, path: &str) -> Option<Box<Icon>> {
    let mut iso = Iso::open(partition.open(path).ok()?).ok()?;
    let mut images = iso.read_dir("casper").ok()?;
    images.retain(|(name, _)| name.to_ascii_lowercase().ends_with(".squashfs"));
    images.sort_by_key(|(_, size)| core::cmp::Reverse(*size));

    for (name, _) in images {
        let image = format!("casper/{name}");
        let Ok(file) = iso.open_file(&image) else {
            continue;
        };
        let Some(mut root) = SquashFs::open(file) else {
            continue;
        };
        for logo in LOGOS {
            let Some(bytes) = root.file(logo, 128 * 1024) else {
                continue;
            };
            if let Some(icon) = png::decode_ubuntu_logo(&bytes) {
                return Some(icon);
            }
        }
    }
    None
}
