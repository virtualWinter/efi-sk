pub(super) mod jpeg;
pub(super) mod png;
pub(super) mod squashfs;

use alloc::{boxed::Box, string::String};
use uefi::Handle;

use super::volume;
use crate::entries::{Entry, Target};

pub const ICON_SIZE: usize = 18;
pub type Icon = [u8; ICON_SIZE * ICON_SIZE * 4];

pub(super) struct Metadata {
    pub(super) label: Option<String>,
    pub(super) icon: Option<Box<Icon>>,
}

pub fn load(esp: Handle, entries: &mut [Entry], need_icons: bool) {
    let needs_metadata = |entry: &Entry| match entry.target {
        Target::Iso(profile, _) => profile.needs_metadata(need_icons),
        _ => false,
    };
    if !entries.iter().any(needs_metadata) {
        return;
    }
    let Ok(partition) = volume::open(esp) else {
        return;
    };
    for entry in entries.iter_mut().filter(|entry| needs_metadata(entry)) {
        let Target::Iso(profile, path) = &entry.target else {
            continue;
        };
        let Some(metadata) = profile.metadata(&partition, path, need_icons) else {
            continue;
        };
        if let Some(label) = metadata.label {
            entry.label = label;
        }
        if let Some(icon) = metadata.icon {
            entry.icon = Some(icon);
        }
    }
}

fn scale_icon(
    width: usize,
    height: usize,
    pixel: impl Fn(usize, usize) -> ([u8; 3], u8),
) -> Box<Icon> {
    let longest = width.max(height);
    let out_width = (width * ICON_SIZE / longest).max(1);
    let out_height = (height * ICON_SIZE / longest).max(1);
    let x_pad = (ICON_SIZE - out_width) / 2;
    let y_pad = (ICON_SIZE - out_height) / 2;
    let mut icon = Box::new([0; ICON_SIZE * ICON_SIZE * 4]);

    for row in 0..out_height {
        let top = row * height / out_height;
        let bottom = ((row + 1) * height / out_height).max(top + 1);
        for column in 0..out_width {
            let left = column * width / out_width;
            let right = ((column + 1) * width / out_width).max(left + 1);
            let mut rgb = [0u32; 3];
            let mut alpha = 0u32;
            for y in top..bottom {
                for x in left..right {
                    let (color, coverage) = pixel(x, y);
                    let coverage = u32::from(coverage);
                    alpha += coverage;
                    for (sum, channel) in rgb.iter_mut().zip(color) {
                        *sum += u32::from(channel) * coverage;
                    }
                }
            }
            if alpha != 0 {
                let dest = ((row + y_pad) * ICON_SIZE + column + x_pad) * 4;
                for (index, value) in rgb.into_iter().enumerate() {
                    icon[dest + index] = (value / alpha) as u8;
                }
                icon[dest + 3] = (alpha / ((bottom - top) * (right - left)) as u32) as u8;
            }
        }
    }
    icon
}
