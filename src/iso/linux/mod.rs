mod archive;
mod initrd;

use alloc::vec::Vec;
use core::mem::size_of_val;
use uefi::boot::{self, LoadImageSource};
use uefi::proto::loaded_image::LoadedImage;
use uefi::{CString16, Handle, Status};

pub(super) fn append_reopen(
    initrd: &mut Vec<u8>,
    uuid: &str,
    path: &str,
    extra: &[(&[u8], &[u8], u32)],
) -> Result<(), Status> {
    let mut files: Vec<(&[u8], &[u8], u32)> = alloc::vec![
        (b"efi-sk-reopen", include_bytes!("reopen.sh"), 0o100755),
        (b"efi-sk-source-uuid", uuid.as_bytes(), 0o100644),
        (b"efi-sk-source-path", path.as_bytes(), 0o100644),
    ];
    files.extend_from_slice(extra);
    archive::append(initrd, &files)
}

pub fn launch(
    parent: Handle,
    kernel: Vec<u8>,
    initrd: Vec<u8>,
    command: &str,
) -> Result<(), Status> {
    let options = CString16::try_from(command).map_err(|_| Status::INVALID_PARAMETER)?;
    let _initrd = initrd::Registration::install(initrd)?;
    let image = boot::load_image(
        parent,
        LoadImageSource::FromBuffer {
            buffer: &kernel,
            file_path: None,
        },
    )
    .map_err(|error| error.status())?;
    drop(kernel);

    let image_info = boot::open_protocol_exclusive::<LoadedImage>(image);
    let mut image_info = match image_info {
        Ok(info) => info,
        Err(error) => {
            let _ = boot::unload_image(image);
            return Err(error.status());
        }
    };
    // The load option allocation stays live until StartImage returns.
    unsafe {
        image_info.set_load_options(
            options.as_ptr().cast(),
            size_of_val(options.as_slice_with_nul()) as u32,
        );
    }
    drop(image_info);
    boot::start_image(image).map_err(|error| error.status())
}
