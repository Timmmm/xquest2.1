//! Bitmaps, collision masks and loaders for the original data files.

use crate::core::tp::real48;

/// A linear 8-bit bitmap whose rows are padded to a multiple of four pixels.
#[derive(Clone, Debug, Default)]
pub struct Bitmap {
    /// Width in pixels as stored in the data file.
    pub width: i32,
    /// Width in bytes per plane (`(width - 1) div 4 + 1`).
    pub bmwidth: i32,
    pub height: i32,
    pub pixels: Vec<u8>,
}

impl Bitmap {
    pub fn new(width: i32, bmwidth: i32, height: i32, pixels: Vec<u8>) -> Self {
        Self { width, bmwidth, height, pixels }
    }

    pub fn padded_width(&self) -> i32 {
        self.bmwidth * 4
    }

    /// The collision mask built by `makemask`. Pixel 0 of a row is the most significant bit.
    pub fn mask(&self) -> Vec<u32> {
        let w = self.padded_width();

        (0..self.height)
            .map(|row| {
                let mut m = 0u32;
                for col in 0..w.min(32) {
                    if self.pixels[(row * w + col) as usize] != 0 {
                        m |= 1 << (31 - col);
                    }
                }
                m
            })
            .collect()
    }
}

/// A bitmap together with its collision mask.
#[derive(Clone, Debug, Default)]
pub struct Sprite {
    pub bm: Bitmap,
    pub mask: Vec<u32>,
}

impl Sprite {
    fn new(bm: Bitmap) -> Self {
        let mask = bm.mask();
        Self { bm, mask }
    }
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn u8(&mut self) -> u8 {
        let v = self.data[self.pos];
        self.pos += 1;
        v
    }

    fn i16(&mut self) -> i32 {
        let v = i16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        v as i32
    }

    fn bytes(&mut self, n: usize) -> &'a [u8] {
        let v = &self.data[self.pos..self.pos + n];
        self.pos += n;
        v
    }

    fn eof(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// A bitmap from `xquest.gfx`: width, height, then rows padded to 4 pixels.
    fn bitmap(&mut self) -> Bitmap {
        let width = self.i16();
        let height = self.i16();
        let bmwidth = (width - 1) / 4 + 1;
        let pixels = self.bytes((bmwidth * 4 * height) as usize).to_vec();
        Bitmap::new(width, bmwidth, height, pixels)
    }
}

/// Converts a planar `.pbm` image (as written by `XGetPBM`) to a linear bitmap.
pub fn planar_pbm(data: &[u8]) -> Bitmap {
    let bmwidth = data[0] as i32;
    let height = data[1] as i32;
    let width = bmwidth * 4;
    let mut pixels = vec![0u8; (width * height) as usize];

    for plane in 0..4 {
        for row in 0..height {
            for b in 0..bmwidth {
                let src = 2 + (plane * height * bmwidth + row * bmwidth + b) as usize;
                pixels[(row * width + b * 4 + plane) as usize] = data[src];
            }
        }
    }

    Bitmap::new(width, bmwidth, height, pixels)
}

/// One record of `xquest.enm` (`enemykindtype`) plus its pictures.
#[derive(Clone, Debug, Default)]
pub struct EnemyKind {
    pub speed: i32,
    pub speed2: i32,
    pub curve: i32,
    pub curve2: i32,
    pub hits: i32,
    pub firetype: i32,
    pub score: i32,
    pub deathsound: i32,

    pub fires: bool,
    pub follows: bool,
    pub curves: bool,
    pub explodes: bool,
    pub laysmines: bool,
    pub shootback: bool,
    pub zoom: bool,
    pub maxspeed: bool,
    pub rebounds: bool,
    pub tribbles: bool,
    pub repulses: bool,

    pub fireprob: f64,
    pub changedir: f64,
    pub changecurve: f64,
    pub follow: f64,

    pub frames: Vec<Sprite>,

    pub width: i32,
    pub height: i32,
    pub bmwidth: i32,
    pub numframes: i32,
    pub framespeed: i32,
}

pub const MAX_ENEMY_KINDS: usize = 18;

pub fn load_enemy_kinds(data: &[u8]) -> Vec<EnemyKind> {
    let mut r = Reader::new(data);

    (0..=MAX_ENEMY_KINDS)
        .map(|_| {
            let mut k = EnemyKind {
                speed: r.i16(),
                speed2: r.i16(),
                curve: r.i16(),
                curve2: r.i16(),
                hits: r.i16(),
                firetype: r.i16(),
                score: r.i16(),
                deathsound: r.i16(),
                ..Default::default()
            };

            let b = r.bytes(11);
            k.fires = b[0] != 0;
            k.follows = b[1] != 0;
            k.curves = b[2] != 0;
            k.explodes = b[3] != 0;
            k.laysmines = b[4] != 0;
            k.shootback = b[5] != 0;
            k.zoom = b[6] != 0;
            k.maxspeed = b[7] != 0;
            k.rebounds = b[8] != 0;
            k.tribbles = b[9] != 0;
            k.repulses = b[10] != 0;

            k.fireprob = real48(r.bytes(6));
            k.changedir = real48(r.bytes(6));
            k.changecurve = real48(r.bytes(6));
            k.follow = real48(r.bytes(6));

            // Picture and mask pointers.
            r.bytes(48);

            k.width = r.i16();
            k.height = r.i16();
            k.bmwidth = r.i16();
            k.numframes = r.i16();
            k.framespeed = r.i16();
            k
        })
        .collect()
}

