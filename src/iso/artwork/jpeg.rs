use alloc::boxed::Box;
use zune_jpeg::JpegDecoder;
use zune_jpeg::zune_core::{bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions};

use super::{Icon, scale_icon};

pub fn decode_debian_logo(data: &[u8]) -> Option<Box<Icon>> {
    let options = DecoderOptions::default()
        .set_max_width(128)
        .set_max_height(128)
        .jpeg_set_out_colorspace(ColorSpace::RGB);
    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(data), options);
    decoder.decode_headers().ok()?;
    let info = decoder.info()?;
    let (width, height) = (usize::from(info.width), usize::from(info.height));
    if !(1..=128).contains(&width) || !(1..=128).contains(&height) {
        return None;
    }
    let pixels = decoder.decode().ok()?;
    if pixels.len() != width * height * 3 {
        return None;
    }
    Some(scale_icon(width, height, |x, y| {
        // Recover coverage from the green channel of the red swirl on white.
        // Ignore near-white JPEG noise so the menu background stays clear.
        let green = pixels[(y * width + x) * 3 + 1];
        let alpha = (u16::from(247u8.saturating_sub(green)) * 255 / 247) as u8;
        ([215, 0, 60], alpha)
    }))
}
