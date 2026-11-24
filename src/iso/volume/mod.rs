use alloc::boxed::Box;
use core::{error::Error, fmt};
use ext4_view::{Ext4, Ext4Read};
use uefi::boot::{self, OpenProtocolAttributes, OpenProtocolParams, ScopedProtocol};
use uefi::proto::ProtocolPointer;
use uefi::proto::device_path::{DevicePath, DeviceSubType, DeviceType};
use uefi::proto::media::{block::BlockIO, disk::DiskIo};
use uefi::{Handle, Status};

struct Partition {
    disk: ScopedProtocol<DiskIo>,
    media_id: u32,
}

#[derive(Debug)]
struct ReadError(Status);

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UEFI disk read failed: {:?}", self.0)
    }
}

impl Error for ReadError {}

impl Ext4Read for Partition {
    fn read(&mut self, offset: u64, bytes: &mut [u8]) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.disk
            .read_disk(self.media_id, offset, bytes)
            .map_err(|error| Box::new(ReadError(error.status())) as Box<dyn Error + Send + Sync>)
    }
}

pub fn open(esp: Handle) -> Result<Ext4, Status> {
    let esp_path = open_readonly::<DevicePath>(esp).map_err(|error| error.status())?;
    let disk_nodes = esp_path
        .node_iter()
        .position(|node| {
            node.device_type() == DeviceType::MEDIA
                && node.sub_type() == DeviceSubType::MEDIA_HARD_DRIVE
        })
        .ok_or(Status::UNSUPPORTED)?;

    let mut found = None;
    for handle in boot::find_handles::<DiskIo>().map_err(|error| error.status())? {
        if handle == esp {
            continue;
        }
        let Ok(path) = open_readonly::<DevicePath>(handle) else {
            continue;
        };
        let mut candidate = path.node_iter();
        if !candidate
            .by_ref()
            .take(disk_nodes)
            .eq(esp_path.node_iter().take(disk_nodes))
            || !candidate.next().is_some_and(|node| {
                node.device_type() == DeviceType::MEDIA
                    && node.sub_type() == DeviceSubType::MEDIA_HARD_DRIVE
            })
            || candidate.next().is_some()
        {
            continue;
        }

        let blocks = open_readonly::<BlockIO>(handle).map_err(|error| error.status())?;
        let media_id = blocks.media().media_id();
        drop(blocks);

        let disk = open_readonly::<DiskIo>(handle).map_err(|error| error.status())?;
        let partition = Partition { disk, media_id };
        if let Ok(fs) = Ext4::load(Box::new(partition)) {
            if fs.label().to_str() == Ok("ISOS") {
                if found.is_some() {
                    return Err(Status::NO_MAPPING);
                }
                found = Some(fs);
            }
        }
    }

    found.ok_or(Status::NOT_FOUND)
}

fn open_readonly<P: ProtocolPointer + ?Sized>(handle: Handle) -> uefi::Result<ScopedProtocol<P>> {
    // SAFETY: This module uses read-only protocol methods and keeps the USB
    // drivers connected until all scoped protocol references have been dropped.
    unsafe {
        boot::open_protocol(
            OpenProtocolParams {
                handle,
                agent: boot::image_handle(),
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )
    }
}
