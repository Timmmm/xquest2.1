//! The game itself. Bevy is only used for its input types; the host (`src/main.rs`, or the
//! headless runner) drives the game one frame at a time.

pub mod assets;
pub mod bitmap;
pub mod demo;
pub mod enemies;
pub mod game;
pub mod input;
pub mod levels;
pub mod platform;
pub mod rng;
pub mod settings;
pub mod sound;
pub mod stars;
pub mod storage;
pub mod ui;
pub mod vga;
