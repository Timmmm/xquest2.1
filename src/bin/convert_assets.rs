//! Converts the original XQuest data files in `xquest2_source` to the formats in `assets`:
//! palette-indexed PNGs, WAV sounds and RON. Run from the repository root:
//!
//!     cargo run --bin convert_assets
//!
//! It also checks that the enemy table in `src/core/enemies.rs` matches `xquest.enm`.

use std::fs;
use std::path::Path;

use xquest::core::assets::{GlyphInfo, TEXT_FONT_CHARS};
use xquest::core::bitmap::Bitmap;
use xquest::core::demo::{Demo, DemoFrame};
use xquest::core::enemies::{ENEMY_KINDS, EnemyKind, EnemyMissileKind};
use xquest::core::input::{KeyCode, MouseButton};
use xquest::core::settings::{Difficulty, InputDevice, KeyBindings, PlayerSettings};
use xquest::core::sound::{MIX_SPEED, Sound};

const SOURCE: &str = "xquest2_source";
const OUT: &str = "assets";

/// Reads little-endian values from an original data file.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn eof(&self) -> bool {
        self.pos >= self.data.len()
    }

    fn u8(&mut self) -> u8 {
        self.pos += 1;
        self.data[self.pos - 1]
    }

    fn i16(&mut self) -> i32 {
        let v = i16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        v as i32
    }

    fn bytes(&mut self, n: usize) -> &'a [u8] {
        self.pos += n;
        &self.data[self.pos - n..self.pos]
    }

    /// A bitmap from `xquest.gfx`: width and height, then rows padded to a multiple of 4.
    /// The padding is kept only if it contains pixels (some status bar icons do, and the
    /// game draws them).
    fn gfx_bitmap(&mut self) -> Bitmap {
        let width = self.i16();
        let height = self.i16();
        let padded = (width + 3) / 4 * 4;
        let data = self.bytes((padded * height) as usize);
        let padded_bitmap = Bitmap::new(padded, height, data.to_vec());

        let padding_used =
            (0..height).any(|y| (width..padded).any(|x| padded_bitmap.pixel(x, y) != 0));
        if padding_used {
            padded_bitmap
        } else {
            crop(&padded_bitmap, width)
        }
    }
}

fn crop(bm: &Bitmap, width: i32) -> Bitmap {
    let pixels = (0..bm.height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| bm.pixel(x, y))
        .collect();
    Bitmap::new(width, bm.height, pixels)
}

/// Joins bitmaps of equal height side by side. If any frame needed its padding, all frames
/// are padded so that they stay the same width.
fn join(frames: &[Bitmap]) -> Bitmap {
    let width = frames.iter().map(|f| f.width).max().unwrap();
    let frames: Vec<Bitmap> = frames.iter().map(|f| crop(f, width)).collect();
    let height = frames[0].height;
    let pixels = (0..height)
        .flat_map(|y| {
            frames
                .iter()
                .flat_map(move |f| (0..f.width).map(move |x| f.pixel(x, y)))
        })
        .collect();

    Bitmap::new(width * frames.len() as i32, height, pixels)
}

/// A planar Mode X image (`.pbm`): byte width and height, then each plane in turn.
fn planar_pbm(data: &[u8]) -> Bitmap {
    let byte_width = data[0] as i32;
    let height = data[1] as i32;
    let width = byte_width * 4;
    let mut pixels = vec![0u8; (width * height) as usize];

    for plane in 0..4 {
        for row in 0..height {
            for b in 0..byte_width {
                let src = 2 + (plane * height * byte_width + row * byte_width + b) as usize;
                pixels[(row * width + b * 4 + plane) as usize] = data[src];
            }
        }
    }

    Bitmap::new(width, height, pixels)
}

