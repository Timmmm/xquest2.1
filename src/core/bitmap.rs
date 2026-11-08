//! 8-bit palette-indexed bitmaps and collision masks.

/// A palette-indexed bitmap.
///
/// The VGA "Mode X" drawing routines work on columns of four pixels, so when a bitmap is
/// drawn or captured its width is rounded up to a multiple of four ([`Bitmap::padded_width`]);
/// the extra columns are treated as zero.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bitmap {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<u8>,
}

impl Bitmap {
    pub fn new(width: i32, height: i32, pixels: Vec<u8>) -> Self {
        assert_eq!(pixels.len(), (width * height) as usize);
        Self {
            width,
            height,
            pixels,
        }
    }

    /// The width rounded up to a whole number of 4-pixel columns.
    pub fn padded_width(&self) -> i32 {
        (self.width + 3) / 4 * 4
    }

    /// The pixel at `(x, y)`, which is zero in the padding columns.
    pub fn pixel(&self, x: i32, y: i32) -> u8 {
        if x < self.width {
            self.pixels[(y * self.width + x) as usize]
        } else {
            0
        }
    }

    /// One collision mask row per pixel row. Pixel 0 of a row is the most significant bit.
    pub fn mask(&self) -> Vec<u32> {
        (0..self.height)
            .map(|y| {
                (0..self.width.min(32))
                    .filter(|&x| self.pixel(x, y) != 0)
                    .fold(0u32, |m, x| m | (1 << (31 - x)))
            })
            .collect()
    }

    /// Splits a horizontal strip into `frames` equally sized bitmaps.
    pub fn split(&self, frames: usize) -> Vec<Bitmap> {
        let w = self.width / frames as i32;
        assert_eq!(
            w * frames as i32,
            self.width,
            "strip width is not a multiple of the frame count"
        );

        (0..frames as i32)
            .map(|f| {
                let pixels = (0..self.height)
                    .flat_map(|y| (0..w).map(move |x| (x, y)))
                    .map(|(x, y)| self.pixel(f * w + x, y))
                    .collect();
                Bitmap::new(w, self.height, pixels)
            })
            .collect()
    }
}

/// A bitmap together with its collision mask.
#[derive(Clone, Debug, Default)]
pub struct Sprite {
    pub bitmap: Bitmap,
    pub mask: Vec<u32>,
}

impl Sprite {
    pub fn new(bitmap: Bitmap) -> Self {
        let mask = bitmap.mask();
        Self { bitmap, mask }
    }

    pub fn width(&self) -> i32 {
        self.bitmap.width
    }

    pub fn height(&self) -> i32 {
        self.bitmap.height
    }
}

/// Pixel-exact collision between two masks whose bounding boxes overlap. The first mask
/// must be the leftmost one (`x1 <= x2`).
pub fn masks_collide(a: &[u32], ax: i32, ay: i32, b: &[u32], bx: i32, by: i32) -> bool {
    let shift = (bx - ax) as u32;
    let dy = by - ay;

    let (start_a, start_b) = if dy >= 0 { (dy, 0) } else { (0, -dy) };
    let rows = (a.len() as i32 - start_a).min(b.len() as i32 - start_b);

    (0..rows.max(0)).any(|r| {
        let ma = a[(start_a + r) as usize];
        let mb = b[(start_b + r) as usize];
        let shifted = if shift < 32 { ma << shift } else { 0 };
        shifted & mb != 0
    })
}
