use uefi::Result;
use uefi::proto::console::gop::{BltOp, BltPixel, BltRegion, GraphicsOutput};

const SHAPE: [&[u8; 11]; 16] = [
    b"X          ",
    b"XX         ",
    b"XoX        ",
    b"XooX       ",
    b"XoooX      ",
    b"XooooX     ",
    b"XoooooX    ",
    b"XooooooX   ",
    b"XoooooooX  ",
    b"XooooooooX ",
    b"XooooXXXXXX",
    b"XooXoX     ",
    b"XoXXoX     ",
    b"XX  XoX    ",
    b"    XoX    ",
    b"     XX    ",
];
const WIDTH: usize = SHAPE[0].len();
const HEIGHT: usize = SHAPE.len();

pub struct Cursor {
    origin: (usize, usize),
    size: (usize, usize),
    background: [BltPixel; WIDTH * HEIGHT],
}

impl Cursor {
    pub fn draw(
        output: &mut GraphicsOutput,
        origin: (usize, usize),
        bounds: (usize, usize),
    ) -> Result<Self> {
        let size = (
            WIDTH.min(bounds.0 - origin.0),
            HEIGHT.min(bounds.1 - origin.1),
        );
        let mut background = [BltPixel::new(0, 0, 0); WIDTH * HEIGHT];
        output.blt(BltOp::VideoToBltBuffer {
            buffer: &mut background,
            src: origin,
            dest: BltRegion::Full,
            dims: size,
        })?;

        let mut pixels = background;
        for (row, pattern) in SHAPE.iter().take(size.1).enumerate() {
            for (column, symbol) in pattern.iter().take(size.0).enumerate() {
                let pixel = &mut pixels[row * size.0 + column];
                match symbol {
                    b'X' => *pixel = 0x101010.into(),
                    b'o' => *pixel = 0xfafafa.into(),
                    _ => {}
                }
            }
        }
        output.blt(BltOp::BufferToVideo {
            buffer: &pixels,
            src: BltRegion::Full,
            dest: origin,
            dims: size,
        })?;
        Ok(Self {
            origin,
            size,
            background,
        })
    }

    pub fn erase(self, output: &mut GraphicsOutput) -> Result {
        output.blt(BltOp::BufferToVideo {
            buffer: &self.background,
            src: BltRegion::Full,
            dest: self.origin,
            dims: self.size,
        })
    }
}
