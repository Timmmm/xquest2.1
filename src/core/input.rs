//! Keyboard and mouse state, in the style of the DOS interrupt handlers the game was written
//! against: keys and buttons count presses until the game reads them.

use std::collections::{HashMap, HashSet};

pub use bevy::input::keyboard::KeyCode;
pub use bevy::input::mouse::MouseButton;

#[derive(Default)]
pub struct Keyboard {
    down: HashSet<KeyCode>,
    presses: HashMap<KeyCode, u32>,
    any_key_pressed: bool,
    /// The key most recently pressed or released.
    pub last_key: Option<KeyCode>,
    /// The character typed by the most recent key press, if any.
    pub last_char: Option<char>,
}

impl Keyboard {
    pub fn event(&mut self, key: KeyCode, pressed: bool, text: Option<char>) {
        if pressed {
            if self.down.insert(key) {
                *self.presses.entry(key).or_default() += 1;
            }
            self.any_key_pressed = true;
            self.last_char = text;
        } else {
            self.down.remove(&key);
        }
        self.last_key = Some(key);
    }

    pub fn is_down(&self, key: KeyCode) -> bool {
        self.down.contains(&key)
    }

    /// Whether `key` was pressed since the last call for that key.
    pub fn take_press(&mut self, key: KeyCode) -> bool {
        self.presses.remove(&key).is_some_and(|n| n > 0)
    }

    /// Whether any of `keys` was pressed since they were last checked.
    pub fn take_any_press(&mut self, keys: &[KeyCode]) -> bool {
        let mut pressed = false;
        for &key in keys {
            pressed |= self.take_press(key);
        }
        pressed
    }

    /// Whether any key was pressed since the last call.
    pub fn take_any_key(&mut self) -> bool {
        std::mem::take(&mut self.any_key_pressed)
    }

    pub fn clear_presses(&mut self) {
        self.presses.clear();
        self.any_key_pressed = false;
    }

    pub fn last_key_was(&self, key: KeyCode) -> bool {
        self.last_key == Some(key)
    }
}

/// The left and right mouse buttons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Buttons {
    pub left: bool,
    pub right: bool,
}

impl Buttons {
    pub fn contains(self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.left,
            MouseButton::Right => self.right,
            _ => false,
        }
    }

    pub fn any(self) -> bool {
        self.left || self.right
    }
}

#[derive(Default)]
pub struct Mouse {
    motion: (i32, i32),
    down: Buttons,
    pressed: Buttons,
}

impl Mouse {
    pub fn motion(&mut self, dx: i32, dy: i32) {
        self.motion.0 += dx;
        self.motion.1 += dy;
    }

    pub fn button_event(&mut self, button: MouseButton, pressed: bool) {
        let (down, was_pressed) = match button {
            MouseButton::Left => (&mut self.down.left, &mut self.pressed.left),
            MouseButton::Right => (&mut self.down.right, &mut self.pressed.right),
            _ => return,
        };

        if pressed && !*down {
            *was_pressed = true;
        }
        *down = pressed;
    }

    /// Motion since the last call.
    pub fn take_motion(&mut self) -> (i32, i32) {
        std::mem::take(&mut self.motion)
    }

    /// Buttons pressed since the last call.
    pub fn take_presses(&mut self) -> Buttons {
        std::mem::take(&mut self.pressed)
    }

    /// Buttons currently held down.
    pub fn buttons_down(&self) -> Buttons {
        self.down
    }

    pub fn reset(&mut self) {
        self.motion = (0, 0);
        self.pressed = Buttons::default();
    }
}
