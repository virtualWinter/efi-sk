use alloc::vec::Vec;
use uefi::Status;

pub(super) fn append(initrd: &mut Vec<u8>, files: &[(&[u8], &[u8], u32)]) -> Result<(), Status> {
    let reserve = files
        .iter()
        .try_fold(128usize, |size, (name, data, _)| {
            size.checked_add(120)?
                .checked_add(name.len())?
                .checked_add(data.len())
        })
        .ok_or(Status::BAD_BUFFER_SIZE)?;
    initrd
        .try_reserve_exact(reserve)
        .map_err(|_| Status::OUT_OF_RESOURCES)?;
    pad(initrd);
    for (index, (name, data, mode)) in files.iter().enumerate() {
        let len = u32::try_from(data.len()).map_err(|_| Status::BAD_BUFFER_SIZE)?;
        header(initrd, name, len, *mode, index as u32 + 1);
        initrd.extend_from_slice(data);
        pad(initrd);
    }
    header(initrd, b"TRAILER!!!", 0, 0, 0);
    Ok(())
}

fn header(out: &mut Vec<u8>, name: &[u8], size: u32, mode: u32, inode: u32) {
    out.extend_from_slice(b"070701");
    for value in [
        inode,
        mode,
        0,
        0,
        1,
        0,
        size,
        0,
        0,
        0,
        0,
        name.len() as u32 + 1,
        0,
    ] {
        for shift in (0..8).rev() {
            out.push(b"0123456789abcdef"[((value >> (shift * 4)) & 0xf) as usize]);
        }
    }
    out.extend_from_slice(name);
    out.push(0);
    pad(out);
}

fn pad(out: &mut Vec<u8>) {
    out.resize(out.len().next_multiple_of(4), 0);
}
