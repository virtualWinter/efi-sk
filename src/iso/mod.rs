mod arch;
pub mod artwork;
mod debian;
pub mod discovery;
mod fedora;
mod filesystem;
mod generic;
mod linux;
mod os_release;
mod ubuntu;
mod volume;

use alloc::{string::String, vec::Vec};
use ext4_view::Ext4;
use uefi::{Handle, Status};

use artwork::Metadata;
use filesystem::Iso;

#[derive(Clone, Copy)]
pub enum Profile {
    Arch,
    Ubuntu,
    Debian,
    Fedora,
    /// An image no other profile recognises, booted through a known layout.
    Generic,
}

impl Profile {
    /// Every supported profile, in detection order.
    const ALL: [Profile; 5] = [
        Self::Arch,
        Self::Ubuntu,
        Self::Debian,
        Self::Fedora,
        Self::Generic,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Arch => arch::NAME,
            Self::Ubuntu => ubuntu::NAME,
            Self::Debian => debian::NAME,
            Self::Fedora => fedora::NAME,
            Self::Generic => "Linux",
        }
    }

    fn detect(self, iso: &mut Iso, partition: &Ext4, path: &str) -> bool {
        match self {
            Self::Arch => arch::detect(iso, partition, path),
            Self::Ubuntu => ubuntu::detect(iso, partition, path),
            Self::Debian => debian::detect(iso, partition, path),
            Self::Fedora => fedora::detect(iso, partition, path),
            Self::Generic => generic::detect(iso, partition, path),
        }
    }

    /// Loads the kernel and initrd and builds the kernel command line.
    fn boot(
        self,
        iso: &mut Iso,
        partition: &Ext4,
        path: &str,
    ) -> Result<(Vec<u8>, Vec<u8>, String), Status> {
        match self {
            Self::Arch => arch::boot(iso, partition, path),
            Self::Ubuntu => ubuntu::boot(iso, partition, path),
            Self::Debian => debian::boot(iso, partition, path),
            Self::Fedora => fedora::boot(iso, partition, path),
            Self::Generic => generic::boot(iso, partition, path),
        }
    }

    /// Whether the entry needs to read the ISO's metadata: Arch and the
    /// generic fallback supply a release name, the other profiles only artwork.
    fn needs_metadata(self, need_icons: bool) -> bool {
        match self {
            Self::Arch | Self::Generic => true,
            _ => need_icons,
        }
    }

    fn metadata(self, partition: &Ext4, path: &str, need_icon: bool) -> Option<Metadata> {
        match self {
            Self::Arch => arch::metadata(partition, path, need_icon),
            Self::Ubuntu => ubuntu::metadata(partition, path, need_icon),
            Self::Debian => debian::metadata(partition, path, need_icon),
            Self::Fedora => fedora::metadata(partition, path, need_icon),
            Self::Generic => generic::metadata(partition, path, need_icon),
        }
    }
}

pub fn launch(parent: Handle, esp: Handle, profile: Profile, path: &str) -> Result<(), Status> {
    let partition = volume::open(esp)?;
    let file = partition.open(path).map_err(|error| match error {
        ext4_view::Ext4Error::NotFound => Status::NOT_FOUND,
        _ => Status::DEVICE_ERROR,
    })?;
    let mut image = Iso::open(file)?;

    let (kernel, initrd, args) = profile.boot(&mut image, &partition, path)?;
    drop(image);
    drop(partition);
    linux::launch(parent, kernel, initrd, &args)
}
