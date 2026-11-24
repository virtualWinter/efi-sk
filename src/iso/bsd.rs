//! In-memory view of an OPNsense boot directory that asks loader(8) to
//! preload its optical media as a FreeBSD memory disk.
use alloc::vec::Vec;
use uefi::{Handle, Status, boot, cstr16, fs::FileSystem};

use super::filesystem::Iso;

const SECTOR: usize = 2048;
const CONFIG: &[u8] =
    b"mdroot_load=\"YES\"\nmdroot_type=\"md_image\"\nmdroot_name=\"/boot/efisk.iso\"\n";
const GEOM_CONFIG: &[u8] = b"geom_ventoy_load=\"YES\"\ngeom_ventoy_name=\"/boot/efisk.ko\"\nhint.ventoy.0.vlnk=\"1\"\ngeom_label_load=\"YES\"\n";
const MAGIC: [u8; 16] = [
    0x44, 0x33, 0x22, 0x11, 0x88, 0x77, 0x66, 0x55, 0xcc, 0xbb, 0xaa, 0x99, 0x00, 0xff, 0xee, 0xdd,
];
const MAX_RANGES: usize = 40960;
const MAP_SIZE: usize = 64 + 16 * MAX_RANGES + 16;

pub(super) type Overlay = (u64, [u8; SECTOR]);

#[derive(Default)]
pub(super) struct Handoff {
    pub overlays: Vec<Overlay>,
    pub module: Vec<u8>,
}

pub(super) fn media_overlay(
    esp: Handle,
    iso: &mut Iso,
    partition: (u64, u64),
) -> Result<Handoff, Status> {
    if !iso
        .name()
        .is_some_and(|name| name.eq_ignore_ascii_case("OPNSENSE_INSTALL"))
    {
        return Ok(Handoff::default());
    }
    if ["boot/loader.conf.local", "boot/efisk.iso", "boot/efisk.ko"]
        .iter()
        .any(|path| iso.open_file(path).is_ok())
    {
        return Err(Status::UNSUPPORTED);
    }
    let mut handoff = Handoff::default();
    let mut fs = FileSystem::new(boot::open_protocol_exclusive(esp).map_err(|e| e.status())?);
    let module_path = cstr16!("\\EFI\\BOOT\\opnsense-26.7-geom.ko");
    if fs
        .try_exists(module_path)
        .map_err(|_| Status::DEVICE_ERROR)?
    {
        let size = fs
            .metadata(module_path)
            .map_err(|_| Status::DEVICE_ERROR)?
            .file_size();
        if !(MAP_SIZE as u64..=2 * 1024 * 1024).contains(&size) {
            return Err(Status::BAD_BUFFER_SIZE);
        }
        let kernel = iso.read_boot_file("boot/kernel/kernel")?;
        if !kernel
            .windows(b"stable/26.7-n283674-12334a596709".len())
            .any(|part| part == b"stable/26.7-n283674-12334a596709")
        {
            return Err(Status::INCOMPATIBLE_VERSION);
        }
        handoff.module = fs.read(module_path).map_err(|_| Status::DEVICE_ERROR)?;
        let (disk_size, identity) = super::volume::disk_identity(esp)?;
        fill_map(&mut handoff.module, iso, partition, disk_size, identity)?;
    }
    let (settings, media) = if handoff.module.is_empty() {
        let size = u32::try_from(iso.image_len()).map_err(|_| Status::BAD_BUFFER_SIZE)?;
        (CONFIG, record(b"EFISK.ISO;1", b"efisk.iso", 0, size))
    } else {
        let lba =
            u32::try_from(iso.image_len() / SECTOR as u64).map_err(|_| Status::BAD_BUFFER_SIZE)?;
        let size = u32::try_from(handoff.module.len()).map_err(|_| Status::BAD_BUFFER_SIZE)?;
        handoff
            .module
            .resize(handoff.module.len().next_multiple_of(SECTOR), 0);
        (GEOM_CONFIG, record(b"EFISK.KO;1", b"efisk.ko", lba, size))
    };
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
            settings.len() as u32,
        );
        if used + config.len() + media.len() <= SECTOR {
            sector[used..used + config.len()].copy_from_slice(&config);
            used += config.len();
            sector[used..used + media.len()].copy_from_slice(&media);
            let mut contents = [0; SECTOR];
            contents[..settings.len()].copy_from_slice(settings);
            handoff.overlays = alloc::vec![(config_offset, contents), (at, sector)];
            return Ok(handoff);
        }
    }
    Err(Status::UNSUPPORTED)
}

fn fill_map(
    module: &mut [u8],
    iso: &Iso,
    (start, length): (u64, u64),
    disk_size: u64,
    identity: [u8; 16],
) -> Result<(), Status> {
    if !identity.starts_with(b"EFISKMAP") {
        return Err(Status::NO_MAPPING);
    }
    let mut candidates = module.windows(MAP_SIZE).enumerate().filter(|(_, window)| {
        window[..16] == MAGIC
            && window[MAP_SIZE - 16..] == MAGIC
            && window[16..MAP_SIZE - 16].iter().all(|&byte| byte == 0)
    });
    let (at, _) = candidates.next().ok_or(Status::LOAD_ERROR)?;
    if candidates.next().is_some() {
        return Err(Status::LOAD_ERROR);
    }
    let map = &mut module[at..at + MAP_SIZE];
    let limit = start
        .checked_add(length)
        .filter(|end| *end <= disk_size)
        .ok_or(Status::BAD_BUFFER_SIZE)?;
    let mut last = None;
    let mut count = 0;
    let mut total = 0;
    let mut zero_cursor = 0;
    if super::volume::ZERO_START + super::volume::ZERO_SIZE > start {
        return Err(Status::NO_MAPPING);
    }
    for block in iso.physical_blocks()? {
        let (offset, len) = block.map_err(|_| Status::DEVICE_ERROR)?;
        let (offset, end) = if let Some(offset) = offset {
            zero_cursor = 0;
            let offset = start.checked_add(offset).ok_or(Status::BAD_BUFFER_SIZE)?;
            let end = offset
                .checked_add(len)
                .filter(|end| *end <= limit)
                .ok_or(Status::BAD_BUFFER_SIZE)?;
            (offset, end)
        } else {
            if len > super::volume::ZERO_SIZE {
                return Err(Status::BAD_BUFFER_SIZE);
            }
            if zero_cursor + len > super::volume::ZERO_SIZE {
                zero_cursor = 0;
            }
            let offset = super::volume::ZERO_START + zero_cursor;
            zero_cursor += len;
            (offset, offset + len)
        };
        if len == 0 || (offset | len) & 511 != 0 {
            return Err(Status::BAD_BUFFER_SIZE);
        }
        total += len;
        if last != Some(offset) {
            if count == MAX_RANGES {
                return Err(Status::OUT_OF_RESOURCES);
            }
            let pos = 64 + count * 16;
            map[pos..pos + 8].copy_from_slice(&offset.to_le_bytes());
            count += 1;
        }
        let pos = 64 + (count - 1) * 16;
        map[pos + 8..pos + 16].copy_from_slice(&end.to_le_bytes());
        last = Some(end);
    }
    if total != iso.image_len() || count == 0 {
        return Err(Status::NO_MAPPING);
    }
    map[16..32].copy_from_slice(&MAGIC);
    map[32..40].copy_from_slice(&(count as u64).to_le_bytes());
    map[40..48].copy_from_slice(&disk_size.to_le_bytes());
    map[48..64].copy_from_slice(&identity);
    Ok(())
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
