use embedded_graphics::{
    drawable::Pixel,
    geometry::{Point, Size},
    pixelcolor::{
        raw::{RawData, RawU16},
        Rgb565,
    },
    primitives::Rectangle,
    style::{PrimitiveStyle, Styled},
    DrawTarget,
};

pub struct FrameBuffer<'a> {
    buf: &'a mut [[u16; 320]; 240],
    dirty: bool,
}

impl<'a> FrameBuffer<'a> {
    pub fn new(raw: &'a mut [[u16; 320]; 240]) -> Self {
        Self { buf: raw, dirty: false }
    }

    pub fn inner(&mut self) -> Option<&[u16]> {
        if self.dirty {
            self.dirty = false;
            Some(unsafe {
                core::slice::from_raw_parts(
                    self.buf.as_ptr().cast::<u16>(),
                    (self.width() * self.height()) as usize,
                )
            })
        } else {
            None
        }
    }

    fn width(&self) -> u32 {
        self.buf[0].len() as u32
    }

    fn height(&self) -> u32 {
        self.buf.len() as u32
    }
}

impl<'a> DrawTarget<Rgb565> for FrameBuffer<'a> {
    type Error = ();

    fn size(&self) -> Size {
        Size::new(self.width(), self.height())
    }

    fn draw_pixel(&mut self, pixel: Pixel<Rgb565>) -> Result<(), Self::Error> {
        let Pixel(pos, color) = pixel;

        if pos.x < 0 || pos.y < 0 || pos.x >= self.width() as i32 || pos.y >= self.height() as i32 {
            return Ok(());
        }
        self.dirty = true;
        self.buf[pos.y as usize][pos.x as usize] = swap(RawU16::from(color).into_inner());
        Ok(())
    }
}

const fn swap(inp: u16) -> u16 {
    (inp & 0x00FF) << 8 | (inp & 0xFF00) >> 8
}
