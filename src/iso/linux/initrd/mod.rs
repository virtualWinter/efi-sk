use alloc::{boxed::Box, vec::Vec};
use core::{cell::RefCell, ffi::c_void, mem, ptr, slice};
use uefi::boot;
use uefi::proto::device_path::build::{DevicePathBuilder, media::Vendor};
use uefi::{Handle, Status, guid};
use uefi_raw::Boolean;
use uefi_raw::protocol::{device_path::DevicePathProtocol, media::LoadFile2Protocol};

use crate::iso::filesystem::Iso;

pub struct Payload {
    pub prefix: Vec<u8>,
    pub media: Option<(Iso, Vec<u8>)>,
}

#[repr(C)]
struct Initrd {
    // The firmware passes this first field to the callback.
    protocol: LoadFile2Protocol,
    data: RefCell<Payload>,
}

pub struct Registration {
    handle: Handle,
    initrd: Option<Box<Initrd>>,
    device_path: Option<Vec<u8>>,
}

impl Registration {
    pub fn install(data: Payload) -> Result<Self, Status> {
        let mut device_path = Vec::new();
        DevicePathBuilder::with_vec(&mut device_path)
            .push(&Vendor {
                vendor_guid: guid!("5568e427-68fc-4f3d-ac74-ca555231cc68"),
                vendor_defined_data: &[],
            })
            .and_then(DevicePathBuilder::finalize)
            .map_err(|_| Status::INVALID_PARAMETER)?;

        let initrd = Box::new(Initrd {
            protocol: LoadFile2Protocol { load_file },
            data: RefCell::new(data),
        });
        // Both interfaces reside in owned allocations with stable addresses.
        // Registration removes them before releasing the allocations.
        let handle = unsafe {
            boot::install_protocol_interface(
                None,
                &LoadFile2Protocol::GUID,
                ptr::from_ref(&initrd.protocol).cast(),
            )
        }
        .map_err(|error| error.status())?;
        let mut registration = Self {
            handle,
            initrd: Some(initrd),
            device_path: None,
        };
        unsafe {
            boot::install_protocol_interface(
                Some(handle),
                &DevicePathProtocol::GUID,
                device_path.as_ptr().cast(),
            )
        }
        .map_err(|error| error.status())?;
        registration.device_path = Some(device_path);
        Ok(registration)
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        if let Some(path) = self.device_path.take() {
            // Keep the allocation if firmware could not remove its reference.
            if unsafe {
                boot::uninstall_protocol_interface(
                    self.handle,
                    &DevicePathProtocol::GUID,
                    path.as_ptr().cast(),
                )
            }
            .is_err()
            {
                mem::forget(path);
            }
        }
        if let Some(initrd) = self.initrd.take() {
            if unsafe {
                boot::uninstall_protocol_interface(
                    self.handle,
                    &LoadFile2Protocol::GUID,
                    ptr::from_ref(&initrd.protocol).cast(),
                )
            }
            .is_err()
            {
                mem::forget(initrd);
            }
        }
    }
}

unsafe extern "efiapi" fn load_file(
    this: *mut LoadFile2Protocol,
    path: *const DevicePathProtocol,
    boot_policy: Boolean,
    size: *mut usize,
    buffer: *mut c_void,
) -> Status {
    if this.is_null() || size.is_null() || path.is_null() {
        return Status::INVALID_PARAMETER;
    }
    if bool::from(boot_policy) {
        return Status::UNSUPPORTED;
    }
    // SAFETY: UEFI supplies the registered interface and a writable size pointer.
    let initrd = unsafe { &*this.cast::<Initrd>() };
    let Ok(mut data) = initrd.data.try_borrow_mut() else {
        return Status::NOT_READY;
    };
    let media_len = data
        .media
        .as_ref()
        .map_or(0, |(iso, _)| iso.image_len() as usize);
    let suffix_len = data.media.as_ref().map_or(0, |(_, suffix)| suffix.len());
    let prefix_len = data.prefix.len();
    let Some(required) = prefix_len
        .checked_add(media_len)
        .and_then(|len| len.checked_add(suffix_len))
    else {
        return Status::BAD_BUFFER_SIZE;
    };
    let provided = unsafe { *size };
    unsafe { *size = required };
    if buffer.is_null() || provided < required {
        return Status::BUFFER_TOO_SMALL;
    }
    // SAFETY: Firmware has supplied a separate destination of at least the required size.
    let target = buffer.cast::<u8>();
    unsafe {
        ptr::copy_nonoverlapping(data.prefix.as_ptr(), target, prefix_len);
    }
    if let Some((iso, suffix)) = &mut data.media {
        for offset in (0..media_len).step_by(1024 * 1024) {
            let count = (media_len - offset).min(1024 * 1024);
            // Initialize memory before passing it to the safe ext4 reader.
            let chunk = unsafe {
                let dst = target.add(prefix_len + offset);
                dst.write_bytes(0, count);
                slice::from_raw_parts_mut(dst, count)
            };
            if let Err(status) = iso.read_image(offset as u64, chunk) {
                return status;
            }
        }
        unsafe {
            ptr::copy_nonoverlapping(
                suffix.as_ptr(),
                target.add(prefix_len + media_len),
                suffix_len,
            );
        }
    }
    Status::SUCCESS
}
