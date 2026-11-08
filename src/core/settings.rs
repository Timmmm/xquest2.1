//! Player settings, the saved configuration and the high score tables.

use serde::{Deserialize, Serialize};

use crate::core::input::{KeyCode, MouseButton};
use crate::core::storage;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Difficulty {
    Wimp,
    Timid,
    Average,
    Tricky,
    Inhuman,
}

impl Difficulty {
    pub const ALL: [Difficulty; 5] = [
        Difficulty::Wimp,
        Difficulty::Timid,
        Difficulty::Average,
        Difficulty::Tricky,
        Difficulty::Inhuman,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn next(self) -> Difficulty {
        Difficulty::ALL[(self.index() + 1) % Difficulty::ALL.len()]
    }

    pub fn name(self) -> &'static str {
        ["Wimp", "Timid", "Average", "Tricky", "Inhuman"][self.index()]
    }

    /// Multiplier for the speed of enemies and their missiles.
    pub fn speed_factor(self) -> f64 {
        [0.7, 1.0, 1.0, 1.5, 2.0][self.index()]
    }

    /// Multiplier for the rate at which enemies appear.
    pub fn enemy_frequency(self) -> f64 {
        [0.7, 1.0, 1.0, 1.2, 1.5][self.index()]
    }

    /// Whether the ship bounces off the walls rather than being destroyed.
    pub fn walls_are_safe(self) -> bool {
        self <= Difficulty::Timid
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputDevice {
    Mouse,
    Keyboard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyBindings {
    pub up: KeyCode,
    pub down: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
    pub up_left: KeyCode,
    pub up_right: KeyCode,
    pub down_left: KeyCode,
    pub down_right: KeyCode,
    pub brake: KeyCode,
    pub fire: KeyCode,
    pub smart_bomb: KeyCode,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            up: KeyCode::Numpad8,
            down: KeyCode::Numpad2,
            left: KeyCode::Numpad4,
            right: KeyCode::Numpad6,
            up_left: KeyCode::Numpad7,
            up_right: KeyCode::Numpad9,
            down_left: KeyCode::Numpad1,
            down_right: KeyCode::Numpad3,
            brake: KeyCode::Numpad5,
            fire: KeyCode::Enter,
            smart_bomb: KeyCode::Space,
        }
    }
}

impl KeyBindings {
    /// The bindings in the order the set up screen asks for them, with their names.
    pub fn named_mut(&mut self) -> [(&'static str, &mut KeyCode); 11] {
        [
            ("Up", &mut self.up),
            ("Down", &mut self.down),
            ("Left", &mut self.left),
            ("Right", &mut self.right),
            ("Up Left", &mut self.up_left),
            ("Up Right", &mut self.up_right),
            ("Down Left", &mut self.down_left),
            ("Down Right", &mut self.down_right),
            ("Brake", &mut self.brake),
            ("Fire", &mut self.fire),
            ("Smart Bomb", &mut self.smart_bomb),
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerSettings {
    pub input_device: InputDevice,
    /// Mouse or keyboard sensitivity; 64 is normal.
    pub horizontal_speed: i32,
    pub vertical_speed: i32,
    pub fire_button: MouseButton,
    pub smart_bomb_button: MouseButton,
    pub difficulty: Difficulty,
    pub keys: KeyBindings,
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            input_device: InputDevice::Mouse,
            horizontal_speed: 64,
            vertical_speed: 64,
            fire_button: MouseButton::Left,
            smart_bomb_button: MouseButton::Right,
            difficulty: Difficulty::Average,
            keys: KeyBindings::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub two_players: bool,
    pub sound_enabled: bool,
    /// 0 to 128.
    pub volume: i32,
    /// The number of sounds that can play at once.
    pub max_sounds: i32,
    pub players: [PlayerSettings; 2],
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            two_players: false,
            sound_enabled: true,
            volume: 96,
            max_sounds: 4,
            players: Default::default(),
        }
    }
}

const SETTINGS_FILE: &str = "xquest.ron";
const SCORES_FILE: &str = "scores.ron";

impl Settings {
    pub fn load() -> Self {
        storage::load(SETTINGS_FILE)
            .and_then(|s| ron::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        storage::save(SETTINGS_FILE, &to_ron(self));
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Score {
    pub score: i64,
    pub level: i32,
    pub name: String,
}

pub const HIGH_SCORES_PER_TABLE: usize = 10;

/// A top ten for each difficulty.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HighScores(pub [Vec<Score>; 5]);

impl Default for HighScores {
    fn default() -> Self {
        let table = vec![
            Score {
                score: 0,
                level: 1,
                name: "XQuest".to_string()
            };
            HIGH_SCORES_PER_TABLE
        ];
        Self(std::array::from_fn(|_| table.clone()))
    }
}

impl HighScores {
    pub fn load() -> Self {
        storage::load(SCORES_FILE)
            .and_then(|s| ron::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        storage::save(SCORES_FILE, &to_ron(self));
    }

    pub fn table_mut(&mut self, difficulty: Difficulty) -> &mut Vec<Score> {
        &mut self.0[difficulty.index()]
    }
}

pub fn to_ron<T: Serialize>(value: &T) -> String {
    ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default()).expect("serialisable")
}
