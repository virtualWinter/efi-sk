mod xz;

use alloc::{vec, vec::Vec};

use crate::iso::filesystem::IsoFile;

const META_SIZE: usize = 8192;
const MAX_DIRECTORY: usize = 256 * 1024;

pub struct SquashFs<'a> {
    image: IsoFile<'a>,
    size: u64,
    block_size: usize,
    root: u64,
    inodes: u64,
    directories: u64,
    fragment_index: u64,
    fragment_count: u32,
}

impl<'a> SquashFs<'a> {
    pub fn open(mut image: IsoFile<'a>) -> Option<Self> {
        let mut header = [0; 96];
        image.read_at(0, &mut header).ok()?;
        let size = le64(&header, 40);
        let block_size = le32(&header, 12) as usize;
        if &header[..4] != b"hsqs"
            || le16(&header, 20) != 4
            || (le16(&header, 28), le16(&header, 30)) != (4, 0)
            || size > image.len()
            || size < header.len() as u64
            || !(4096..=1024 * 1024).contains(&block_size)
            || !block_size.is_power_of_two()
        {
            return None;
        }

        Some(Self {
            image,
            size,
            block_size,
            root: le64(&header, 32),
            inodes: le64(&header, 64),
            directories: le64(&header, 72),
            fragment_index: le64(&header, 80),
            fragment_count: le32(&header, 16),
        })
    }

    pub fn file(&mut self, path: &str, limit: usize) -> Option<Vec<u8>> {
        let mut reference = self.root;
        for component in path.split('/') {
            let inode = self.inode(reference)?;
            let (block, offset, size) = match le16(&inode, 0) {
                1 => (
                    le32(&inode, 16),
                    le16(&inode, 26),
                    u32::from(le16(&inode, 24)),
                ),
                8 => (le32(&inode, 24), le16(&inode, 34), le32(&inode, 20)),
                _ => return None,
            };
            let size = size.checked_sub(3)? as usize;
            if size > MAX_DIRECTORY {
                return None;
            }
            let directory = self.metadata(
                self.directories.checked_add(u64::from(block))?,
                usize::from(offset),
                size,
            )?;
            reference = child(&directory, component.as_bytes())?;
        }

        let inode = self.inode(reference)?;
        let (start, length, fragment, offset, block_entry) = match le16(&inode, 0) {
            2 => (
                u64::from(le32(&inode, 16)),
                u64::from(le32(&inode, 28)),
                le32(&inode, 20),
                le32(&inode, 24),
                32,
            ),
            9 => (
                le64(&inode, 16),
                le64(&inode, 24),
                le32(&inode, 44),
                le32(&inode, 48),
                56,
            ),
            _ => return None,
        };
        let length = usize::try_from(length).ok()?;
        if length == 0 || length > limit || length >= self.block_size {
            return None;
        }
        if fragment == u32::MAX {
            let bytes = self.data_block(start, le32(&inode, block_entry))?;
            return Some(bytes.get(..length)?.to_vec());
        }
        if fragment >= self.fragment_count {
            return None;
        }
        let table_byte = u64::from(fragment) * 16;
        let block = self
            .fragment_index
            .checked_add(table_byte / META_SIZE as u64 * 8)?;
        let mut pointer = [0; 8];
        self.read(block, &mut pointer)?;
        let entry = self.metadata(
            le64(&pointer, 0),
            (table_byte % META_SIZE as u64) as usize,
            16,
        )?;
        let bytes = self.data_block(le64(&entry, 0), le32(&entry, 8))?;
        let offset = offset as usize;
        Some(bytes.get(offset..offset.checked_add(length)?)?.to_vec())
    }

    fn inode(&mut self, reference: u64) -> Option<Vec<u8>> {
        let block = self.inodes.checked_add(reference >> 16)?;
        let offset = (reference & 0xffff) as usize;
        // Fixed fields of the largest inode plus the first file data-block size.
        self.metadata(block, offset, 60)
    }

    fn metadata(&mut self, mut block: u64, mut offset: usize, length: usize) -> Option<Vec<u8>> {
        let mut result = Vec::with_capacity(length);
        while result.len() < length {
            let mut header = [0; 2];
            self.read(block, &mut header)?;
            let header = le16(&header, 0);
            let size = usize::from(header & 0x7fff);
            if !(1..=META_SIZE).contains(&size) {
                return None;
            }
            block = block.checked_add(2)?;
            let mut data = vec![0; size];
            self.read(block, &mut data)?;
            if header & 0x8000 == 0 {
                data = xz::decode(&data, META_SIZE)?;
            }
            let data = data.get(offset..)?;
            if data.is_empty() {
                return None;
            }
            result.extend_from_slice(&data[..data.len().min(length - result.len())]);
            block = block.checked_add(size as u64)?;
            offset = 0;
        }
        Some(result)
    }

    fn data_block(&mut self, offset: u64, encoded_size: u32) -> Option<Vec<u8>> {
        let size = (encoded_size & 0x00ff_ffff) as usize;
        if !(1..=self.block_size).contains(&size) || encoded_size >> 25 != 0 {
            return None;
        }
        let mut bytes = vec![0; size];
        self.read(offset, &mut bytes)?;
        if encoded_size & 0x0100_0000 != 0 {
            Some(bytes)
        } else {
            xz::decode(&bytes, self.block_size)
        }
    }

    fn read(&mut self, offset: u64, data: &mut [u8]) -> Option<()> {
        if offset.checked_add(data.len() as u64)? > self.size {
            return None;
        }
        self.image.read_at(offset, data).ok()
    }
}

fn child(mut directory: &[u8], name: &[u8]) -> Option<u64> {
    while !directory.is_empty() {
        let (header, rest) = directory.split_at_checked(12)?;
        directory = rest;
        let count = le32(header, 0).checked_add(1)?;
        if count > 256 {
            return None;
        }
        for _ in 0..count {
            let (entry, rest) = directory.split_at_checked(8)?;
            let length = usize::from(le16(entry, 6)) + 1;
            let (entry_name, rest) = rest.split_at_checked(length)?;
            directory = rest;
            if entry_name == name {
                return Some((u64::from(le32(header, 4)) << 16) | u64::from(le16(entry, 0)));
            }
        }
    }
    None
}

fn le16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
}

fn le32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

fn le64(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())
}
