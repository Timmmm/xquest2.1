//! A software model of the VGA "Mode X" screen used by the original game.
//!
//! Video memory is four 64K planes. A pixel at `(x, y)` on a page starting at byte offset
//! `page` lives in plane `x & 3` at address `page + y * BYTE_WIDTH + (x >> 2)`. Storing the
//! planes interleaved makes that address `(page * 4 + y * LOGICAL_WIDTH + x) mod 2^18`, so
//! drawing outside a page wraps around exactly as on real hardware.

use crate::core::bitmap::Bitmap;

pub const VRAM_SIZE: usize = 4 * 65536;
pub const SCREEN_WIDTH: usize = 320;
pub const SCREEN_HEIGHT: usize = 240;

/// Width of the virtual screen in pixels.
pub const LOGICAL_WIDTH: i32 = 392;
const BYTE_WIDTH: i32 = LOGICAL_WIDTH / 4;

/// Screen line at which the split screen (the status bar) starts.
pub const SPLIT_SCREEN_LINE: i32 = 217;

/// 6-bit RGB DAC values.
pub type Palette = [[u8; 3]; 256];

/// Which part of video memory to draw on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    /// The page currently being displayed.
    Visible,
    /// The page being drawn for the next frame.
    Hidden,
    Page0,
    Page1,
    /// The status bar below the split.
    Split,
}

pub struct Vga {
    vram: Vec<u8>,
    pub palette: Palette,

    split_screen: bool,
    split_offset: i32,
    page0_offset: i32,
    page1_offset: i32,
    visible_offset: i32,
    hidden_offset: i32,
    /// Height of a page in lines.
    pub page_height: i32,

    /// Display start in pixels within the visible page.
    display_x: i32,
    display_y: i32,
}

impl Default for Vga {
    fn default() -> Self {
        Self::new()
    }
}

impl Vga {
    pub fn new() -> Self {
        let mut vga = Self {
            vram: vec![0; VRAM_SIZE],
            palette: [[0; 3]; 256],
            split_screen: false,
            split_offset: 0,
            page0_offset: 0,
            page1_offset: 0,
            visible_offset: 0,
            hidden_offset: 0,
            page_height: 0,
            display_x: 0,
            display_y: 0,
        };
        vga.set_single_page_mode();
        vga
    }

    /// One page covering all of video memory, no split screen. Used by the menus.
    pub fn set_single_page_mode(&mut self) {
        self.vram.fill(0);
        self.split_screen = false;
        self.split_offset = 0;
        self.page0_offset = 0;
        self.page1_offset = 0;
        self.visible_offset = 0;
        self.hidden_offset = 0;
        self.page_height = 65536 / BYTE_WIDTH;
        self.display_x = 0;
        self.display_y = 0;
    }

    /// Two pages of `page_height` lines for double buffering, and the status bar split
    /// screen at line 217. Used in game.
    pub fn set_double_buffer_mode(&mut self, page_height: i32) {
        self.set_single_page_mode();

        // One more line is reserved than is displayed below the split; the status bar fills
        // exactly these 24 lines.
        let split_height = SCREEN_HEIGHT as i32 - SPLIT_SCREEN_LINE + 1;

        self.split_screen = true;
        self.page0_offset = split_height * BYTE_WIDTH;
        self.page1_offset = self.page0_offset + page_height * BYTE_WIDTH;
        self.visible_offset = self.page0_offset;
        self.hidden_offset = self.page1_offset;
        self.page_height = page_height;
    }

    /// Shows the hidden page, scrolled to `(x, y)`.
    pub fn page_flip(&mut self, x: i32, y: i32) {
        std::mem::swap(&mut self.visible_offset, &mut self.hidden_offset);
        self.display_x = x;
        self.display_y = y;
    }

    fn offset(&self, page: Page) -> i32 {
        match page {
            Page::Visible => self.visible_offset,
            Page::Hidden => self.hidden_offset,
            Page::Page0 => self.page0_offset,
            Page::Page1 => self.page1_offset,
            Page::Split => self.split_offset,
        }
    }

    #[inline]
    fn index(offset: i32, x: i32, y: i32) -> usize {
        ((offset as i64 * 4 + y as i64 * LOGICAL_WIDTH as i64 + x as i64)
            .rem_euclid(VRAM_SIZE as i64)) as usize
    }

    #[inline]
    fn read(&self, offset: i32, x: i32, y: i32) -> u8 {
        self.vram[Self::index(offset, x, y)]
    }

    #[inline]
    fn write(&mut self, offset: i32, x: i32, y: i32, color: u8) {
        self.vram[Self::index(offset, x, y)] = color;
    }

