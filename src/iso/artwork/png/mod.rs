use alloc::boxed::Box;
use zune_png::PngDecoder;
use zune_png::zune_core::{bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions};

use super::Icon;

const MAX_DIMENSION: usize = 512;

pub fn decode(data: &[u8]) -> Option<Box<Icon>> {
    decode_with_crop(data, |width, height| Some((0, 0, width, height)))
}

pub fn decode_debian_logo(data: &[u8]) -> Option<Box<Icon>> {
    decode_with_crop(data, |width, height| {
        Some(if (width, height) == (70, 120) {
            // This Debian installer asset includes a reflection below the swirl.
            (8, 9, 52, 64)
        } else {
            (0, 0, width, height)
        })
    })
}

pub fn decode_ubuntu_logo(data: &[u8]) -> Option<Box<Icon>> {
    decode_with_crop(data, |width, height| {
        // Ubuntu ships the Circle of Friends inside the orange square at the
        // left of the "Ubuntu" wordmark. A plain square asset needs no crop.
        match (width, height) {
            (187, 72) => Some((0, 28, 41, 41)),
            (width, height) if width == height => Some((0, 0, width, height)),
            _ => None,
        }
    })
}

fn decode_with_crop(
    data: &[u8],
    crop: impl FnOnce(usize, usize) -> Option<(usize, usize, usize, usize)>,
) -> Option<Box<Icon>> {
    let options = DecoderOptions::default()
        .set_max_width(MAX_DIMENSION)
        .set_max_height(MAX_DIMENSION)
        .png_set_add_alpha_channel(true)
        .png_set_strip_to_8bit(true);
    let mut decoder = PngDecoder::new_with_options(ZCursor::new(data), options);
    decoder.decode_headers().ok()?;
    let (width, height) = decoder.dimensions()?;
    if width == 0
        || height == 0
        || width > MAX_DIMENSION
        || height > MAX_DIMENSION
        || decoder.is_animated()
    {
        return None;
    }
    let pixels = decoder.decode_raw().ok()?;
    let (channels, grayscale) = match decoder.colorspace()? {
        ColorSpace::RGBA => (4, false),
        ColorSpace::RGB => (3, false),
        ColorSpace::LumaA => (2, true),
        ColorSpace::Luma => (1, true),
        _ => return None,
    };
    if pixels.len() != width * height * channels {
        return None;
    }

    let source_width = width;
    let source_height = height;
    let (crop_x, crop_y, width, height) = crop(width, height)?;
    let inside = crop_x
        .checked_add(width)
        .is_some_and(|end| end <= source_width)
        && crop_y
            .checked_add(height)
            .is_some_and(|end| end <= source_height);
    if width == 0 || height == 0 || !inside {
        return None;
    }
    Some(super::scale_icon(width, height, |x, y| {
        let offset = ((crop_y + y) * source_width + crop_x + x) * channels;
        let color = if grayscale {
            [pixels[offset]; 3]
        } else {
            [pixels[offset], pixels[offset + 1], pixels[offset + 2]]
        };
        let alpha = if channels == 4 || channels == 2 {
            pixels[offset + channels - 1]
        } else {
            255
        };
        (color, alpha)
    }))
}
