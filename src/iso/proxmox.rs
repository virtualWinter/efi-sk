use alloc::{vec, vec::Vec};
use uefi::{Handle, Status};

use super::{filesystem::Iso, linux};

const KERNEL: &str = "boot/linux26";
const INITRD: &str = "boot/initrd.img";
const ISO_NAME: &[u8] = b"proxmox.iso";
const TRAILER: &[u8] = b"TRAILER!!!";
const MAX_ISO: u64 = 2 * 1024 * 1024 * 1024;

pub fn detect(iso: &mut Iso) -> bool {
    if iso.image_len() > MAX_ISO || !iso.has_boot_file(KERNEL) || !iso.has_boot_file(INITRD) {
        return false;
    }
    let Ok(mut info) = iso.open_file(".disk/info") else {
        return false;
    };
    let mut bytes = [0; 1024];
    let len = info.len().min(bytes.len() as u64) as usize;
    info.read_at(0, &mut bytes[..len]).is_ok()
        && bytes[..len]
            .windows(b"Proxmox VE".len())
            .any(|part| part == b"Proxmox VE")
}

pub fn launch(parent: Handle, mut iso: Iso) -> Result<(), Status> {
    if !detect(&mut iso) {
        return Err(Status::UNSUPPORTED);
    }
    let kernel = iso.read_boot_file(KERNEL)?;
    let mut initrd = iso.read_boot_file(INITRD)?;
    let size = iso.image_len() as usize;
    // Proxmox's iPXE boot uses the same initramfs layout: the installer
    // looks for a file named /proxmox.iso in the unpacked initramfs.
    initrd
        .try_reserve_exact(128)
        .map_err(|_| Status::OUT_OF_RESOURCES)?;
    pad(&mut initrd);
    header(&mut initrd, ISO_NAME, size as u32, 0o100644, 1);
    let mut suffix = Vec::new();
    header(&mut suffix, TRAILER, 0, 0, 0);
    let mut tail = vec![0; size.next_multiple_of(4) - size];
    tail.extend_from_slice(&suffix);
    linux::launch_payload(
        parent,
        kernel,
        linux::Payload {
            prefix: initrd,
            media: Some((iso, tail)),
        },
        "ramdisk_size=16777216 rw",
    )
}

fn header(out: &mut Vec<u8>, name: &[u8], size: u32, mode: u32, inode: u32) {
    // Linux accepts concatenated initramfs members in the cpio newc format.
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
