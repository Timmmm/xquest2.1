//! The original data files, embedded in the binary.

use crate::core::gfx::parse_pascal_bytes;

pub const GFX: &[u8] = include_bytes!("../../assets/original/xquest.gfx");
pub const ENM: &[u8] = include_bytes!("../../assets/original/xquest.enm");
pub const FNT: &[u8] = include_bytes!("../../assets/original/xquest.fnt");
pub const FNT2: &[u8] = include_bytes!("../../assets/original/xquest2.fnt");
pub const SND: &[u8] = include_bytes!("../../assets/original/xquest.snd");
pub const DEMO: &[u8] = include_bytes!("../../assets/original/xquest.dmo");
pub const TITLE: &[u8] = include_bytes!("../../assets/original/title.pbm");
pub const TITLE0: &[u8] = include_bytes!("../../assets/original/title0.pbm");
pub const STARTPIC: &[u8] = include_bytes!("../../assets/original/startpic.pbm");

const PALETTE_INC: &str = include_str!("../../assets/original/palette.inc");
const TITLEMAP_INC: &str = include_str!("../../assets/original/titlemap.inc");

/// `palette` from `palette.inc`: `[first, count, r, g, b, ...]`.
pub fn palette() -> Vec<u8> {
    parse_pascal_bytes(PALETTE_INC)
}

/// `titlepalette` from `titlemap.inc`.
pub fn title_palette() -> Vec<u8> {
    parse_pascal_bytes(TITLEMAP_INC)
}

/// `fontmap`: maps a character to an entry of `xquest.fnt` (0 means no glyph).
pub fn fontmap(c: u8) -> usize {
    match c {
        b' ' => 39,
        b'+' => 37,
        b'0'..=b'9' => (c - b'0') as usize + 1,
        b':' => 38,
        b'?' => 40,
        b'A'..=b'Z' => (c - b'A') as usize + 11,
        _ => 0,
    }
}
