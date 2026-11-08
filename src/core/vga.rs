//! A software model of the VGA "Mode X" screen used by the original game.
//!
//! Video memory is modelled as four 64K planes. A pixel at `(x, y)` on a page starting at
//! byte offset `page` lives in plane `x & 3` at address `page + y * byte_width + (x >> 2)`.
//! Storing the planes interleaved means that address is simply
//! `(page * 4 + y * pixel_width + x) mod 2^18`, so out of range drawing wraps around exactly
//! as it does on real hardware.

use crate::core::gfx::Bitmap;

pub const VRAM_SIZE: usize = 4 * 65536;
pub const PHYSICAL_WIDTH: usize = 320;
pub const PHYSICAL_HEIGHT: usize = 240;

/// Logical screen width used by the game (`PageWidth`).
pub const LOGICAL_WIDTH: i32 = 392;
pub const BYTE_WIDTH: i32 = LOGICAL_WIDTH / 4;

/// Screen line at which the split screen (the status bar) starts.
pub const SPLIT_SCREEN_LINE: i32 = 217;

pub struct Vga {
    pub vram: Vec<u8>,
    /// 6-bit RGB DAC palette.
    pub palette: [[u8; 3]; 256],

    pub split_screen: bool,
    pub split_offs: i32,
    pub page0_offs: i32,
    pub page1_offs: i32,
    pub visible_page_offs: i32,
    pub hidden_page_offs: i32,
    /// `ScrnLogicalHeight`.
    pub logical_height: i32,