/// A palette from a Pascal include file: `[first, count, r, g, b, ...]` in 6-bit values.
/// Entry 255 is never loaded by the original, so it stays black.
fn pascal_palette(name: &str) -> Vec<[u8; 3]> {
    let src = fs::read_to_string(Path::new(SOURCE).join(name)).unwrap();
    let body = &src[src.find('=').unwrap() + 1..];
    let mut numbers = Vec::new();
    let mut in_comment = false;
    let mut num = String::new();

    for ch in body.chars() {
        match ch {
            '{' => in_comment = true,
            '}' => in_comment = false,
            c if !in_comment && c.is_ascii_digit() => num.push(c),
            _ if !num.is_empty() => numbers.push(std::mem::take(&mut num).parse::<u8>().unwrap()),
            _ => {}
        }
    }

    let count = numbers[1] as usize;
    let mut palette = vec![[0u8; 3]; 256];
    for i in 0..count {
        let c = &numbers[2 + i * 3..5 + i * 3];
        palette[i] = [c[0], c[1], c[2]].map(|v| (v << 2) | (v >> 4));
    }
    palette
}

fn write_png(name: &str, bm: &Bitmap, palette: &[[u8; 3]]) {
    let path = Path::new(OUT).join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();

    let file = fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(file),
        bm.width as u32,
        bm.height as u32,
    );
    encoder.set_color(png::ColorType::Indexed);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_palette(palette.concat());
    encoder.set_compression(png::Compression::High);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&bm.pixels)
        .unwrap();
}

fn write_wav(name: &str, samples: &[u8]) {
    let mut d = Vec::new();
    let data_len = samples.len() as u32;
    d.extend_from_slice(b"RIFF");
    d.extend_from_slice(&(36 + data_len + (data_len & 1)).to_le_bytes());
    d.extend_from_slice(b"WAVEfmt ");
    d.extend_from_slice(&16u32.to_le_bytes());
    d.extend_from_slice(&1u16.to_le_bytes()); // PCM
    d.extend_from_slice(&1u16.to_le_bytes()); // mono
    d.extend_from_slice(&MIX_SPEED.to_le_bytes());
    d.extend_from_slice(&MIX_SPEED.to_le_bytes()); // bytes per second
    d.extend_from_slice(&1u16.to_le_bytes()); // block align
    d.extend_from_slice(&8u16.to_le_bytes()); // bits per sample
    d.extend_from_slice(b"data");
    d.extend_from_slice(&data_len.to_le_bytes());
    d.extend_from_slice(samples);
    if data_len & 1 == 1 {
        d.push(0);
    }

    let path = Path::new(OUT).join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, d).unwrap();
}

/// A 16x16 swatch showing every palette entry.
fn swatch() -> Bitmap {
    Bitmap::new(16, 16, (0..=255).collect())
}

fn read(name: &str) -> Vec<u8> {
    fs::read(Path::new(SOURCE).join(name)).unwrap()
}

