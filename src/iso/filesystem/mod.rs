use alloc::{string::String, vec::Vec};
use ext4_view::File;
use uefi::Status;

const SECTOR: usize = 2048;
const MAX_DIRECTORY: usize = 8 * 1024 * 1024;
const MAX_BOOT_FILE: usize = 512 * 1024 * 1024;

#[derive(Clone, Copy)]
struct Extent {
    offset: u64,
    length: usize,
    directory: bool,
}

impl Extent {
    fn is_boot_file(self, image_size: u64) -> bool {
        !self.directory
            && (1..=MAX_BOOT_FILE).contains(&self.length)
            && self.offset + self.length as u64 <= image_size
    }

    fn parse(record: &[u8]) -> Result<Self, Status> {
        if record.len() < 34 || record[1] != 0 || record[25] & 0x80 != 0 {
            return Err(Status::UNSUPPORTED);
        }

        let lba = u32::from_le_bytes(record[2..6].try_into().unwrap());
        let length = u32::from_le_bytes(record[10..14].try_into().unwrap());
        Ok(Self {
            offset: u64::from(lba) * SECTOR as u64,
            length: length as usize,
            directory: record[25] & 2 != 0,
        })
    }
}

pub struct Iso {
    file: File,
    root: Extent,
}

impl Iso {
    pub fn open(mut file: File) -> Result<Self, Status> {
        let mut descriptor = [0; SECTOR];
        for sector in 16..80 {
            read_at(&mut file, sector * SECTOR as u64, &mut descriptor)?;
            if &descriptor[1..7] != b"CD001\x01" {
                return Err(Status::VOLUME_CORRUPTED);
            }
            match descriptor[0] {
                1 => {
                    if u16::from_le_bytes([descriptor[128], descriptor[129]]) != SECTOR as u16 {
                        return Err(Status::UNSUPPORTED);
                    }
                    let root = Extent::parse(&descriptor[156..])?;
                    return Ok(Self { file, root });
                }
                255 => break,
                _ => {}
            }
        }
        Err(Status::UNSUPPORTED)
    }

    pub fn has_boot_file(&mut self, path: &str) -> bool {
        self.lookup(path)
            .is_ok_and(|extent| extent.is_boot_file(self.file.metadata().len()))
    }

    pub fn read_boot_file(&mut self, path: &str) -> Result<Vec<u8>, Status> {
        let extent = self.lookup(path)?;
        if !extent.is_boot_file(self.file.metadata().len()) {
            return Err(Status::UNSUPPORTED);
        }

        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(extent.length)
            .map_err(|_| Status::OUT_OF_RESOURCES)?;
        bytes.resize(extent.length, 0);
        read_at(&mut self.file, extent.offset, &mut bytes)?;
        Ok(bytes)
    }

    pub fn open_file(&mut self, path: &str) -> Result<IsoFile<'_>, Status> {
        let extent = self.lookup(path)?;
        if extent.directory {
            return Err(Status::UNSUPPORTED);
        }
        Ok(IsoFile {
            file: &mut self.file,
            extent,
        })
    }

    fn lookup(&mut self, path: &str) -> Result<Extent, Status> {
        let mut extent = self.root;
        for component in path.split('/').filter(|part| !part.is_empty()) {
            if !extent.directory {
                return Err(Status::NOT_FOUND);
            }
            extent = self.find_entry(extent, component)?;
        }
        Ok(extent)
    }

    fn find_entry(&mut self, dir: Extent, name: &str) -> Result<Extent, Status> {
        let mut found = None;
        self.for_each_record(dir, |record| {
            if entry_matches(record, name.as_bytes())? {
                found = Some(Extent::parse(record)?);
                return Ok(false);
            }
            Ok(true)
        })?;
        found.ok_or(Status::NOT_FOUND)
    }

    /// Lists the entries of the directory at `path` as `(name, size)`.
    pub fn read_dir(&mut self, path: &str) -> Result<Vec<(String, u64)>, Status> {
        let dir = self.lookup(path)?;
        if !dir.directory {
            return Err(Status::NOT_FOUND);
        }
        let mut entries = Vec::new();
        self.for_each_record(dir, |record| {
            if let Some((name, _)) = record_name(record)? {
                let size = record.get(10..14).ok_or(Status::VOLUME_CORRUPTED)?;
                entries.push((
                    name,
                    u64::from(u32::from_le_bytes(size.try_into().unwrap())),
                ));
            }
            Ok(true)
        })?;
        Ok(entries)
    }

    /// Visits every directory record, stopping early when the callback
    /// returns `Ok(false)`.
    fn for_each_record(
        &mut self,
        dir: Extent,
        mut visit: impl FnMut(&[u8]) -> Result<bool, Status>,
    ) -> Result<(), Status> {
        if dir.length > MAX_DIRECTORY || dir.offset + dir.length as u64 > self.file.metadata().len()
        {
            return Err(Status::VOLUME_CORRUPTED);
        }
        let mut sector = [0; SECTOR];
        for offset in (0..dir.length).step_by(SECTOR) {
            let length = (dir.length - offset).min(SECTOR);
            read_at(
                &mut self.file,
                dir.offset + offset as u64,
                &mut sector[..length],
            )?;
            let mut records = &sector[..length];
            while records.first().is_some_and(|length| *length != 0) {
                let record_length = usize::from(records[0]);
                let (record, rest) = records
                    .split_at_checked(record_length)
                    .ok_or(Status::VOLUME_CORRUPTED)?;
                records = rest;
                if !visit(record)? {
                    return Ok(());
                }
            }
        }
        Ok(())
    }
}

