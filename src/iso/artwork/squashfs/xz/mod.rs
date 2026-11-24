use alloc::vec;
use alloc::vec::Vec;
use lzma_rust2::{Read, XzReader};

pub fn decode(input: &[u8], limit: usize) -> Option<Vec<u8>> {
    let mut output = vec![0; limit];
    // The memory limit of an LZMA2 stream is its declared dictionary
    // size. SquashFS xz streams often use a large dictionary (xz preset 6
    // uses an 8 MiB one) that the old 8 KiB limit rejected, so decode up to
    // `limit` bytes instead; the limit above already caps the output.
    let mut reader = XzReader::new_mem_limit(input, false, u32::MAX);
    let mut written = 0;
    while written < limit {
        let count = reader.read(&mut output[written..]).ok()?;
        if count == 0 {
            break;
        }
        written += count;
    }
    if reader.read(&mut [0]).ok()? != 0 {
        return None;
    }
    output.truncate(written);
    Some(output)
}
