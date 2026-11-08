//! Sound effects and the mixer, which works like the original's Sound Blaster mixer.
//!
//! Sounds are 8-bit unsigned samples at 11025 Hz. Up to `max_voices` voices are summed
//! and converted back to 8 bits through a volume ("volume_table") table.

use std::sync::{Arc, Mutex};

pub const MIX_SPEED: u32 = 11025;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    Fire6,
    Fire5,
    Phew,
    Fire4,
    Fire,
    Boing,
    Squelch,
    WooHoo,
    AllRight,
    OhYeah,
    GetCrystal,
    Explosion,
    Explosion2,
    Explosion3,
    Retaliate,
    Ow,
    Countdown,
    Gate,
    SuperCrystalSmash,
    Bark,
    Applause,
    EnemyEnter,
    MenuClick,
    Doh,
    Repulse,
}

impl Sound {
    pub const ALL: [Sound; 25] = [
        Sound::Fire6,
        Sound::Fire5,
        Sound::Phew,
        Sound::Fire4,
        Sound::Fire,
        Sound::Boing,
        Sound::Squelch,
        Sound::WooHoo,
        Sound::AllRight,
        Sound::OhYeah,
        Sound::GetCrystal,
        Sound::Explosion,
        Sound::Explosion2,
        Sound::Explosion3,
        Sound::Retaliate,
        Sound::Ow,
        Sound::Countdown,
        Sound::Gate,
        Sound::SuperCrystalSmash,
        Sound::Bark,
        Sound::Applause,
        Sound::EnemyEnter,
        Sound::MenuClick,
        Sound::Doh,
        Sound::Repulse,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    /// One of the three explosion sounds.
    pub fn explosion(variant: i32) -> Sound {
        [Sound::Explosion, Sound::Explosion2, Sound::Explosion3][variant as usize]
    }

    pub fn file_name(self) -> String {
        let name = format!("{self:?}");
        let mut out = String::new();
        for (i, c) in name.chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        }
        out
    }
}

pub const MAX_VOICES: i32 = 8;
pub const VOLUME_TABLE_SIZE: i32 = 255 * MAX_VOICES;
pub const MAX_VOLUME: i32 = 128;

#[derive(Clone, Copy, Default)]
struct Voice {
    sound: Option<Sound>,
    length: usize,
    position: usize,
}

pub struct Mixer {
    /// Sample data indexed by [`Sound::index`].
    sounds: Arc<Vec<Vec<u8>>>,
    /// One more voice than is mixed (see [`Mixer::play`]).
    voices: [Voice; MAX_VOICES as usize + 1],
    pub max_voices: i32,
    volume_table: Vec<u8>,
}

pub type MixerRef = Arc<Mutex<Mixer>>;

impl Mixer {
    pub fn new(sounds: Arc<Vec<Vec<u8>>>) -> Self {
        let mut m = Self {
            sounds,
            voices: [Voice::default(); MAX_VOICES as usize + 1],
            max_voices: 4,
            volume_table: vec![0; VOLUME_TABLE_SIZE as usize],
        };
        m.set_volume(96);
        m
    }

    /// Builds the volume table in 8.8 fixed point (volume 0 to 128).
    pub fn set_volume(&mut self, volume: i32) {
        let step = volume * 2;
        let clamped = volume.min(MAX_VOLUME);
        let mut acc: i32 = -(VOLUME_TABLE_SIZE / 2) * clamped * 2 + 127 * 256 + 128;

        for entry in self.volume_table.iter_mut() {
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

    /// Starts a sound on the voice with the least remaining playing time. As in the
    /// original, one more voice is considered than is mixed, so a sound can occasionally be
    /// silently dropped.
    pub fn play(&mut self, sound: Sound) {
        if self.max_voices <= 0 {
            return;
        }

        let mut best = 0;
        let mut least = i32::from(i16::MAX);

        for i in 0..=self.max_voices as usize {
            let v = &self.voices[i];
            let remaining = (v.length as i32 - v.position as i32) as i16 as i32;
            if remaining < least {
                least = remaining;
                best = i;
            }
        }

        let length = self.sounds[sound.index()].len();
        self.voices[best] = Voice {
            sound: Some(sound),
            length,
            position: 0,
        };
    }

    pub fn stop_all(&mut self) {
        for v in self.voices.iter_mut() {
            v.position = v.length;
        }
    }

    /// Produces the next unsigned 8-bit output sample.
    pub fn next_sample(&mut self) -> u8 {
        let mut mixed = 0i32;

        for i in 0..self.max_voices.max(0) as usize {
            let v = &mut self.voices[i];
            if let Some(sound) = v.sound.filter(|_| v.position < v.length) {
                mixed += self.sounds[sound.index()][v.position] as i32 - 128;
                v.position += 1;
            }
        }

        let index = (VOLUME_TABLE_SIZE / 2 + mixed).clamp(0, VOLUME_TABLE_SIZE - 1);
        self.volume_table[index as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_table_is_centred_and_scaled() {
        let m = Mixer::new(Arc::new(Vec::new()));

        // Silence maps to the middle of the 8-bit range.
        let centre = m.volume_table[(VOLUME_TABLE_SIZE / 2) as usize];
        assert!((127..=128).contains(&centre));

        // At volume 96 a full scale sample is scaled by 3/4.
        let top = m.volume_table[(VOLUME_TABLE_SIZE / 2 + 127) as usize] as i32;
        assert!((top - 223).abs() <= 1, "{top}");
    }

    #[test]
    fn voices_mix_and_finish() {
        let sounds = Arc::new(vec![vec![255; 4], vec![0; 2]]);
        let mut m = Mixer::new(sounds);
        m.set_volume(128);

        m.play(Sound::Fire6);
        m.play(Sound::Fire5);
        let mixed: Vec<u8> = (0..5).map(|_| m.next_sample()).collect();

        // 127 + (-128) cancels out, then the first sound continues alone, then silence.
        assert_eq!(mixed[0], 127);
        assert_eq!(mixed[2], 255);
        assert_eq!(mixed[4], 128);
    }
}
