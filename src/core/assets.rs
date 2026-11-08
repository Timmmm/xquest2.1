//! The game's graphics, sounds and built-in demo, embedded in the binary.
//!
//! Images are palette-indexed PNGs whose pixel values are used directly as VGA colour
//! indices; palettes are 16x16 PNG swatches. Sounds are 8-bit 11025 Hz WAV files. They were
//! converted from the original data files by `src/bin/convert_assets.rs`.

use std::collections::HashMap;
use std::io::Cursor;

use serde::{Deserialize, Serialize};

use crate::core::bitmap::{Bitmap, Sprite};
use crate::core::demo::Demo;
use crate::core::enemies::{EnemyKind, EnemyMissileKind};
use crate::core::game::PowerUpKind;
use crate::core::sound::Sound;
use crate::core::vga::{Palette, to_dac};

macro_rules! embed {
    ($($name:literal),* $(,)?) => {
        &[$(($name, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $name)) as &[u8])),*]
    };
}

static FILES: &[(&str, &[u8])] = embed![
    "palettes/game.png",
    "palettes/title.png",
    "pictures/title.png",
    "pictures/title_logo.png",
    "pictures/menu_logo.png",
    "sprites/ship.png",
    "sprites/ship_missile.png",
    "sprites/crystal.png",
    "sprites/mine.png",
    "sprites/smart_bomb.png",
    "sprites/enemy_mine.png",
    "sprites/attractor.png",
    "sprites/enemies/powercharge.png",
    "sprites/enemies/explosion.png",
    "sprites/enemies/grunger.png",
    "sprites/enemies/zippo.png",
    "sprites/enemies/zinger.png",
    "sprites/enemies/vince.png",
    "sprites/enemies/unused.png",
    "sprites/enemies/miner.png",
    "sprites/enemies/meeby.png",
    "sprites/enemies/retaliator.png",
    "sprites/enemies/terrier.png",
    "sprites/enemies/doinger.png",
    "sprites/enemies/snipe.png",
    "sprites/enemies/tribbler.png",
    "sprites/enemies/tribble.png",
    "sprites/enemies/buckshot.png",
    "sprites/enemies/cluster.png",
    "sprites/enemies/sticktight.png",
    "sprites/enemies/repulsor.png",
    "sprites/enemy_missiles/zinger.png",
    "sprites/enemy_missiles/buckshot.png",
    "sprites/enemy_missiles/retaliator.png",
    "sprites/enemy_missiles/doinger.png",
    "sprites/enemy_missiles/snipe.png",
    "sprites/enemy_missiles/cluster.png",
    "scenery/gate_left.png",
    "scenery/gate_right.png",
    "scenery/corner_top_left.png",
    "scenery/corner_top_right.png",
    "scenery/corner_bottom_right.png",
    "scenery/corner_bottom_left.png",
    "scenery/enemy_gate_left.png",
    "scenery/enemy_gate_right.png",
    "status/ship.png",
    "status/smart_bomb.png",
    "status/crystal.png",
    "status/powerups.png",
    "fonts/digits.png",
    "fonts/text.png",
    "fonts/comix.png",
    "fonts/comix.ron",
    "sounds/fire6.wav",
    "sounds/fire5.wav",
    "sounds/phew.wav",
    "sounds/fire4.wav",
    "sounds/fire.wav",
    "sounds/boing.wav",
    "sounds/squelch.wav",
    "sounds/woo_hoo.wav",
    "sounds/all_right.wav",
    "sounds/oh_yeah.wav",
    "sounds/get_crystal.wav",
    "sounds/explosion.wav",
    "sounds/explosion2.wav",
    "sounds/explosion3.wav",
    "sounds/retaliate.wav",
    "sounds/ow.wav",
    "sounds/countdown.wav",
    "sounds/gate.wav",
    "sounds/super_crystal_smash.wav",
    "sounds/bark.wav",
    "sounds/applause.wav",
    "sounds/enemy_enter.wav",
    "sounds/menu_click.wav",
    "sounds/doh.wav",
    "sounds/repulse.wav",
    "demo.ron",
];

