use alloc::{format, string::String, vec::Vec};
use ext4_view::Ext4;
use uefi::Status;

use super::{filesystem::Iso, linux};

const KERNEL: &str = "boot/linux26";
const INITRD: &str = "boot/initrd.img";

pub fn detect(iso: &mut Iso) -> bool {
    if !iso.has_boot_file(KERNEL) || !iso.has_boot_file(INITRD) {
        return false;
    }
    let Ok(mut info) = iso.open_file(".cd-info") else {
        return false;
    };
    let mut bytes = [0; 1024];
    let len = info.len().min(bytes.len() as u64) as usize;
    info.read_at(0, &mut bytes[..len]).is_ok()
        && bytes[..len]
            .split(|&byte| byte == b'\n')
            .any(|line| line == b"PRODUCT='pve'")
}

pub fn boot(
    iso: &mut Iso,
    partition: &Ext4,
    path: &str,
) -> Result<(Vec<u8>, Vec<u8>, String), Status> {
    let kernel = iso.read_boot_file(KERNEL)?;
    let mut initrd = iso.read_boot_file(INITRD)?;
    linux::append_reopen(
        &mut initrd,
        &format!("{}\n", partition.uuid()),
        &format!("{path}\n"),
        &[],
    )?;
    Ok((
        kernel,
        initrd,
        String::from("ramdisk_size=16777216 rw rdinit=/efi-sk-reopen proxtui"),
    ))
}
