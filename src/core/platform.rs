//! The interface between the game logic and the host (Bevy, or a headless test harness).
//!
//! The game runs as a future which is polled once per video frame. Each time it waits for
//! the vertical retrace it renders the screen into [`Shared::frame`] and yields. Input
//! events that the host collected since the previous frame are then applied.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use crate::core::vga::{PHYSICAL_HEIGHT, PHYSICAL_WIDTH};

/// Video frames per second. Mode X 320x240 refreshes at 60 Hz.
pub const FRAMES_PER_SECOND: f64 = 60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEvent {
    /// A key identified by its PC scan code (set 1, with `$E0` prefixed keys plus 128).
    Key { scan: u8, pressed: bool },
    /// Mouse button 0 (left) or 1 (right).
    MouseButton { button: u8, pressed: bool },
}

pub struct Shared {
    /// The current screen as 320x240 RGBA pixels.
    pub frame: Vec<u8>,
    /// Incremented every time a frame is published.
    pub frame_serial: u64,

    pub events: Vec<InputEvent>,
    /// Mouse motion since the previous frame, in mickeys.
    pub mouse_dx: i32,
    pub mouse_dy: i32,

    /// True while the game wants relative mouse input (the cursor should be grabbed).
    pub wants_mouse_grab: bool,
    /// Set when the player chooses Quit.
    pub finished: bool,

    /// A summary of the game state, for tests and debugging.
    pub status: GameStatus,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GameStatus {
    pub total_level: i32,
    pub time_on_level: i64,
    pub score: i64,
    pub lives: i32,
    pub rand_seed: u32,
}

impl Default for Shared {
    fn default() -> Self {
        Self {
            frame: vec![0; PHYSICAL_WIDTH * PHYSICAL_HEIGHT * 4],
            frame_serial: 0,
            events: Vec::new(),
            mouse_dx: 0,
            mouse_dy: 0,
            wants_mouse_grab: false,
            finished: false,
            status: GameStatus::default(),
        }
    }
}

pub type SharedRef = Arc<Mutex<Shared>>;

/// A future that returns `Pending` once, which hands control back to the host.
pub struct YieldNow(bool);

impl YieldNow {
    pub fn new() -> Self {
        Self(false)
    }
}

impl Default for YieldNow {
    fn default() -> Self {
        Self::new()
    }
}

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            Poll::Pending
        }
    }
}

/// Runs a future one frame at a time.
pub struct FrameExecutor {
    future: Option<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

impl FrameExecutor {
    pub fn new(future: impl Future<Output = ()> + Send + 'static) -> Self {
        Self { future: Some(Box::pin(future)) }
    }

    /// Advances the game by one frame. Returns false once the game has ended.
    pub fn step(&mut self) -> bool {
        let Some(future) = self.future.as_mut() else {
            return false;
        };

        let mut cx = Context::from_waker(std::task::Waker::noop());

        if future.as_mut().poll(&mut cx).is_ready() {
            self.future = None;
            return false;
        }

        true
    }
}