pub fn file(name: &str) -> &'static [u8] {
    FILES
        .iter()
        .find(|(n, _)| *n == name)
        .unwrap_or_else(|| panic!("missing asset {name}"))
        .1
}

/// Characters of the status bar font, in the order of `fonts/text.png`.
pub const TEXT_FONT_CHARS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ+: ?";

/// One glyph of `fonts/comix.png`, in order.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GlyphInfo {
    pub char: char,
    pub width: i32,
}

/// Decodes a palette-indexed PNG to its colour indices and palette (8-bit RGB).
pub fn decode_png(data: &[u8]) -> (Bitmap, Vec<[u8; 3]>) {
    let mut decoder = png::Decoder::new(Cursor::new(data));
    decoder.set_transformations(png::Transformations::IDENTITY);
    let mut reader = decoder.read_info().expect("valid PNG");

    let info = reader.info();
    assert_eq!(info.color_type, png::ColorType::Indexed);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    let palette = info
        .palette
        .as_ref()
        .expect("PNG palette")
        .chunks(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();

    let mut pixels = vec![0; reader.output_buffer_size().expect("PNG size")];
    let frame = reader.next_frame(&mut pixels).expect("valid PNG");
    pixels.truncate(frame.buffer_size());

    (
        Bitmap::new(frame.width as i32, frame.height as i32, pixels),
        palette,
    )
}

fn bitmap(name: &str) -> Bitmap {
    decode_png(file(name)).0
}

fn strip(name: &str, frames: usize) -> Vec<Bitmap> {
    bitmap(name).split(frames)
}

/// Loads a palette swatch as DAC values.
pub fn palette(name: &str) -> Palette {
    let (_, colours) = decode_png(file(name));
    let mut p = [[0; 3]; 256];
    for (dst, src) in p.iter_mut().zip(colours) {
        *dst = src.map(to_dac);
    }
    p
}

/// Reads the samples of an 8-bit mono WAV file.
pub fn decode_wav(data: &[u8]) -> Vec<u8> {
    let mut pos = 12;

    while pos + 8 <= data.len() {
        let id = &data[pos..pos + 4];
        let len = u32::from_le_bytes(data[pos + 4..pos + 8].try_into().unwrap()) as usize;
        if id == b"data" {
            return data[pos + 8..pos + 8 + len].to_vec();
        }
        pos += 8 + len + (len & 1);
    }

    panic!("WAV file without data");
}

pub fn sounds() -> Vec<Vec<u8>> {
    Sound::ALL
        .iter()
        .map(|s| decode_wav(file(&format!("sounds/{}.wav", s.file_name()))))
        .collect()
}

pub fn builtin_demo() -> Demo {
    Demo::from_ron(std::str::from_utf8(file("demo.ron")).unwrap()).expect("valid built-in demo")
}

/// A proportional font whose glyphs are drawn tinted by the text colour.
#[derive(Default)]
pub struct Font {
    glyphs: HashMap<char, Bitmap>,
}

impl Font {
    pub fn glyph(&self, c: char) -> Option<&Bitmap> {
        self.glyphs.get(&c)
    }

    pub fn text_width(&self, s: &str) -> i32 {
        s.chars()
            .filter_map(|c| self.glyph(c))
            .map(|g| g.width)
            .sum()
    }
}

pub struct Graphics {
    pub game_palette: Palette,
    pub title_palette: Palette,
    pub title: Bitmap,
    pub title_logo: Bitmap,
    pub menu_logo: Bitmap,

    /// The ship pointing in 24 directions, clockwise from straight down.
    pub ship: Vec<Sprite>,
    pub ship_missile: Sprite,
    pub crystal: Sprite,
    pub mine: Sprite,
    pub smart_bomb: Sprite,
    pub enemy_mine: Sprite,
    pub attractor: Bitmap,
    /// Animation frames, indexed by [`EnemyKind::index`].
    pub enemies: Vec<Vec<Sprite>>,
    /// Indexed by [`EnemyMissileKind::index`].
    pub enemy_missiles: Vec<Sprite>,

    pub gate_left: Sprite,
    pub gate_right: Sprite,
    pub corner_top_left: Bitmap,
    pub corner_top_right: Bitmap,
    pub corner_bottom_right: Bitmap,
    pub corner_bottom_left: Bitmap,
    /// The doors enemies enter by: closed, then five animation frames.
    pub enemy_gate_left: Vec<Bitmap>,
    pub enemy_gate_right: Vec<Bitmap>,

    pub status_ship: Bitmap,
    pub status_smart_bomb: Bitmap,
    pub status_crystal: Bitmap,
    /// Indexed by [`PowerUpKind::index`].
    pub powerups: Vec<Bitmap>,

    pub digits: Vec<Bitmap>,
    /// The fixed width status bar font, see [`TEXT_FONT_CHARS`].
    pub text_font: HashMap<char, Bitmap>,
    /// The proportional font used by the menus.
    pub comix: Font,
}

impl Graphics {
    pub fn load() -> Self {
        let sprite = |name: &str| Sprite::new(bitmap(name));

        let comix_glyphs: Vec<GlyphInfo> =
            ron::from_str(std::str::from_utf8(file("fonts/comix.ron")).unwrap()).unwrap();
        let comix_strip = bitmap("fonts/comix.png");
        let mut x = 0;
        let mut comix = Font::default();
        for g in comix_glyphs {
            let pixels = (0..comix_strip.height)
                .flat_map(|row| (0..g.width).map(move |col| (col, row)))
                .map(|(col, row)| comix_strip.pixel(x + col, row))
                .collect();
            comix
                .glyphs
                .insert(g.char, Bitmap::new(g.width, comix_strip.height, pixels));
            x += g.width;
        }

        let text_font = TEXT_FONT_CHARS
            .chars()
            .zip(strip("fonts/text.png", TEXT_FONT_CHARS.len()))
            .collect::<HashMap<_, _>>();

        Self {
            game_palette: palette("palettes/game.png"),
            title_palette: palette("palettes/title.png"),
            title: bitmap("pictures/title.png"),
            title_logo: bitmap("pictures/title_logo.png"),
            menu_logo: bitmap("pictures/menu_logo.png"),

            ship: strip("sprites/ship.png", 24)
                .into_iter()
                .map(Sprite::new)
                .collect(),
            ship_missile: sprite("sprites/ship_missile.png"),
            crystal: sprite("sprites/crystal.png"),
            mine: sprite("sprites/mine.png"),
            smart_bomb: sprite("sprites/smart_bomb.png"),
            enemy_mine: sprite("sprites/enemy_mine.png"),
            attractor: bitmap("sprites/attractor.png"),
            enemies: EnemyKind::ALL
                .iter()
                .map(|k| {
                    let name = format!("sprites/enemies/{}.png", k.file_name());
                    strip(&name, k.info().frames)
                        .into_iter()
                        .map(Sprite::new)
                        .collect()
                })
                .collect(),
            enemy_missiles: EnemyMissileKind::ALL
                .iter()
                .map(|k| sprite(&format!("sprites/enemy_missiles/{}.png", k.file_name())))
                .collect(),

            gate_left: sprite("scenery/gate_left.png"),
            gate_right: sprite("scenery/gate_right.png"),
            corner_top_left: bitmap("scenery/corner_top_left.png"),
            corner_top_right: bitmap("scenery/corner_top_right.png"),
            corner_bottom_right: bitmap("scenery/corner_bottom_right.png"),
            corner_bottom_left: bitmap("scenery/corner_bottom_left.png"),
            enemy_gate_left: strip("scenery/enemy_gate_left.png", 6),
            enemy_gate_right: strip("scenery/enemy_gate_right.png", 6),

            status_ship: bitmap("status/ship.png"),
            status_smart_bomb: bitmap("status/smart_bomb.png"),
            status_crystal: bitmap("status/crystal.png"),
            powerups: strip("status/powerups.png", PowerUpKind::ALL.len()),

            digits: strip("fonts/digits.png", 10),
            text_font,
            comix,
        }
    }

    pub fn enemy(&self, kind: EnemyKind) -> &[Sprite] {
        &self.enemies[kind.index()]
    }

    pub fn enemy_missile(&self, kind: EnemyMissileKind) -> &Sprite {
        &self.enemy_missiles[kind.index()]
    }
}