    pub fn pixel(&self, x: i32, y: i32, page: Page) -> u8 {
        self.read(self.offset(page), x, y)
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, page: Page, color: u8) {
        self.write(self.offset(page), x, y, color);
    }

    /// Fills the rectangle from `(x1, y1)` up to but not including `(x2, y2)`.
    pub fn fill_rect(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, page: Page, color: u8) {
        let o = self.offset(page);
        for y in y1..y2 {
            for x in x1..x2 {
                self.write(o, x, y, color);
            }
        }
    }

    /// A Bresenham line including both end points.
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, page: Page, color: u8) {
        let o = self.offset(page);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (x0, y0);

        loop {
            self.write(o, x, y, color);

            if x == x1 && y == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Copies a block of the screen, `width` rounded up to a multiple of 4.
    pub fn capture(&self, x: i32, y: i32, width: i32, height: i32, page: Page) -> Bitmap {
        let o = self.offset(page);
        let width = (width + 3) / 4 * 4;
        let pixels = (0..height)
            .flat_map(|row| (0..width).map(move |col| (col, row)))
            .map(|(col, row)| self.read(o, x + col, y + row))
            .collect();

        Bitmap::new(width, height, pixels)
    }

    /// Draws every pixel of the (padded) bitmap, including zeros.
    pub fn draw_opaque(&mut self, x: i32, y: i32, page: Page, bm: &Bitmap) {
        self.draw_with(x, y, page, bm, Some);
    }

    /// Draws the bitmap with zero pixels transparent.
    pub fn draw_transparent(&mut self, x: i32, y: i32, page: Page, bm: &Bitmap) {
        self.draw_with(x, y, page, bm, |c| (c != 0).then_some(c));
    }

    /// Draws the (padded) bitmap opaquely, adding `tint` to every non-zero pixel.
    pub fn draw_tinted(&mut self, x: i32, y: i32, tint: u8, page: Page, bm: &Bitmap) {
        self.draw_with(x, y, page, bm, |c| {
            Some(if c != 0 { c.wrapping_add(tint) } else { 0 })
        });
    }

    /// Draws the padded bitmap; `f` maps each pixel to the colour to write, if any.
    fn draw_with(&mut self, x: i32, y: i32, page: Page, bm: &Bitmap, f: impl Fn(u8) -> Option<u8>) {
        let o = self.offset(page);

        for row in 0..bm.height {
            for col in 0..bm.padded_width() {
                if let Some(c) = f(bm.pixel(col, row)) {
                    self.write(o, x + col, y + row, c);
                }
            }
        }
    }

    /// Clears a whole page to colour 0.
    pub fn clear_page(&mut self, page: Page) {
        self.fill_rect(0, 0, LOGICAL_WIDTH, self.page_height, page, 0);
    }

    /// Copies a whole page to another.
    pub fn copy_page(&mut self, from: Page, to: Page) {
        let (src, dst) = (self.offset(from), self.offset(to));
        for y in 0..self.page_height {
            for x in 0..LOGICAL_WIDTH {
                let c = self.read(src, x, y);
                self.write(dst, x, y, c);
            }
        }
    }

    pub fn set_color(&mut self, index: u8, rgb: [u8; 3]) {
        self.palette[index as usize] = rgb.map(|v| v & 63);
    }

    /// Renders what the monitor would show as 320x240 RGBA pixels.
    pub fn render_rgba(&self, out: &mut [u8]) {
        let rgba: Vec<[u8; 4]> = self
            .palette
            .iter()
            .map(|p| {
                let c = |v: u8| (v << 2) | (v >> 4);
                [c(p[0]), c(p[1]), c(p[2]), 255]
            })
            .collect();

        let split_line = if self.split_screen {
            SPLIT_SCREEN_LINE as usize
        } else {
            SCREEN_HEIGHT
        };

        for row in 0..SCREEN_HEIGHT {
            for col in 0..SCREEN_WIDTH {
                let c = if row < split_line {
                    self.read(
                        self.visible_offset,
                        self.display_x + col as i32,
                        self.display_y + row as i32,
                    )
                } else {
                    self.read(self.split_offset, col as i32, (row - split_line) as i32)
                };

                let o = (row * SCREEN_WIDTH + col) * 4;
                out[o..o + 4].copy_from_slice(&rgba[c as usize]);
            }
        }
    }
}

/// Converts an 8-bit colour component to a 6-bit DAC value.
pub fn to_dac(v: u8) -> u8 {
    v >> 2
}
