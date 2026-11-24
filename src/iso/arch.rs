use alloc::{format, string::String, vec::Vec};
use ext4_view::Ext4;
use uefi::Status;

use super::{
    artwork::{self, Metadata},
    filesystem::Iso,
    os_release,
};

pub const NAME: &str = "Arch Linux";
pub const KERNEL: &str = "arch/boot/x86_64/vmlinuz-linux";
pub const INITRD: &str = "arch/boot/x86_64/initramfs-linux.img";

pub fn detect(iso: &mut Iso, _partition: &Ext4, _path: &str) -> bool {
    iso.has_boot_file(KERNEL) && iso.has_boot_file(INITRD)
}

pub fn boot(
    iso: &mut Iso,
    partition: &Ext4,
    path: &str,
) -> Result<(Vec<u8>, Vec<u8>, String), Status> {
    let kernel = iso.read_boot_file(KERNEL)?;
    let initrd = iso.read_boot_file(INITRD)?;
    let args = format!(
        "archisobasedir=arch img_dev=/dev/disk/by-uuid/{} img_loop={path}",
        partition.uuid()
    );
    Ok((kernel, initrd, args))
}

/// Reads the entry label and icon from `os-release` inside the live system.
pub fn metadata(partition: &Ext4, path: &str, need_icon: bool) -> Option<Metadata> {
    let mut iso = Iso::open(partition.open(path).ok()?).ok()?;
    let mut root =
        artwork::squashfs::SquashFs::open(iso.open_file("arch/x86_64/airootfs.sfs").ok()?)?;
    let os_release = root.file("usr/lib/os-release", 16 * 1024)?;
    let release = core::str::from_utf8(&os_release).ok()?;

    let label = os_release::pretty_name(release);
    let icon = if need_icon {
        os_release::logo(release)
            .and_then(|name| root.file(&format!("usr/share/pixmaps/{name}.png"), 128 * 1024))
            .and_then(|logo| artwork::png::decode(&logo))
    } else {
        None
    };
    Some(Metadata { label, icon })
}
