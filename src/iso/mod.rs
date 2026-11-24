pub mod artwork;
mod bsd;
mod debian;
pub mod discovery;
mod efi;
mod filesystem;
mod linux;
mod proxmox;
mod volume;

use alloc::{format, string::String};
use ext4_view::{Ext4, Uuid};
use uefi::{Handle, Status};

use filesystem::Iso;

struct LinuxLayout {
    kernel: &'static str,
    initrd: &'static str,
    marker: Option<&'static str>,
    args: fn(&str, Uuid) -> String,
}

const LAYOUTS: &[LinuxLayout] = &[
    LinuxLayout {
        kernel: "arch/boot/x86_64/vmlinuz-linux",
        initrd: "arch/boot/x86_64/initramfs-linux.img",
        marker: None,
        args: |path, uuid| {
            format!("archisobasedir=arch img_dev=/dev/disk/by-uuid/{uuid} img_loop={path}")
        },
    },
    LinuxLayout {
        kernel: "casper/vmlinuz",
        initrd: "casper/initrd",
        marker: None,
        args: |path, _| format!("boot=casper iso-scan/filename={path}"),
    },
    LinuxLayout {
        kernel: "images/pxeboot/vmlinuz",
        initrd: "images/pxeboot/initrd.img",
        marker: Some("images/install.img"),
        args: |path, uuid| format!("inst.stage2=hd:UUID={uuid}:{path}"),
    },
    LinuxLayout {
        kernel: "live/vmlinuz",
        initrd: "live/initrd.img",
        marker: None,
        args: |path, _| format!("boot=live findiso={path}"),
    },
];

enum Boot {
    Linux(&'static LinuxLayout),
    VersionedLive(String, String),
    DebianInstaller,
    ProxmoxInstaller,
    Efi,
}

fn detect(image: &mut Iso, partition: &Ext4, path: &str) -> Option<Boot> {
    if debian::detect(image, partition, path) {
        return Some(Boot::DebianInstaller);
    }
    if proxmox::detect(image) {
        return Some(Boot::ProxmoxInstaller);
    }
    if let Some(layout) = LAYOUTS.iter().find(|layout| {
        image.has_boot_file(layout.kernel)
            && image.has_boot_file(layout.initrd)
            && layout
                .marker
                .is_none_or(|marker| image.open_file(marker).is_ok())
    }) {
        return Some(Boot::Linux(layout));
    }
    if let Some([kernel, initrd]) = versioned_live(image) {
        return Some(Boot::VersionedLive(kernel, initrd));
    }
    efi::detect(image).then_some(Boot::Efi)
}

fn versioned_live(image: &mut Iso) -> Option<[String; 2]> {
    image.open_file("live/filesystem.squashfs").ok()?;
    let mut entries = image.read_dir("live").ok()?;
    entries.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    for (name, _) in entries {
        let Some(version) = name.strip_prefix("vmlinuz-") else {
            continue;
        };
        if version.is_empty() {
            continue;
        }
        let kernel = format!("live/{name}");
        let initrd = format!("live/initrd.img-{version}");
        if image.has_boot_file(&kernel) && image.has_boot_file(&initrd) {
            return Some([kernel, initrd]);
        }
    }
    None
}

pub fn launch(parent: Handle, esp: Handle, path: &str) -> Result<(), Status> {
    let (partition, start, length) = volume::open_partition(esp)?;
    let file = partition.open(path).map_err(|error| match error {
        ext4_view::Ext4Error::NotFound => Status::NOT_FOUND,
        _ => Status::DEVICE_ERROR,
    })?;
    let mut image = Iso::open(file)?;
    let (kernel, initrd, args) = match detect(&mut image, &partition, path)
        .ok_or(Status::UNSUPPORTED)?
    {
        Boot::Linux(layout) => (
            image.read_boot_file(layout.kernel)?,
            image.read_boot_file(layout.initrd)?,
            (layout.args)(path, partition.uuid()),
        ),
        Boot::VersionedLive(kernel, initrd) => {
            let options = if kernel.ends_with("-vyos") {
                " hostname=vyos username=live nopersistence noautologin nonetworking union=overlay console=ttyS0,115200 console=tty0 net.ifnames=0 biosdevname=0"
            } else {
                ""
            };
            (
                image.read_boot_file(&kernel)?,
                image.read_boot_file(&initrd)?,
                format!("boot=live components findiso={path}{options}"),
            )
        }
        Boot::DebianInstaller => debian::boot(&mut image, &partition, path)?,
        Boot::ProxmoxInstaller => proxmox::boot(&mut image, &partition, path)?,
        Boot::Efi => return efi::launch(parent, esp, image, (start, length)),
    };
    drop(image);
    drop(partition);
    linux::launch(parent, kernel, initrd, &args)
}
