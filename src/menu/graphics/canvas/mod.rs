mod font;

use alloc::vec::Vec;
use uefi::proto::console::gop::BltPixel;

pub use self::font::Face;
use self::font::SUBPIXELS;

pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<BltPixel>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Option<Self> {
        let mut pixels = Vec::new();
        pixels.try_reserve_exact(width * height).ok()?;
        pixels.resize(width * height, BltPixel::new(0, 0, 0));
        Some(Self {
            width,
            height,
            pixels,
        })
    }

    pub fn rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: u32) {
        let right = (x + width).min(self.width);
        if x >= right {
            return;
        }
        let color = BltPixel::from(color);
        for row in y..(y + height).min(self.height) {
            let start = row * self.width;
            self.pixels[start + x..start + right].fill(color);
        }
    }

    pub fn rounded_rect(
        &mut self,
        (x, y): (usize, usize),
        (width, height): (usize, usize),
        radius: usize,
        color: u32,
    ) {
        let radius = radius.min(width / 2).min(height / 2);
        self.rect(x, y + radius, width, height - 2 * radius, color);
        for row in 0..radius {
            let distance = radius - row - 1;
            let mut inset = 0;
            while (radius - inset).pow(2) + distance.pow(2) > radius.pow(2) {
                inset += 1;
            }
            self.rect(x + inset, y + row, width - 2 * inset, 1, color);
            self.rect(x + inset, y + height - 1 - row, width - 2 * inset, 1, color);
        }
    }

    pub fn text(&mut self, x: usize, baseline: usize, text: &str, color: u32) {
        self.styled_text((x, baseline), text, Face::Regular, color);
    }

    pub fn styled_text(
        &mut self,
        (x, baseline): (usize, usize),
        text: &str,
        face: Face,
        color: u32,
    ) {
        let mut pen = x as i32 * SUBPIXELS;
        let ink = BltPixel::from(color);

        for character in text.chars() {
            let glyph = face.glyph(character);
            let left = (pen + SUBPIXELS / 2) / SUBPIXELS + glyph.left;
            let top = baseline as i32 + glyph.top;

            for (index, alpha) in glyph
                .pixels
                .iter()
                .copied()
                .enumerate()
                .filter(|(_, alpha)| *alpha != 0)
            {
                let px = left + (index % glyph.width) as i32;
                let py = top + (index / glyph.width) as i32;
                if px >= 0 && py >= 0 && (px as usize) < self.width && (py as usize) < self.height {
                    composite(
                        &mut self.pixels[py as usize * self.width + px as usize],
                        ink,
                        alpha,
                    );
                }
            }
            pen += glyph.advance;
        }
    }

    pub fn icon(&mut self, (x, y): (usize, usize), width: usize, pixels: &[u8]) {
        for (index, rgba) in pixels.chunks_exact(4).enumerate() {
            let px = x + index % width;
            let py = y + index / width;
            if rgba[3] != 0 && px < self.width && py < self.height {
                let ink = BltPixel::new(rgba[0], rgba[1], rgba[2]);
                composite(&mut self.pixels[py * self.width + px], ink, rgba[3]);
            }
        }
    }

    pub fn fit_label<'a>(&self, label: &'a str, width: usize) -> (&'a str, bool) {
        let face = Face::Regular;
        let width = width as i32 * SUBPIXELS;
        if face.measure(label) <= width {
            return (label, false);
        }

        let available = width - face.measure("…");
        let mut used = 0;
        for (index, character) in label.char_indices() {
            used += face.glyph(character).advance;
            if used > available {
                return (&label[..index], true);
            }
        }
        (label, false)
    }

    pub fn text_width(&self, text: &str, face: Face) -> usize {
        ((face.measure(text) + SUBPIXELS / 2) / SUBPIXELS) as usize
    }
}

fn composite(background: &mut BltPixel, ink: BltPixel, alpha: u8) {
    let alpha = u16::from(alpha);
    let blend = |ink: u8, base: u8| {
        ((u16::from(ink) * alpha + u16::from(base) * (255 - alpha)) / 255) as u8
    };
    *background = BltPixel::new(
        blend(ink.red, background.red),
        blend(ink.green, background.green),
        blend(ink.blue, background.blue),
    );
}