fn main() {
    let game_palette = pascal_palette("palette.inc");
    let title_palette = pascal_palette("titlemap.inc");

    write_png("palettes/game.png", &swatch(), &game_palette);
    write_png("palettes/title.png", &swatch(), &title_palette);

    let title = read("title.pbm");
    let (top, bottom) = title.split_at(title.len() / 2);
    let (top, bottom) = (planar_pbm(top), planar_pbm(bottom));
    let pixels = [top.pixels, bottom.pixels].concat();
    write_png(
        "pictures/title.png",
        &Bitmap::new(320, 240, pixels),
        &title_palette,
    );
    write_png(
        "pictures/title_logo.png",
        &planar_pbm(&read("title0.pbm")),
        &game_palette,
    );
    write_png(
        "pictures/menu_logo.png",
        &planar_pbm(&read("startpic.pbm")),
        &game_palette,
    );

    let enm = read("xquest.enm");
    check_enemy_table(&enm);

    let gfx = read("xquest.gfx");
    let mut r = Reader::new(&gfx);
    let png = |name: &str, bm: &Bitmap| write_png(name, bm, &game_palette);

    let ship: Vec<Bitmap> = (0..24).map(|_| r.gfx_bitmap()).collect();
    png("sprites/ship.png", &join(&ship));
    png("sprites/ship_missile.png", &r.gfx_bitmap());
    png("sprites/crystal.png", &r.gfx_bitmap());
    png("sprites/mine.png", &r.gfx_bitmap());
    png("sprites/smart_bomb.png", &r.gfx_bitmap());
    png("sprites/enemy_mine.png", &r.gfx_bitmap());

    for kind in EnemyKind::ALL {
        let frames: Vec<Bitmap> = (0..kind.info().frames).map(|_| r.gfx_bitmap()).collect();
        png(
            &format!("sprites/enemies/{}.png", kind.file_name()),
            &join(&frames),
        );
    }

    for kind in EnemyMissileKind::ALL {
        png(
            &format!("sprites/enemy_missiles/{}.png", kind.file_name()),
            &r.gfx_bitmap(),
        );
    }

    png("status/ship.png", &r.gfx_bitmap());
    png("status/smart_bomb.png", &r.gfx_bitmap());
    png("status/crystal.png", &r.gfx_bitmap());
    let powerups: Vec<Bitmap> = (0..7).map(|_| r.gfx_bitmap()).collect();
    png("status/powerups.png", &join(&powerups));

    png("scenery/gate_left.png", &r.gfx_bitmap());
    png("scenery/gate_right.png", &r.gfx_bitmap());
    png("scenery/corner_top_left.png", &r.gfx_bitmap());
    png("scenery/corner_top_right.png", &r.gfx_bitmap());
    png("scenery/corner_bottom_right.png", &r.gfx_bitmap());
    png("scenery/corner_bottom_left.png", &r.gfx_bitmap());
    let left: Vec<Bitmap> = (0..6).map(|_| r.gfx_bitmap()).collect();
    png("scenery/enemy_gate_left.png", &join(&left));
    let right: Vec<Bitmap> = (0..6).map(|_| r.gfx_bitmap()).collect();
    png("scenery/enemy_gate_right.png", &join(&right));
    png("sprites/attractor.png", &r.gfx_bitmap());

    let digits: Vec<Bitmap> = (0..10).map(|_| r.gfx_bitmap()).collect();
    png("fonts/digits.png", &join(&digits));
    assert!(r.eof(), "unexpected data at the end of xquest.gfx");

    // xquest.fnt: 40 glyphs of 116 bytes (word width, word height, pixels).
    let fnt = read("xquest.fnt");
    let glyphs: Vec<Bitmap> = (0..TEXT_FONT_CHARS.len())
        .map(|i| {
            let e = &fnt[i * 116..(i + 1) * 116];
            let (w, h) = (e[0] as i32, e[2] as i32);
            Bitmap::new(w, h, e[4..4 + (w * h) as usize].to_vec())
        })
        .collect();
    png("fonts/text.png", &join(&glyphs));

    // xquest2.fnt: a character, word width, word height, then the rows.
    let fnt2 = read("xquest2.fnt");
    let mut r = Reader::new(&fnt2);
    let mut info = Vec::new();
    let mut glyphs = Vec::new();
    while !r.eof() {
        let c = r.u8() as char;
        let (w, h) = (r.i16(), r.i16());
        glyphs.push(Bitmap::new(w, h, r.bytes((w * h) as usize).to_vec()));
        info.push(GlyphInfo { char: c, width: w });
    }
    let height = glyphs[0].height;
    let strip_pixels = (0..height)
        .flat_map(|y| {
            glyphs
                .iter()
                .flat_map(move |g| (0..g.width).map(move |x| g.pixel(x, y)))
        })
        .collect();
    let strip_width = glyphs.iter().map(|g| g.width).sum();
    png(
        "fonts/comix.png",
        &Bitmap::new(strip_width, height, strip_pixels),
    );
    fs::write(
        Path::new(OUT).join("fonts/comix.ron"),
        ron::to_string(&info).unwrap(),
    )
    .unwrap();

    // xquest.snd: each sound is a word length followed by the samples.
    let snd = read("xquest.snd");
    let mut r = Reader::new(&snd);
    for sound in Sound::ALL {
        let len = r.i16() as u16 as usize;
        write_wav(&format!("sounds/{}.wav", sound.file_name()), r.bytes(len));
    }

    fs::write(
        Path::new(OUT).join("demo.ron"),
        convert_demo(&read("xquest.dmo")).to_ron(),
    )
    .unwrap();
}

