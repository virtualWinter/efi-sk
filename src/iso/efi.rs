//! Firmware-side virtual optical media backed by an ISO on the USB.
use alloc::{boxed::Box, vec::Vec};
use core::{cell::RefCell, ffi::c_void, mem, ptr, slice};
use uefi::{
    Handle, Status, boot,
    proto::{
        device_path::{
            DevicePath,
            build::{DevicePathBuilder, media::Vendor},
        },
        media::fs::SimpleFileSystem,
    },
};
use uefi_raw::{
    Boolean,
    protocol::{
        block::{BlockIoMedia, BlockIoProtocol},
        device_path::DevicePathProtocol,
    },
};

use super::filesystem::Iso;

const SECTOR: u64 = 2048;
const READ_WINDOW: usize = 128 * 1024;
const ISO_DEVICE: uefi::Guid = uefi::guid!("bff95fa8-312a-43bf-823e-c862b96c64d5");

pub fn detect(iso: &mut Iso) -> bool {
    let len = iso.image_len();
    if len == 0 || !len.is_multiple_of(SECTOR) {
        return false;
    }
    let mut sector = [0; SECTOR as usize];
    for lba in 16..80 {
        if iso.read_image(lba * SECTOR, &mut sector).is_err() || &sector[1..7] != b"CD001\x01" {
            return false;
        }
        if sector[0] == 255 {
            break;
        }
        if sector[0] == 0 && &sector[7..30] == b"EL TORITO SPECIFICATION" {
            let catalog = u32::from_le_bytes([sector[71], sector[72], sector[73], sector[74]]);
            return iso
                .read_image(u64::from(catalog) * SECTOR, &mut sector)
                .is_ok()
                && efi_catalog(&sector, |lba| fat_boot_image(iso, lba));
        }
    }
    false
}

fn fat_boot_image(iso: &mut Iso, lba: u32) -> bool {
    let mut header = [0; 512];
    iso.read_image(u64::from(lba) * SECTOR, &mut header).is_ok()
        && header[510..512] == [0x55, 0xaa]
        && matches!(header[0], 0xeb | 0xe9)
        && matches!(
            u16::from_le_bytes([header[11], header[12]]),
            512 | 1024 | 2048 | 4096
        )
        && header[13].is_power_of_two()
        && (header[54..62] == *b"FAT12   "
            || header[54..62] == *b"FAT16   "
            || header[82..90] == *b"FAT32   ")
}

fn efi_catalog(catalog: &[u8; SECTOR as usize], mut is_fat: impl FnMut(u32) -> bool) -> bool {
    let mut entries = catalog.as_chunks::<32>().0.iter();
    let Some(validation) = entries.next() else {
        return false;
    };
    if validation[0] != 1
        || validation[30..] != [0x55, 0xaa]
        || validation
            .as_chunks::<2>()
            .0
            .iter()
            .fold(0u16, |sum, word| {
                sum.wrapping_add(u16::from_le_bytes(*word))
            })
            != 0
    {
        return false;
    }
    // Some BSD masters label the EFI image as platform 0 (BIOS).
    // Accept a no-emulation FAT entry as a candidate; the launch path
    // still requires it to contain the x86-64 fallback EFI executable.
    let mut bootable = |platform, entry: &[u8; 32]| {
        entry[0] == 0x88
            && (platform == 0xef
                || (platform == 0
                    && entry[1] == 0
                    && is_fat(u32::from_le_bytes([
                        entry[8], entry[9], entry[10], entry[11],
                    ]))))
    };
    if entries
        .next()
        .is_some_and(|entry| bootable(validation[1], entry))
    {
        return true;
    }
    while let Some(header) = entries.next() {
        if !matches!(header[0], 0x90 | 0x91) {
            break;
        }
        for _ in 0..u16::from_le_bytes([header[2], header[3]]) {
            let Some(entry) = entries.next() else {
                return false;
            };
            if bootable(header[1], entry) {
                return true;
            }
        }
        if header[0] == 0x91 {
            break;
        }
    }
    false
}

#[repr(C)]
struct Optical {
    protocol: BlockIoProtocol,
    media: BlockIoMedia,
    source: RefCell<OpticalSource>,
    overlay: Vec<super::bsd::Overlay>,
    module: Vec<u8>,
}

