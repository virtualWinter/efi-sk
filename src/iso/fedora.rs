use alloc::{boxed::Box, format, string::String, vec::Vec};
use ext4_view::Ext4;
use uefi::Status;

use super::{
    artwork::{Icon, Metadata, png, squashfs::SquashFs},
    filesystem::Iso,
};

pub const NAME: &str = "Fedora Installer";
pub const KERNEL: &str = "images/pxeboot/vmlinuz";
pub const INITRD: &str = "images/pxeboot/initrd.img";

/// Anaconda's stage2 image: the root filesystem of the installer.
const STAGE2: &str = "images/install.img";

pub fn detect(iso: &mut Iso, _partition: &Ext4, _path: &str) -> bool {
    iso.has_boot_file(KERNEL) && iso.has_boot_file(INITRD) && iso.open_file(STAGE2).is_ok()
}

pub fn boot(
    iso: &mut Iso,
    partition: &Ext4,
    path: &str,
) -> Result<(Vec<u8>, Vec<u8>, String), Status> {
    let kernel = iso.read_boot_file(KERNEL)?;
    let initrd = iso.read_boot_file(INITRD)?;
    let args = format!("inst.stage2=hd:UUID={}:{}", partition.uuid(), path);
    Ok((kernel, initrd, args))
}

pub fn metadata(partition: &Ext4, path: &str, need_icon: bool) -> Option<Metadata> {
    need_icon.then(|| Metadata {
        label: None,
        icon: icon(partition, path),
    })
}

/// The installer image carries the logo in the hicolor icon theme.
const LOGOS: [&str; 3] = [
    "usr/share/icons/hicolor/96x96/apps/fedora-logo-icon.png",
    "usr/share/icons/hicolor/48x48/apps/fedora-logo-icon.png",
    "usr/share/icons/hicolor/32x32/apps/fedora-logo-icon.png",
];

fn icon(partition: &Ext4, path: &str) -> Option<Box<Icon>> {
    let mut iso = Iso::open(partition.open(path).ok()?).ok()?;
    let mut root = SquashFs::open(iso.open_file(STAGE2).ok()?)?;
    for logo in LOGOS {
        let Some(bytes) = root.file(logo, 128 * 1024) else {
            continue;
        };
        if let Some(icon) = png::decode(&bytes) {
            return Some(icon);
        }
    }
    None
}