/// Checks the hand-written enemy table against the original `enemykindtype` records.
fn check_enemy_table(enm: &[u8]) {
    for (i, info) in ENEMY_KINDS.iter().enumerate() {
        let mut r = Reader::new(&enm[i * 109..(i + 1) * 109]);
        let ints: Vec<i32> = (0..8).map(|_| r.i16()).collect();
        assert_eq!(
            ints[..5],
            [
                info.speed,
                info.speed_offset,
                info.curve,
                info.curve_offset,
                info.hits
            ]
        );
        assert_eq!(ints[5], info.missile.map_or(0, |m| m.index() as i32 + 1));
        assert_eq!(ints[6], info.score);

        let flags = r.bytes(11);
        let expected = [
            info.fires,
            info.follows,
            info.curves,
            info.explodes,
            info.lays_mines,
            info.shoots_back,
            info.zooms,
            info.fixed_speed,
            info.rebounds,
            info.tribbles,
            info.repulses,
        ];
        assert!(
            flags.iter().zip(expected).all(|(&f, e)| (f != 0) == e),
            "flags of enemy {i}"
        );

        let probs: Vec<f64> = (0..4).map(|_| real48(r.bytes(6))).collect();
        let expected = [
            info.fire_prob,
            info.change_dir_prob,
            info.change_curve_prob,
            info.follow_prob,
        ];
        assert!(
            probs
                .iter()
                .zip(expected)
                .all(|(a, b)| (a - b).abs() < 1e-9),
            "probabilities of enemy {i}"
        );

        r.bytes(48);
        let tail: Vec<i32> = (0..5).map(|_| r.i16()).collect();
        assert_eq!(tail[3] as usize + 1, info.frames, "frames of enemy {i}");
        assert_eq!(tail[4], info.frame_speed, "frame speed of enemy {i}");
    }
}

/// Decodes a Turbo Pascal 6-byte `Real`.
fn real48(b: &[u8]) -> f64 {
    if b[0] == 0 {
        return 0.0;
    }

    let mantissa = b[1..6].iter().rev().fold(0u64, |m, &x| (m << 8) | x as u64);
    let value = (1.0 + (mantissa & ((1 << 39) - 1)) as f64 / (1u64 << 39) as f64)
        * 2f64.powi(b[0] as i32 - 129);
    if mantissa & (1 << 39) != 0 {
        -value
    } else {
        value
    }
}

/// Converts an original demo: seed, game mode, two `PlayerInfoType` records, then a
/// 5-byte record per frame.
fn convert_demo(d: &[u8]) -> Demo {
    let mut r = Reader::new(d);
    let seed = u32::from_le_bytes(r.bytes(4).try_into().unwrap());
    let two_players = r.u8() == 1;

    let mut player = || {
        let input_device = if r.u8() == 2 {
            InputDevice::Keyboard
        } else {
            InputDevice::Mouse
        };
        let horizontal_speed = r.i16();
        let vertical_speed = r.i16();
        let _joystick_buttons = (r.i16(), r.i16());
        let fire_button = mouse_button(r.i16());
        let smart_bomb_button = mouse_button(r.i16());
        let difficulty = Difficulty::ALL[r.i16() as usize];
        let keys: Vec<KeyCode> = (0..11).map(|_| key_code(r.i16() as u8)).collect();
        let keys = KeyBindings {
            up: keys[0],
            down: keys[1],
            left: keys[2],
            right: keys[3],
            up_left: keys[4],
            up_right: keys[5],
            down_left: keys[6],
            down_right: keys[7],
            brake: keys[8],
            fire: keys[9],
            smart_bomb: keys[10],
        };
        PlayerSettings {
            input_device,
            horizontal_speed,
            vertical_speed,
            fire_button,
            smart_bomb_button,
            difficulty,
            keys,
        }
    };
    let players = [player(), player()];

    let mut frames = Vec::new();
    while r.pos + 5 <= d.len() {
        frames.push(DemoFrame(r.i16() as i16, r.i16() as i16, r.u8()));
    }

    Demo {
        seed,
        two_players,
        players,
        frames,
    }
}