struct OpticalSource {
    iso: Iso,
    cache: Vec<u8>,
    offset: u64,
    valid: usize,
}

impl OpticalSource {
    fn read(&mut self, start: u64, target: &mut [u8]) -> Result<(), Status> {
        let aligned = start / READ_WINDOW as u64 * READ_WINDOW as u64;
        let index = (start - aligned) as usize;
        if self.cache.is_empty() || target.len() > READ_WINDOW - index {
            return self.iso.read_image(start, target);
        }
        if self.offset != aligned || self.valid < index + target.len() {
            self.valid = 0;
            let count = (self.iso.image_len() - aligned).min(READ_WINDOW as u64) as usize;
            self.iso.read_image(aligned, &mut self.cache[..count])?;
            self.offset = aligned;
            self.valid = count;
        }
        let slice = self.cache[..self.valid]
            .get(index..index + target.len())
            .ok_or(Status::DEVICE_ERROR)?;
        target.copy_from_slice(slice);
        Ok(())
    }
}

pub fn launch(
    parent: Handle,
    esp: Handle,
    mut iso: Iso,
    partition: (u64, u64),
) -> Result<(), Status> {
    if !detect(&mut iso) {
        return Err(Status::UNSUPPORTED);
    }
    let handoff = super::bsd::media_overlay(esp, &mut iso, partition)?;
    let media_len = iso
        .image_len()
        .checked_add(handoff.module.len() as u64)
        .ok_or(Status::BAD_BUFFER_SIZE)?;
    // FreeBSD's loader preloads raw modules in small reads. Read ahead on
    // this path so that USB transfers aren't dominated by per-read overhead.
    let mut cache = Vec::new();
    if !handoff.overlays.is_empty() && cache.try_reserve_exact(READ_WINDOW).is_ok() {
        cache.resize(READ_WINDOW, 0);
    }
    let mut optical = Box::new(Optical {
        protocol: BlockIoProtocol {
            revision: BlockIoProtocol::REVISION,
            media: ptr::null(),
            reset,
            read_blocks,
            write_blocks,
            flush_blocks,
        },
        media: BlockIoMedia {
            media_id: 1,
            removable_media: true.into(),
            media_present: true.into(),
            logical_partition: false.into(),
            read_only: true.into(),
            write_caching: false.into(),
            block_size: SECTOR as u32,
            io_align: 1,
            last_block: media_len / SECTOR - 1,
            lowest_aligned_lba: 0,
            logical_blocks_per_physical_block: 0,
            optimal_transfer_length_granularity: 0,
        },
        source: RefCell::new(OpticalSource {
            iso,
            cache,
            offset: 0,
            valid: 0,
        }),
        overlay: handoff.overlays,
        module: handoff.module,
    });
    optical.protocol.media = ptr::from_ref(&optical.media);
    let mut path = Vec::new();
    let instance = (ptr::from_ref(&*optical) as usize).to_le_bytes();
    DevicePathBuilder::with_vec(&mut path)
        .push(&Vendor {
            vendor_guid: ISO_DEVICE,
            vendor_defined_data: &instance,
        })
        .and_then(DevicePathBuilder::finalize)
        .map_err(|_| Status::INVALID_PARAMETER)?;
    let path_ptr = path.as_ptr().cast();
    let block_ptr = ptr::from_ref(&optical.protocol).cast();
    // Both pointers remain valid until their protocols are uninstalled.
    let handle =
        unsafe { boot::install_protocol_interface(None, &DevicePathProtocol::GUID, path_ptr) }
            .map_err(|e| e.status())?;
    let installed = unsafe {
        boot::install_protocol_interface(Some(handle), &BlockIoProtocol::GUID, block_ptr)
    };
    if let Err(e) = installed {
        if unsafe {
            boot::uninstall_protocol_interface(handle, &DevicePathProtocol::GUID, path_ptr)
        }
        .is_err()
        {
            mem::forget(path);
        }
        return Err(e.status());
    }
    let _ = boot::connect_controller(handle, &[], None, true);
    let result = launch_registered(parent, unsafe { DevicePath::from_ffi_ptr(path_ptr.cast()) });
    let _ = boot::disconnect_controller(handle, None, None);
    if unsafe { boot::uninstall_protocol_interface(handle, &BlockIoProtocol::GUID, block_ptr) }
        .is_err()
    {
        // A driver still owns the virtual device. Keep both allocations alive.
        mem::forget(optical);
        mem::forget(path);
    } else if unsafe {
        boot::uninstall_protocol_interface(handle, &DevicePathProtocol::GUID, path_ptr)
    }
    .is_err()
    {
        mem::forget(path);
    }
    result
}

