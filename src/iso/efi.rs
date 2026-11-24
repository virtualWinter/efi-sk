//! Experimental firmware-only El Torito boot via EFI_RAM_DISK_PROTOCOL.
use alloc::vec::Vec;
use core::{mem, ptr};
use uefi::{
    Handle, Status, boot,
    proto::{
        device_path::{DevicePath, FfiDevicePath},
        media::fs::SimpleFileSystem,
        unsafe_protocol,
    },
};

use super::filesystem::Iso;

const SECTOR: u64 = 2048;
const MAX_IMAGE: u64 = 2 * 1024 * 1024 * 1024;
const VIRTUAL_CD: uefi::Guid = uefi::guid!("3d5abd30-4175-87ce-6d64-d2ade523c4bb");

pub fn detect(iso: &mut Iso) -> bool {
    let len = iso.image_len();
    if len > MAX_IMAGE || !len.is_multiple_of(SECTOR) {
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
                && efi_catalog(&sector);
        }
    }
    false
}

fn efi_catalog(catalog: &[u8; SECTOR as usize]) -> bool {
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
    if entries
        .next()
        .is_some_and(|entry| validation[1] == 0xef && entry[0] == 0x88)
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
            if header[1] == 0xef && entry[0] == 0x88 {
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
#[unsafe_protocol("ab38a0df-6873-44a9-87e6-d4eb56148449")]
struct RamDisk {
    register: unsafe extern "efiapi" fn(
        u64,
        u64,
        *const uefi::Guid,
        *const FfiDevicePath,
        *mut *const FfiDevicePath,
    ) -> Status,
    unregister: unsafe extern "efiapi" fn(*const FfiDevicePath) -> Status,
}

pub fn launch(parent: Handle, mut iso: Iso) -> Result<(), Status> {
    if !detect(&mut iso) {
        return Err(Status::UNSUPPORTED);
    }
    let handle = boot::get_handle_for_protocol::<RamDisk>().map_err(|e| e.status())?;
    let protocol = boot::open_protocol_exclusive::<RamDisk>(handle).map_err(|e| e.status())?;
    let len = iso.image_len() as usize;
    let mut image = Vec::new();
    image
        .try_reserve_exact(len)
        .map_err(|_| Status::OUT_OF_RESOURCES)?;
    image.resize(len, 0);
    iso.read_image(0, &mut image)?;
    drop(iso);

    let mut path = ptr::null();
    // The buffer is never resized after registration and outlives all EFI I/O.
    let status = unsafe {
        (protocol.register)(
            image.as_mut_ptr() as u64,
            len as u64,
            &VIRTUAL_CD,
            ptr::null(),
            &mut path,
        )
    };
    if status.is_error() {
        return Err(status);
    }
    let result = if path.is_null() {
        Err(Status::DEVICE_ERROR)
    } else {
        // The protocol owns this device path until unregister returns.
        launch_registered(parent, unsafe { DevicePath::from_ffi_ptr(path) })
    };
    if path.is_null() || unsafe { (protocol.unregister)(path) }.is_error() {
        // Firmware may still refer to the backing memory.
        mem::forget(image);
    }
    result
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