/// Enemy missile kinds (`emisskindtype`).
#[derive(Clone, Debug, Default)]
pub struct EMissKind {
    pub mspeed: i32,
    pub soundnum: i32,
    pub rebound: bool,
    pub firedirect: bool,
    pub sprite: Sprite,
}

/// Everything loaded from `xquest.gfx`, `xquest.fnt` and `xquest2.fnt`.
#[derive(Default)]
pub struct Graphics {
    pub ship: Vec<Sprite>,
    pub missile: Sprite,
    /// Crystal, mine and smart bomb.
    pub objects: [Sprite; 3],
    pub emine: Sprite,
    pub emiss: Vec<Sprite>,

    pub ship_pic: Bitmap,
    pub smart_pic: Bitmap,
    pub crystal_pic: Bitmap,
    pub powerups: Vec<Bitmap>,
    pub gates: [Sprite; 2],
    pub tl_corner: Bitmap,
    pub tr_corner: Bitmap,
    pub br_corner: Bitmap,
    pub bl_corner: Bitmap,
    pub l_enemy_gate: Vec<Bitmap>,
    pub r_enemy_gate: Vec<Bitmap>,
    pub attractor: Bitmap,
    pub small_font: Vec<Bitmap>,

    /// `font[1..40]` (index 0 unused).
    pub font: Vec<Bitmap>,
    /// `ComixFont`, indexed by character code.
    pub comix: Vec<Option<Bitmap>>,
}

pub const MAX_SHIP_PICS: usize = 24;

/// Loads `xquest.gfx`. Enemy frame counts come from the enemy kinds, which receive their
/// pictures and sizes.
pub fn load_graphics(gfx: &[u8], fnt: &[u8], fnt2: &[u8], kinds: &mut [EnemyKind]) -> Graphics {
    let mut r = Reader::new(gfx);
    let mut g = Graphics::default();

    for _ in 0..MAX_SHIP_PICS {
        g.ship.push(Sprite::new(r.bitmap()));
    }
    g.missile = Sprite::new(r.bitmap());

    for i in 0..3 {
        g.objects[i] = Sprite::new(r.bitmap());
    }
    g.emine = Sprite::new(r.bitmap());

    for k in kinds.iter_mut() {
        for _ in 0..=k.numframes {
            let bm = r.bitmap();
            k.width = bm.width;
            k.height = bm.height;
            k.bmwidth = bm.bmwidth;
            k.frames.push(Sprite::new(bm));
        }
    }

    for _ in 0..6 {
        g.emiss.push(Sprite::new(r.bitmap()));
    }

    g.ship_pic = r.bitmap();
    g.smart_pic = r.bitmap();
    g.crystal_pic = r.bitmap();
    for _ in 0..7 {
        g.powerups.push(r.bitmap());
    }
    for i in 0..2 {
        g.gates[i] = Sprite::new(r.bitmap());
    }
    g.tl_corner = r.bitmap();
    g.tr_corner = r.bitmap();
    g.br_corner = r.bitmap();
    g.bl_corner = r.bitmap();
    for _ in 0..6 {
        g.l_enemy_gate.push(r.bitmap());
    }
    for _ in 0..6 {
        g.r_enemy_gate.push(r.bitmap());
    }
    g.attractor = r.bitmap();

    for _ in 0..10 {
        g.small_font.push(r.bitmap());
    }

    // xquest.fnt: 40 entries of 116 bytes (word width, word height, 112 bytes of pixels).
    g.font.push(Bitmap::default());
    for i in 0..40 {
        let e = &fnt[i * 116..(i + 1) * 116];
        let width = e[0] as i32;
        let height = e[2] as i32;
        let pixels = e[4..4 + (width * height) as usize].to_vec();
        g.font.push(Bitmap::new(width, width / 4, height, pixels));
    }

    // xquest2.fnt: a character code, word width, word height, then the rows.
    g.comix = vec![None; 128];
    let mut r = Reader::new(fnt2);
    while !r.eof() {
        let c = r.u8() as usize;
        let width = r.i16();
        let height = r.i16();
        let bmwidth = (width - 1) / 4 + 1;
        let mut pixels = vec![0u8; (bmwidth * 4 * height) as usize];

        for row in 0..height {
            let src = r.bytes(width as usize);
            let o = (row * bmwidth * 4) as usize;
            pixels[o..o + width as usize].copy_from_slice(src);
        }

        if c < 128 {
            g.comix[c] = Some(Bitmap::new(width, bmwidth, height, pixels));
        }
    }

    g
}

/// The digitised sound effects from `xquest.snd` (8-bit unsigned, 11025 Hz).
pub fn load_sounds(data: &[u8]) -> Vec<Vec<u8>> {
    let mut r = Reader::new(data);
    let mut sounds = vec![Vec::new()];

    for _ in 0..25 {
        let len = r.i16() as u16 as usize;
        sounds.push(r.bytes(len).to_vec());
    }

    sounds
}

/// Extracts all the numbers after the `=` of a Pascal constant array in an include file.
pub fn parse_pascal_bytes(src: &str) -> Vec<u8> {
    let body = &src[src.find('=').unwrap() + 1..];
    let mut out = Vec::new();
    let mut in_comment = false;
    let mut num = String::new();

    for ch in body.chars() {
        if in_comment {
            if ch == '}' {
                in_comment = false;
            }
            continue;
        }
        if ch == '{' {
            in_comment = true;
            continue;
        }
        if ch.is_ascii_digit() {
            num.push(ch);
        } else if !num.is_empty() {
            out.push(num.parse::<u32>().unwrap() as u8);
            num.clear();
        }
    }

    out
}