unsafe extern "efiapi" fn reset(_: *mut BlockIoProtocol, _: Boolean) -> Status {
    Status::SUCCESS
}

unsafe extern "efiapi" fn flush_blocks(_: *mut BlockIoProtocol) -> Status {
    Status::SUCCESS
}

unsafe extern "efiapi" fn write_blocks(
    _: *mut BlockIoProtocol,
    _: u32,
    _: u64,
    _: usize,
    _: *const c_void,
) -> Status {
    Status::WRITE_PROTECTED
}

unsafe extern "efiapi" fn read_blocks(
    this: *const BlockIoProtocol,
    media: u32,
    lba: u64,
    len: usize,
    buffer: *mut c_void,
) -> Status {
    if this.is_null() || (len != 0 && buffer.is_null()) {
        return Status::INVALID_PARAMETER;
    }
    // The protocol is the first field of the registered Optical allocation.
    let optical = unsafe { &*this.cast::<Optical>() };
    if media != optical.media.media_id {
        return Status::MEDIA_CHANGED;
    }
    if !len.is_multiple_of(SECTOR as usize) || len > isize::MAX as usize {
        return Status::BAD_BUFFER_SIZE;
    }
    if len == 0 {
        return Status::SUCCESS;
    }
    let blocks = len as u64 / SECTOR;
    if lba
        .checked_add(blocks - 1)
        .is_none_or(|end| end > optical.media.last_block)
    {
        return Status::INVALID_PARAMETER;
    }
    let Ok(mut source) = optical.source.try_borrow_mut() else {
        return Status::NOT_READY;
    };
    // UEFI supplies a writable buffer. Initialize it before creating a slice.
    let target = unsafe {
        let p = buffer.cast::<u8>();
        p.write_bytes(0, len);
        slice::from_raw_parts_mut(p, len)
    };
    let start = lba * SECTOR;
    let image_len = source.iso.image_len();
    let iso_bytes = image_len.saturating_sub(start).min(len as u64) as usize;
    if iso_bytes > 0
        && let Err(status) = source.read(start, &mut target[..iso_bytes])
    {
        return status;
    }
    if iso_bytes < len {
        let offset = start.saturating_sub(image_len) as usize;
        let Some(data) = optic_module_slice(optical, offset, len - iso_bytes) else {
            return Status::DEVICE_ERROR;
        };
        target[iso_bytes..].copy_from_slice(data);
    }
    for (offset, sector) in &optical.overlay {
        if let Some(index) = offset
            .checked_sub(start)
            .and_then(|i| usize::try_from(i).ok())
            && let Some(dest) = target.get_mut(index..index.saturating_add(SECTOR as usize))
        {
            dest.copy_from_slice(sector);
        }
    }
    Status::SUCCESS
}

fn optic_module_slice(optical: &Optical, offset: usize, size: usize) -> Option<&[u8]> {
    optical.module.get(offset..offset.checked_add(size)?)
}

fn launch_registered(parent: Handle, ram_path: &DevicePath) -> Result<(), Status> {
    let node_count = ram_path.node_iter().count();
    if node_count == 0 {
        return Err(Status::DEVICE_ERROR);
    }
    for handle in boot::find_handles::<SimpleFileSystem>().map_err(|e| e.status())? {
        let matches = boot::open_protocol_exclusive::<DevicePath>(handle)
            .is_ok_and(|path| path.node_iter().take(node_count).eq(ram_path.node_iter()));
        if matches {
            match crate::chainload::launch(parent, handle, "\\EFI\\BOOT\\BOOTX64.EFI") {
                Err(Status::NOT_FOUND) => continue,
                result => return result,
            }
        }
    }
    Err(Status::NOT_FOUND)
}