fn mouse_button(mask: i32) -> MouseButton {
    if mask & 2 != 0 {
        MouseButton::Right
    } else {
        MouseButton::Left
    }
}

/// Maps a PC scan code (with 128 added for `$E0` prefixed keys) to a key.
fn key_code(scan: u8) -> KeyCode {
    use KeyCode::*;

    match scan {
        0x01 => Escape,
        0x02 => Digit1,
        0x03 => Digit2,
        0x04 => Digit3,
        0x05 => Digit4,
        0x06 => Digit5,
        0x07 => Digit6,
        0x08 => Digit7,
        0x09 => Digit8,
        0x0A => Digit9,
        0x0B => Digit0,
        0x0C => Minus,
        0x0D => Equal,
        0x0E => Backspace,
        0x0F => Tab,
        0x10 => KeyQ,
        0x11 => KeyW,
        0x12 => KeyE,
        0x13 => KeyR,
        0x14 => KeyT,
        0x15 => KeyY,
        0x16 => KeyU,
        0x17 => KeyI,
        0x18 => KeyO,
        0x19 => KeyP,
        0x1A => BracketLeft,
        0x1B => BracketRight,
        0x1C => Enter,
        0x1D => ControlLeft,
        0x1E => KeyA,
        0x1F => KeyS,
        0x20 => KeyD,
        0x21 => KeyF,
        0x22 => KeyG,
        0x23 => KeyH,
        0x24 => KeyJ,
        0x25 => KeyK,
        0x26 => KeyL,
        0x27 => Semicolon,
        0x28 => Quote,
        0x29 => Backquote,
        0x2A => ShiftLeft,
        0x2B => Backslash,
        0x2C => KeyZ,
        0x2D => KeyX,
        0x2E => KeyC,
        0x2F => KeyV,
        0x30 => KeyB,
        0x31 => KeyN,
        0x32 => KeyM,
        0x33 => Comma,
        0x34 => Period,
        0x35 => Slash,
        0x36 => ShiftRight,
        0x37 => NumpadMultiply,
        0x38 => AltLeft,
        0x39 => Space,
        0x3A => CapsLock,
        0x3B => F1,
        0x3C => F2,
        0x3D => F3,
        0x3E => F4,
        0x3F => F5,
        0x40 => F6,
        0x41 => F7,
        0x42 => F8,
        0x43 => F9,
        0x44 => F10,
        0x45 => NumLock,
        0x46 => ScrollLock,
        0x47 => Numpad7,
        0x48 => Numpad8,
        0x49 => Numpad9,
        0x4A => NumpadSubtract,
        0x4B => Numpad4,
        0x4C => Numpad5,
        0x4D => Numpad6,
        0x4E => NumpadAdd,
        0x4F => Numpad1,
        0x50 => Numpad2,
        0x51 => Numpad3,
        0x52 => Numpad0,
        0x53 => NumpadDecimal,
        0x57 => F11,
        0x58 => F12,
        0x9C => NumpadEnter,
        0x9D => ControlRight,
        0xB5 => NumpadDivide,
        0xB8 => AltRight,
        0xC7 => Home,
        0xC8 => ArrowUp,
        0xC9 => PageUp,
        0xCB => ArrowLeft,
        0xCD => ArrowRight,
        0xCF => End,
        0xD0 => ArrowDown,
        0xD1 => PageDown,
        0xD2 => Insert,
        0xD3 => Delete,
        _ => panic!("unexpected scan code {scan:#x} in demo"),
    }
}
