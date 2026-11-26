use alloc::boxed::Box;
use core::{error::Error, fmt};
use ext4_view::{Ext4, Ext4Read};
use uefi::boot::{self, OpenProtocolAttributes, OpenProtocolParams, ScopedProtocol};
use uefi::proto::ProtocolPointer;
use uefi::proto::device_path::media::HardDrive;
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
    open_partition(esp).map(|(fs, _, _)| fs)
}

pub(super) fn open_partition(esp: Handle) -> Result<(Ext4, u64, u64), Status> {
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
        {
            continue;
        }
        let Some(partition_node) = candidate
            .next()
            .and_then(|node| <&HardDrive>::try_from(node).ok())
        else {
            continue;
        };
        if candidate.next().is_some() {
            continue;
        }

        let blocks = open_readonly::<BlockIO>(handle).map_err(|error| error.status())?;
        let media_id = blocks.media().media_id();
        let sector = u64::from(blocks.media().block_size());
        drop(blocks);

        let disk = open_readonly::<DiskIo>(handle).map_err(|error| error.status())?;
        let partition = Partition { disk, media_id };
        if let Ok(fs) = Ext4::load(Box::new(partition))
            && fs.label().to_str() == Ok("ISOS")
        {
            if found.is_some() {
                return Err(Status::NO_MAPPING);
            }
            let start = partition_node
                .partition_start()
                .checked_mul(sector)
                .ok_or(Status::BAD_BUFFER_SIZE)?;
            let size = partition_node
                .partition_size()
                .checked_mul(sector)
                .ok_or(Status::BAD_BUFFER_SIZE)?;
            found = Some((fs, start, size));
        }
    }

    found.ok_or(Status::NOT_FOUND)
}

pub(super) const ZERO_START: u64 = 128 * 1024;
pub(super) const ZERO_SIZE: u64 = 512 * 1024;

pub(super) fn disk_identity(esp: Handle) -> Result<(u64, [u8; 16]), Status> {
    let esp_path = open_readonly::<DevicePath>(esp).map_err(|e| e.status())?;
    let nodes = esp_path
        .node_iter()
        .position(|node| {
            node.device_type() == DeviceType::MEDIA
                && node.sub_type() == DeviceSubType::MEDIA_HARD_DRIVE
        })
        .ok_or(Status::UNSUPPORTED)?;
    for handle in boot::find_handles::<BlockIO>().map_err(|e| e.status())? {
        let Ok(path) = open_readonly::<DevicePath>(handle) else {
            continue;
        };
        if !path.node_iter().eq(esp_path.node_iter().take(nodes)) {
            continue;
        }
        let blocks = open_readonly::<BlockIO>(handle).map_err(|e| e.status())?;
        let media = blocks.media();
        if media.block_size() != 512 {
            return Err(Status::UNSUPPORTED);
        }
        let size = media
            .last_block()
            .checked_add(1)
            .and_then(|count| count.checked_mul(u64::from(media.block_size())))
            .ok_or(Status::BAD_BUFFER_SIZE)?;
        let disk = open_readonly::<DiskIo>(handle).map_err(|e| e.status())?;
        let mut identity = [0; 16];
        disk.read_disk(media.media_id(), 0x180, &mut identity)
            .map_err(|e| e.status())?;
        if !identity.starts_with(b"EFISKMAP") {
            return Err(Status::NO_MAPPING);
        }
        let mut zero = [0; 4096];
        for offset in (ZERO_START..ZERO_START + ZERO_SIZE).step_by(zero.len()) {
            disk.read_disk(media.media_id(), offset, &mut zero)
                .map_err(|e| e.status())?;
            if zero.iter().any(|&byte| byte != 0) {
                return Err(Status::NO_MAPPING);
            }
        }
        return Ok((size, identity));
    }
    Err(Status::NOT_FOUND)
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
