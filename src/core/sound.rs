//! A port of the Sound Blaster mixer in `sbunit.pas`.
//!
//! Sounds are 8-bit unsigned samples at 11025 Hz. Up to `max_sound_effects` voices are summed
//! and converted back to 8 bits through the "compander" volume table.

use std::sync::{Arc, Mutex};

pub const MIX_SPEED: u32 = 11025;
pub const COMPILED_MAXIMUM_SOUND_EFFECTS: i32 = 8;
pub const COMP_SIZE: i32 = 255 * COMPILED_MAXIMUM_SOUND_EFFECTS;
pub const MAX_SOUND_VOLUME: i32 = 128;

#[derive(Clone, Copy, Default)]
struct Voice {
    sound: usize,
    length: usize,
    position: usize,
}

pub struct Mixer {
    sounds: Arc<Vec<Vec<u8>>>,
    /// One more slot than can be mixed, as in the original `AddSound`.
    voices: [Voice; COMPILED_MAXIMUM_SOUND_EFFECTS as usize + 1],
    pub max_sound_effects: i32,
    compander: Vec<u8>,
}

pub type MixerRef = Arc<Mutex<Mixer>>;

impl Mixer {
    pub fn new(sounds: Arc<Vec<Vec<u8>>>) -> Self {
        let mut m = Self {
            sounds,
            voices: [Voice::default(); COMPILED_MAXIMUM_SOUND_EFFECTS as usize + 1],
            max_sound_effects: 4,
            compander: vec![0; COMP_SIZE as usize],
        };
        m.set_volume(96);
        m
    }

    /// `SetSBVolume`: builds the compander table in 8.8 fixed point.
    pub fn set_volume(&mut self, volume: i32) {
        let step = volume * 2;
        let clamped = volume.min(MAX_SOUND_VOLUME);
        let mut acc: i32 = -(COMP_SIZE / 2) * clamped * 2 + 127 * 256 + 128;

        for entry in self.compander.iter_mut() {
            acc = acc.wrapping_add(step);
            *entry = if acc < 0 {
                0
            } else if acc >> 16 != 0 {
                255
            } else {
                (acc >> 8) as u8
            };
        }
    }

    /// `AddSound`: replaces the voice with the least remaining playing time.
    pub fn add_sound(&mut self, sound: usize) {
        if self.max_sound_effects <= 0 {
            return;
        }

        let mut best = 0;
        let mut least = i32::from(i16::MAX);

        for i in 0..=self.max_sound_effects as usize {
            let v = &self.voices[i];
            let remaining = (v.length as i32 - v.position as i32) as i16 as i32;
            if remaining < least {
                least = remaining;
                best = i;
            }
        }

        let length = self.sounds[sound].len();
        self.voices[best] = Voice { sound, length, position: 0 };
    }

    pub fn stop_all(&mut self) {
        for v in self.voices.iter_mut() {
            v.position = v.length;
        }
    }

    /// Produces the next unsigned 8-bit output sample.
    pub fn next_sample(&mut self) -> u8 {
        let mut mixed = 0i32;

        for i in 0..self.max_sound_effects.max(0) as usize {
            let v = &mut self.voices[i];
            if v.position < v.length {
                mixed += self.sounds[v.sound][v.position] as i32 - 128;
                v.position += 1;
            }
        }

        let index = (COMP_SIZE / 2 + mixed).clamp(0, COMP_SIZE - 1);
        self.compander[index as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compander_is_centred_and_scaled() {
        let m = Mixer::new(Arc::new(vec![Vec::new()]));

        // Silence maps to the middle of the 8-bit range.
        let centre = m.compander[(COMP_SIZE / 2) as usize];
        assert!((127..=128).contains(&centre));

        // At volume 96 a full scale sample is scaled by 3/4.
        let top = m.compander[(COMP_SIZE / 2 + 127) as usize] as i32;
        assert!((top - 223).abs() <= 1, "{top}");
    }

    #[test]
    fn voices_mix_and_finish() {
        let sounds = Arc::new(vec![Vec::new(), vec![255; 4], vec![0; 2]]);
        let mut m = Mixer::new(sounds);
        m.set_volume(128);

        m.add_sound(1);
        m.add_sound(2);
        let mixed: Vec<u8> = (0..5).map(|_| m.next_sample()).collect();

        // 127 + (-128) cancels out, then the first sound continues alone, then silence.
        assert_eq!(mixed[0], 127);
        assert_eq!(mixed[2], 255);
        assert_eq!(mixed[4], 128);
    }
}