    /// Display start in pixels within the visible page.
    pub display_x: i32,
    pub display_y: i32,
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
            split_offs: 0,
            page0_offs: 0,
            page1_offs: 0,
            visible_page_offs: 0,
            hidden_page_offs: 0,
            logical_height: 0,
            display_x: 0,
            display_y: 0,
        };
        vga.set_mode_no_split_screen();
        vga
    }

    /// `XSetMode(XMode320x240, PageWidth)` without split screen or double buffering.
    pub fn set_mode_no_split_screen(&mut self) {
        self.vram.fill(0);
        self.split_screen = false;
        self.split_offs = 0;
        self.page0_offs = 0;
        self.page1_offs = 0;
        self.visible_page_offs = 0;
        self.hidden_page_offs = 0;
        self.logical_height = 65536 / BYTE_WIDTH;
        self.display_x = 0;
        self.display_y = 0;
    }

    /// `SetXMode`: mode set, split screen at line 217 and two 320 line pages.
    pub fn set_mode_split_double_buffer(&mut self, page_height: i32) {
        self.set_mode_no_split_screen();

        self.split_screen = true;
        self.split_offs = 0;

        let split_height = PHYSICAL_HEIGHT as i32 - SPLIT_SCREEN_LINE;
        self.page0_offs = split_height * BYTE_WIDTH;
        self.page1_offs = self.page0_offs + page_height * BYTE_WIDTH;
        self.visible_page_offs = self.page0_offs;
        self.hidden_page_offs = self.page1_offs;
        self.logical_height = page_height;
    }

    /// `XPageFlip`: show the hidden page, panned to `(x, y)`.
    pub fn page_flip(&mut self, x: i32, y: i32) {
        std::mem::swap(&mut self.visible_page_offs, &mut self.hidden_page_offs);
        self.display_x = x;
        self.display_y = y;
    }

    #[inline]
    fn index(page: i32, x: i32, y: i32) -> usize {
        ((page as i64 * 4 + y as i64 * LOGICAL_WIDTH as i64 + x as i64)
            .rem_euclid(VRAM_SIZE as i64)) as usize
    }

    #[inline]
    pub fn get_pix(&self, x: i32, y: i32, page: i32) -> u8 {
        self.vram[Self::index(page, x, y)]
    }

    #[inline]
    pub fn put_pix(&mut self, x: i32, y: i32, page: i32, color: u8) {
        self.vram[Self::index(page, x, y)] = color;
    }

    /// `XRectFill`: fills up to but not including `(x2, y2)`.
    pub fn rect_fill(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, page: i32, color: u8) {
        for y in y1..y2 {
            for x in x1..x2 {
                self.put_pix(x, y, page, color);
            }
        }
    }

    /// `XLine`: a Bresenham line including both end points.
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, page: i32, color: u8) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (x0, y0);

        loop {
            self.put_pix(x, y, page, color);

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

    /// `XGetPBM`: copies a `byte_width * 4` by `height` block of pixels.
    pub fn get_pbm(&self, x: i32, y: i32, byte_width: i32, height: i32, page: i32) -> Bitmap {
        let width = byte_width * 4;
        let mut pixels = Vec::with_capacity((width * height) as usize);

        for row in 0..height {
            for col in 0..width {
                pixels.push(self.get_pix(x + col, y + row, page));
            }
        }

        Bitmap::new(width, byte_width, height, pixels)
    }

    /// `XPutPBM`: an opaque copy of the whole padded bitmap.
    pub fn put_pbm(&mut self, x: i32, y: i32, page: i32, bm: &Bitmap) {
        let w = bm.padded_width();

        for row in 0..bm.height {
            for col in 0..w {
                self.put_pix(x + col, y + row, page, bm.pixels[(row * w + col) as usize]);
            }
        }
    }

    /// `XPutCBitmap`: a compiled bitmap, in which zero pixels are transparent.
    pub fn put_cbitmap(&mut self, x: i32, y: i32, page: i32, bm: &Bitmap) {
        let w = bm.padded_width();

        for row in 0..bm.height {
            for col in 0..w {
                let c = bm.pixels[(row * w + col) as usize];
                if c != 0 {
                    self.put_pix(x + col, y + row, page, c);
                }
            }
        }
    }

    /// `XPutFontPBM`: opaque, with `color` added to every non-zero pixel.
    pub fn put_font_pbm(&mut self, x: i32, y: i32, color: u8, page: i32, bm: &Bitmap) {
        let w = bm.padded_width();

        for row in 0..bm.height {
            for col in 0..w {
                let c = bm.pixels[(row * w + col) as usize];
                let c = if c != 0 { c.wrapping_add(color) } else { 0 };
                self.put_pix(x + col, y + row, page, c);
            }
        }
    }

    /// `XCpVidRect`: copies a rectangle between pages (end exclusive).
    #[allow(clippy::too_many_arguments)]
    pub fn cp_vid_rect(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, dx: i32, dy: i32, src: i32, dst: i32) {
        for y in 0..(y2 - y1) {
            for x in 0..(x2 - x1) {
                let c = self.get_pix(x1 + x, y1 + y, src);
                self.put_pix(dx + x, dy + y, dst, c);
            }
        }
    }

    pub fn set_rgb(&mut self, color: u8, r: u8, g: u8, b: u8) {
        self.palette[color as usize] = [r & 63, g & 63, b & 63];
    }

    /// `XPutPalStruc`: `[first colour, count, r, g, b, ...]`.
    pub fn put_pal_struc(&mut self, pal: &[u8]) {
        let first = pal[0] as usize;
        let count = pal[1] as usize;

        for i in 0..count {
            let c = first + i;
            if c > 255 {
                break;
            }
            let o = 2 + i * 3;
            self.palette[c] = [pal[o] & 63, pal[o + 1] & 63, pal[o + 2] & 63];
        }
    }

    /// `XPutPalRaw`.
    pub fn put_pal_raw(&mut self, pal: &[[u8; 3]], count: usize, first: usize) {
        for i in 0..count {
            self.palette[first + i] = pal[i];
        }
    }

    /// Renders what the monitor would show as 320x240 RGBA pixels.
    pub fn render_rgba(&self, out: &mut [u8]) {
        let mut rgb = [[0u8; 4]; 256];
        for (i, p) in self.palette.iter().enumerate() {
            let conv = |v: u8| (v << 2) | (v >> 4);
            rgb[i] = [conv(p[0]), conv(p[1]), conv(p[2]), 255];
        }

        let split_line = if self.split_screen { SPLIT_SCREEN_LINE as usize } else { PHYSICAL_HEIGHT };

        for row in 0..PHYSICAL_HEIGHT {
            for col in 0..PHYSICAL_WIDTH {
                let c = if row < split_line {
                    self.get_pix(self.display_x + col as i32, self.display_y + row as i32, self.visible_page_offs)
                } else {
                    self.get_pix(col as i32, (row - split_line) as i32, self.split_offs)
                };

                let o = (row * PHYSICAL_WIDTH + col) * 4;
                out[o..o + 4].copy_from_slice(&rgb[c as usize]);
            }
        }
    }
}
