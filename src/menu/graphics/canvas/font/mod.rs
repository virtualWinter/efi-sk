const ASCII_GLYPHS: usize = 95;
const GLYPH_COUNT: usize = ASCII_GLYPHS + 1;
const RECORD_SIZE: usize = 10;
const BITMAP_START: usize = GLYPH_COUNT * RECORD_SIZE;

// Matches tools/build_font_atlas.py. All advances are in 1/64 pixels.
pub const SUBPIXELS: i32 = 64;

#[derive(Clone, Copy)]
pub enum Face {
    Regular,
}

pub struct Glyph {
    pub advance: i32,
    pub left: i32,
    pub top: i32,
    pub width: usize,
    pub pixels: &'static [u8],
}

impl Face {
    pub fn glyph(self, character: char) -> Glyph {
        let atlas: &[u8] = match self {
            Self::Regular => include_bytes!("../../../../../assets/geist-regular.atlas"),
        };
        let index = match character {
            ' '..='~' => character as usize - b' ' as usize,
            '…' => ASCII_GLYPHS,
            _ => (b'?' - b' ') as usize,
        };
        let start = index * RECORD_SIZE;
        let record = &atlas[start..start + RECORD_SIZE];
        let offset = BITMAP_START + u32::from_le_bytes(record[..4].try_into().unwrap()) as usize;
        let advance = i32::from(u16::from_le_bytes([record[4], record[5]]));
        let width = usize::from(record[8]);
        let height = usize::from(record[9]);

        Glyph {
            advance,
            left: i32::from(record[6] as i8),
            top: i32::from(record[7] as i8),
            width,
            pixels: &atlas[offset..offset + width * height],
        }
    }

    pub fn measure(self, text: &str) -> i32 {
        text.chars()
            .map(|character| self.glyph(character).advance)
            .sum()
    }
}
