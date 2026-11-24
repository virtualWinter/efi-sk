use alloc::vec::Vec;
use uefi::boot::{self, LoadImageSource};
use uefi::proto::BootPolicy;
use uefi::proto::device_path::DevicePath;
use uefi::proto::device_path::build::{DevicePathBuilder, media::FilePath};
use uefi::proto::loaded_image::LoadedImage;
use uefi::{CString16, Handle, Status};

pub fn boot_volume(image: Handle) -> Result<Handle, Status> {
    boot::open_protocol_exclusive::<LoadedImage>(image)
        .map_err(|error| error.status())?
        .device()
        .ok_or(Status::NOT_FOUND)
}

pub fn launch(parent: Handle, device: Handle, path: &str) -> Result<(), Status> {
    launch_with_options(parent, device, path, None)
}

pub fn launch_with_options(
    parent: Handle,
    device: Handle,
    path: &str,
    options: Option<&str>,
) -> Result<(), Status> {
    let options = options
        .map(CString16::try_from)
        .transpose()
        .map_err(|_| Status::INVALID_PARAMETER)?;
    let path = CString16::try_from(path).map_err(|_| Status::INVALID_PARAMETER)?;
    let mut bytes = Vec::new();

    // Release the device-path protocol before transferring control to the child.
    let full_path = {
        let volume =
            boot::open_protocol_exclusive::<DevicePath>(device).map_err(|error| error.status())?;
        let mut builder = DevicePathBuilder::with_vec(&mut bytes);
        for node in volume.node_iter() {
            builder = builder.push(&node).map_err(|_| Status::INVALID_PARAMETER)?;
        }
        builder
            .push(&FilePath { path_name: &path })
            .and_then(DevicePathBuilder::finalize)
            .map_err(|_| Status::INVALID_PARAMETER)?
    };

    let child = boot::load_image(
        parent,
        LoadImageSource::FromDevicePath {
            device_path: full_path,
            boot_policy: BootPolicy::ExactMatch,
        },
    )
    .map_err(|error| error.status())?;
    if let Some(options) = &options {
        let mut loaded = match boot::open_protocol_exclusive::<LoadedImage>(child) {
            Ok(loaded) => loaded,
            Err(error) => {
                let _ = boot::unload_image(child);
                return Err(error.status());
            }
        };
        // The UTF-16 buffer lives until StartImage returns.
        unsafe {
            loaded.set_load_options(
                options.as_ptr().cast(),
                core::mem::size_of_val(options.as_slice_with_nul()) as u32,
            );
        }
    }
    boot::start_image(child).map_err(|error| error.status())
}
