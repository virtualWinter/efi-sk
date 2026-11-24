use alloc::{format, string::String, vec::Vec};
use ext4_view::{Ext4, Uuid};
use uefi::Status;

use super::{
    artwork::{Metadata, png, squashfs::SquashFs},
    filesystem::Iso,
    os_release,
};

/// Where an unrecognised image keeps its kernel and initrd, and what its
/// initramfs has to be told to find the ISO again.
struct Layout {
    kernel: &'static str,
    initrd: &'static str,
    args: fn(&str, &Uuid) -> String,
}

/// Layouts consulted once every known profile has declined an image.
const LAYOUTS: [Layout; 1] = [
    // Debian live and derivatives (Tails, Kali, LMDE, ...).
    Layout {
        kernel: "live/vmlinuz",
        initrd: "live/initrd.img",
        args: |path, _| format!("boot=live findiso={path}"),
    },
];

/// Directories that commonly hold a live system's root filesystem.
const IMAGE_DIRS: [&str; 6] = ["casper", "live", "LiveOS", "arch/x86_64", "images", ""];
const IMAGE_SUFFIXES: [&str; 3] = [".squashfs", ".sfs", ".img"];
/// At most this many images are opened per directory while probing.
const MAX_IMAGES: usize = 4;

pub fn detect(iso: &mut Iso, _partition: &Ext4, _path: &str) -> bool {
    LAYOUTS.iter().any(|layout| has_layout(iso, layout))
}

pub fn boot(
    iso: &mut Iso,
    partition: &Ext4,
    path: &str,
) -> Result<(Vec<u8>, Vec<u8>, String), Status> {
    let layout = LAYOUTS.iter().find(|layout| has_layout(iso, layout));
    let Some(layout) = layout else {
        return Err(Status::UNSUPPORTED);
    };
    let kernel = iso.read_boot_file(layout.kernel)?;
    let initrd = iso.read_boot_file(layout.initrd)?;
    let args = (layout.args)(path, &partition.uuid());
    Ok((kernel, initrd, args))
}

/// Reads the entry label and icon from `os-release` inside the live system.
pub fn metadata(partition: &Ext4, path: &str, need_icon: bool) -> Option<Metadata> {
    let mut iso = Iso::open(partition.open(path).ok()?).ok()?;
    for image in image_candidates(&mut iso) {
        let Ok(file) = iso.open_file(&image) else {
            continue;
        };
        let Some(mut root) = SquashFs::open(file) else {
            continue;
        };
        let release = root
            .file("usr/lib/os-release", 16 * 1024)
            .or_else(|| root.file("etc/os-release", 16 * 1024));
        let Some(release) =
            release.and_then(|bytes| core::str::from_utf8(&bytes).ok().map(String::from))
        else {
            continue;
        };
        let label = os_release::pretty_name(&release);
        let icon = if need_icon {
            os_release::logo(&release).and_then(|logo| {
                root.file(&format!("usr/share/pixmaps/{logo}.png"), 128 * 1024)
                    .and_then(|bytes| png::decode(&bytes))
            })
        } else {
            None
        };
        if label.is_some() || icon.is_some() {
            return Some(Metadata { label, icon });
        }
    }
    None
}

fn has_layout(iso: &mut Iso, layout: &Layout) -> bool {
    iso.has_boot_file(layout.kernel) && iso.has_boot_file(layout.initrd)
}

/// The root filesystem images to probe, largest first per directory.
fn image_candidates(iso: &mut Iso) -> Vec<String> {
    let mut images = Vec::new();
    for dir in IMAGE_DIRS {
        let Ok(mut entries) = iso.read_dir(dir) else {
            continue;
        };
        entries.retain(|(name, size)| {
            let name = name.to_ascii_lowercase();
            *size > 0 && IMAGE_SUFFIXES.iter().any(|suffix| name.ends_with(suffix))
        });
        entries.sort_by_key(|(_, size)| core::cmp::Reverse(*size));
        images.extend(
            entries
                .into_iter()
                .take(MAX_IMAGES)
                .map(|(name, _)| format!("{dir}/{name}")),
        );
    }
    images
}
