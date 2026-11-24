//! In-memory view of an OPNsense boot directory that asks loader(8) to
//! preload its optical media as a FreeBSD memory disk.
use alloc::vec::Vec;
use uefi::Status;

use super::filesystem::Iso;

const SECTOR: usize = 2048;
const CONFIG: &[u8] =
    b"mdroot_load=\"YES\"\nmdroot_type=\"md_image\"\nmdroot_name=\"/boot/efisk.iso\"\n";

pub(super) type Overlay = (u64, [u8; SECTOR]);

pub(super) fn media_overlay(iso: &mut Iso) -> Result<Vec<Overlay>, Status> {
    if !iso
        .name()
        .is_some_and(|name| name.eq_ignore_ascii_case("OPNSENSE_INSTALL"))
    {
        return Ok(Vec::new());
    }
    if iso.open_file("boot/loader.conf.local").is_ok() || iso.open_file("boot/efisk.iso").is_ok() {
        return Err(Status::UNSUPPORTED);
    }
    let size = u32::try_from(iso.image_len()).map_err(|_| Status::BAD_BUFFER_SIZE)?;
    let mut sector = [0; SECTOR];
    let mut config_offset = None;
    for lba in 1..16 {
        let offset = lba * SECTOR as u64;
        iso.read_image(offset, &mut sector)?;
        if sector.iter().all(|&byte| byte == 0) {
            config_offset = Some(offset);
            break;
        }
    }
    let config_offset = config_offset.ok_or(Status::UNSUPPORTED)?;
    let (offset, len) = iso.directory_location("boot")?;
    for step in (0..len).step_by(SECTOR) {
        if len - step < SECTOR {
            break;
        }
        let at = offset + step as u64;
        iso.read_image(at, &mut sector)?;
        let mut used = 0;
        while used < SECTOR && sector[used] != 0 {
            let len = usize::from(sector[used]);
            if len < 34 || used + len > SECTOR {
                return Err(Status::VOLUME_CORRUPTED);
            }
            used += len;
        }
        let config = record(
            b"LOADER.CONF_LOCAL;1",
            b"loader.conf.local",
            (config_offset / SECTOR as u64) as u32,
            CONFIG.len() as u32,
        );
        let media = record(b"EFISK.ISO;1", b"efisk.iso", 0, size);
        if used + config.len() + media.len() <= SECTOR {
            sector[used..used + config.len()].copy_from_slice(&config);
            used += config.len();
            sector[used..used + media.len()].copy_from_slice(&media);
            let mut contents = [0; SECTOR];
            contents[..CONFIG.len()].copy_from_slice(CONFIG);
            return Ok(alloc::vec![(config_offset, contents), (at, sector)]);
        }
    }
    Err(Status::UNSUPPORTED)
}

fn record(identifier: &[u8], name: &[u8], lba: u32, size: u32) -> Vec<u8> {
    let mut bytes = alloc::vec![0; 33];
    bytes[2..6].copy_from_slice(&lba.to_le_bytes());
    bytes[6..10].copy_from_slice(&lba.to_be_bytes());
    bytes[10..14].copy_from_slice(&size.to_le_bytes());
    bytes[14..18].copy_from_slice(&size.to_be_bytes());
    bytes[18..25].copy_from_slice(&[126, 1, 1, 0, 0, 0, 0]);
    bytes[28..32].copy_from_slice(&[1, 0, 0, 1]);
    bytes[32] = identifier.len() as u8;
    bytes.extend_from_slice(identifier);
    if identifier.len().is_multiple_of(2) {
        bytes.push(0);
    }
    bytes.extend_from_slice(&[b'N', b'M', (5 + name.len()) as u8, 1, 0]);
    bytes.extend_from_slice(name);
    if !bytes.len().is_multiple_of(2) {
        bytes.push(0);
    }
    bytes[0] = bytes.len() as u8;
    bytes
}
