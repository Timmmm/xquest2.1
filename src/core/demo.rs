//! Recorded demos: the random seed, the players' settings and the ship's input each frame.

use serde::{Deserialize, Serialize};

use crate::core::settings::PlayerSettings;

/// The ship's velocity after the player's input, and the buttons used, for one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemoFrame(pub i16, pub i16, pub u8);

impl DemoFrame {
    const FIRE_PRESSED: u8 = 1;
    const SMART_BOMB_PRESSED: u8 = 2;
    const FIRE_HELD: u8 = 4;

    pub fn new(dx: i32, dy: i32) -> Self {
        Self(dx as i16, dy as i16, 0)
    }

    pub fn dx(self) -> i32 {
        self.0 as i32
    }

    pub fn dy(self) -> i32 {
        self.1 as i32
    }

    pub fn fire_pressed(self) -> bool {
        self.2 & Self::FIRE_PRESSED != 0
    }

    pub fn smart_bomb_pressed(self) -> bool {
        self.2 & Self::SMART_BOMB_PRESSED != 0
    }

    pub fn fire_held(self) -> bool {
        self.2 & Self::FIRE_HELD != 0
    }

    pub fn set_fire_pressed(&mut self) {
        self.2 |= Self::FIRE_PRESSED;
    }

    pub fn set_smart_bomb_pressed(&mut self) {
        self.2 |= Self::SMART_BOMB_PRESSED;
    }

    pub fn set_fire_held(&mut self) {
        self.2 |= Self::FIRE_HELD;
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Demo {
    pub seed: u32,
    pub two_players: bool,
    pub players: [PlayerSettings; 2],
    pub frames: Vec<DemoFrame>,
}

impl Demo {
    pub fn from_ron(text: &str) -> Option<Self> {
        ron::from_str(text).ok()
    }

    /// Arrays are written on one line to keep files small.
    pub fn to_ron(&self) -> String {
        let config = ron::ser::PrettyConfig::default().compact_arrays(true);
        ron::ser::to_string_pretty(self, config).expect("serialisable")
    }
}