pub struct IsoFile<'a> {
    file: &'a mut File,
    extent: Extent,
}

impl IsoFile<'_> {
    pub fn len(&self) -> u64 {
        self.extent.length as u64
    }

    pub fn read_at(&mut self, offset: u64, bytes: &mut [u8]) -> Result<(), Status> {
        if offset
            .checked_add(bytes.len() as u64)
            .is_none_or(|end| end > self.len())
        {
            return Err(Status::VOLUME_CORRUPTED);
        }
        read_at(self.file, self.extent.offset + offset, bytes)
    }
}

fn read_at(file: &mut File, offset: u64, mut bytes: &mut [u8]) -> Result<(), Status> {
    if offset
        .checked_add(bytes.len() as u64)
        .is_none_or(|end| end > file.metadata().len())
    {
        return Err(Status::VOLUME_CORRUPTED);
    }
    if file.position() != offset {
        file.seek_to(offset).map_err(|_| Status::DEVICE_ERROR)?;
    }
    while !bytes.is_empty() {
        let read = file.read_bytes(bytes).map_err(|_| Status::DEVICE_ERROR)?;
        if read == 0 {
            return Err(Status::END_OF_FILE);
        }
        bytes = &mut bytes[read..];
    }
    Ok(())
}

fn entry_matches(record: &[u8], name: &[u8]) -> Result<bool, Status> {
    let Some((entry_name, exact)) = record_name(record)? else {
        return Ok(false);
    };
    Ok(if exact {
        entry_name.as_bytes() == name
    } else {
        entry_name.as_bytes().eq_ignore_ascii_case(name)
    })
}

/// Reads an entry's name, preferring the Rock Ridge name over the ISO9660
/// identifier. The flag reports whether the name must be matched exactly.
fn record_name(record: &[u8]) -> Result<Option<(String, bool)>, Status> {
    let length = usize::from(*record.get(32).ok_or(Status::VOLUME_CORRUPTED)?);
    let end = 33 + length;
    let identifier = record.get(33..end).ok_or(Status::VOLUME_CORRUPTED)?;
    if matches!(identifier, [0] | [1]) {
        return Ok(None);
    }
    // Rock Ridge provides the original name for ISO9660 identifiers with substitutions.
    let start = end + usize::from(length % 2 == 0);
    let mut system_use = record.get(start..).unwrap_or_default();
    while system_use.len() >= 4 {
        let size = usize::from(system_use[2]);
        if size < 4 || size > system_use.len() {
            break;
        }
        let (field, rest) = system_use.split_at(size);
        if field.starts_with(b"NM") && field.get(3..5) == Some(&[1, 0]) {
            return Ok(String::from_utf8(field[5..].to_vec())
                .ok()
                .map(|name| (name, true)));
        }
        system_use = rest;
    }

    let identifier = identifier
        .split(|byte| *byte == b';')
        .next()
        .unwrap_or_default();
    let identifier = identifier.strip_suffix(b".").unwrap_or(identifier);
    Ok(String::from_utf8(identifier.to_vec())
        .ok()
        .map(|name| (name, false)))
}
