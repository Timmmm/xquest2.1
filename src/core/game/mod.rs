//! The game state and the main game loop.
//!
//! The game is written as an `async` function that awaits [`XQuest::vsync`] whenever it
//! waits for the next video frame, so it can keep the straightforward loop structure of the
//! original while being driven one frame at a time by the host.
//!
//! The rules (and their quirks) match the original exactly, including the order in which
//! random numbers are drawn, so recorded demos stay in sync. Positions and velocities are
//! 16-bit values in the original and some of its arithmetic overflows; [`wrap16`] marks the
//! places where that matters.

mod actors;
mod draw;
mod level;

use std::ops::{Index, IndexMut};
use std::sync::{Arc, Mutex};

use crate::core::assets::{self, Graphics};
use crate::core::bitmap::Bitmap;
use crate::core::demo::{Demo, DemoFrame};
use crate::core::enemies::{EnemyKind, EnemyMissileKind};
use crate::core::input::{Buttons, KeyCode, Keyboard, Mouse};
use crate::core::levels::{LEVELS, Level};
use crate::core::platform::{FRAMES_PER_SECOND, GameStatus, InputEvent, SharedRef, YieldNow};
use crate::core::rng::Rng;
use crate::core::rng::wrap16;
use crate::core::settings::{PlayerSettings, Settings};
use crate::core::sound::{Mixer, MixerRef, Sound};
use crate::core::stars::Stars;
use crate::core::vga::{Page, Vga};

// The playing area.
pub const PAGE_WIDTH: i32 = 392;
pub const PAGE_HEIGHT: i32 = 320;
pub const VIEW_WIDTH: i32 = 320;
pub const VIEW_HEIGHT: i32 = crate::core::vga::SPLIT_SCREEN_LINE;
pub const MAX_SCROLL_X: i32 = PAGE_WIDTH - VIEW_WIDTH;
pub const MAX_SCROLL_Y: i32 = PAGE_HEIGHT - VIEW_HEIGHT;
/// How close the ship can get to the edge of the view before it scrolls.
pub const SCROLL_BORDER_X: i32 = VIEW_WIDTH / 2 - 20;
pub const SCROLL_BORDER_Y: i32 = VIEW_HEIGHT / 2;

pub const SHIP_MIN_X: i32 = 10;
pub const SHIP_MAX_X: i32 = PAGE_WIDTH - 11;
pub const SHIP_MIN_Y: i32 = 10;
pub const SHIP_MAX_Y: i32 = PAGE_HEIGHT - 11;
pub const SHIP_START_X: i32 = PAGE_WIDTH / 2;
pub const SHIP_START_Y: i32 = PAGE_HEIGHT / 2;

pub const ENEMY_MIN_X: i32 = 10;
pub const ENEMY_MAX_X: i32 = PAGE_WIDTH - 10;
pub const ENEMY_MIN_Y: i32 = 10;
pub const ENEMY_MAX_Y: i32 = PAGE_HEIGHT - 10;
pub const ENEMY_START_Y: i32 = PAGE_HEIGHT / 2 - 5;

pub const MAX_MISSILES: usize = 50;
pub const MISSILE_LIFE: i32 = 300;
pub const MAX_ENEMY_MISSILES: usize = 70;
pub const MAX_ENEMY_MINES: usize = 60;
pub const MAX_ENEMIES: usize = 40;
pub const MAX_SHIP_SPEED: i32 = 640;
pub const BASE_GAME_SPEED: i32 = 64;

/// The frame rate the game's timings assume. The screen actually refreshes at 60 Hz, so
/// timers run slightly slow, as they did originally.
pub const NOMINAL_FRAME_RATE: i32 = 67;
/// Frames that the explosion of the ship takes before the next life.
pub const SHIP_DESTROYED_FRAMES: i32 = 64;

/// Ranks given each time all 50 levels are completed; each makes the game faster.
pub const RANKS: [&str; 5] = [
    "XQUEST WARRIOR",
    "XQUEST WARRIOR SUPREME",
    "XQUEST COMMANDER",
    "XQUEST WARLORD",
    "XQUEST GOD",
];
const RANK_SPEED_UP: [f64; 6] = [0.0, 0.2, 0.5, 1.0, 1.5, 2.0];

/// Colour 0 fades back from white to black after a smart bomb.
const SMART_BOMB_FLASH: [[u8; 3]; 11] = [
    [0, 0, 0],
    [8, 0, 0],
    [14, 0, 0],
    [20, 4, 0],
    [26, 12, 0],
    [32, 20, 10],
    [38, 28, 20],
    [44, 36, 30],
    [50, 44, 40],
    [56, 52, 50],
    [63, 63, 63],
];

/// Converts a time in seconds to video frames.
pub fn frames(seconds: f64) -> i64 {
    (seconds * FRAMES_PER_SECOND).round() as i64
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerUpKind {
    Shield,
    AimedFire,
    RapidFire,
    MultiFire,
    /// Also fires backwards.
    RearFire,
    /// Destroys any enemy with one hit.
    HeavyFire,
    /// Bullets bounce off the walls.
    Bounce,
}

impl PowerUpKind {
    pub const ALL: [PowerUpKind; 7] = [
        PowerUpKind::Shield,
        PowerUpKind::AimedFire,
        PowerUpKind::RapidFire,
        PowerUpKind::MultiFire,
        PowerUpKind::RearFire,
        PowerUpKind::HeavyFire,
        PowerUpKind::Bounce,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    /// Minimum and random extra duration in frames.
    fn duration(self) -> (i32, i32) {
        let seconds = match self {
            PowerUpKind::Shield => (10, 15),
            PowerUpKind::AimedFire | PowerUpKind::Bounce => (30, 60),
            _ => (60, 90),
        };
        (
            seconds.0 * NOMINAL_FRAME_RATE,
            seconds.1 * NOMINAL_FRAME_RATE,
        )
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PowerUp {
    /// Frames left; zero when the power-up is not active.
    pub remaining: i32,
    /// Position of its icon in the status bar, or zero if it is not shown.
    pub icon_x: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PowerUps([PowerUp; 7]);

impl PowerUps {
    pub fn active(&self, kind: PowerUpKind) -> bool {
        self[kind].remaining > 0
    }
}

impl Index<PowerUpKind> for PowerUps {
    type Output = PowerUp;

    fn index(&self, kind: PowerUpKind) -> &PowerUp {
        &self.0[kind.index()]
    }
}

impl IndexMut<PowerUpKind> for PowerUps {
    fn index_mut(&mut self, kind: PowerUpKind) -> &mut PowerUp {
        &mut self.0[kind.index()]
    }
}

/// A player's progress through the game.
#[derive(Clone, Copy, Debug, Default)]
pub struct Progress {
    /// The level being played, 1 to 50.
    pub level: i32,
    /// Levels played in total, which keeps counting after level 50.
    pub total_level: i32,
    pub smart_bombs: i32,
    pub lives: i32,
    /// How many times all 50 levels have been completed.
    pub rank: i32,
    pub score: i64,
    pub next_bonus_life_score: i64,
    pub last_bonus_life_score: i64,
    /// Frames spent on this level.
    pub time_on_level: i64,
    pub timing: bool,
    pub crystals: i32,
    pub mines: i32,
    pub smart_bomb_pickups: i32,
    pub gate_moving_right: bool,
    /// After the first rank the ship is pulled towards this point.
    pub attractor: (i32, i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ObjectKind {
    #[default]
    Crystal,
    Mine,
    SmartBomb,
}

/// Something lying in the playing area to collect or avoid.
#[derive(Clone, Debug, Default)]
pub struct Object {
    pub x: i32,
    pub y: i32,
    pub xbr: i32,
    pub ybr: i32,
    pub kind: ObjectKind,
    /// Hit this frame; it is removed when the sprites are next erased.
    pub hit: bool,
    pub background: Bitmap,
}

/// A rectangle given by its top left and bottom right corners (inclusive).
#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub xbr: i32,
    pub ybr: i32,
}

/// Position, velocity and background of something that moves. Positions are pixels and
/// `sx`/`sy` are the same positions in 1/64 pixels; `xbr`/`ybr` is the bottom right corner.
#[derive(Clone, Debug, Default)]
pub struct Body {
    pub x: i32,
    pub y: i32,
    pub xbr: i32,
    pub ybr: i32,
    pub sx: i32,
    pub sy: i32,
    pub dx: i32,
    pub dy: i32,
    pub old_x: i32,
    pub old_y: i32,
    /// What was under the sprite on this page and the other one.
    pub background: Bitmap,
    pub old_background: Bitmap,
}

#[derive(Clone, Debug, Default)]
pub struct Ship {
    pub body: Body,
    /// Index into the 24 ship pictures.
    pub direction: usize,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Missile {
    pub body: Body,
    pub age: i32,
}

#[derive(Clone, Debug)]
pub struct EnemyMissile {
    pub body: Body,
    pub kind: EnemyMissileKind,
}

#[derive(Clone, Debug)]
pub struct Enemy {
    pub body: Body,
    pub kind: EnemyKind,
    /// Curvature as the cosine and sine of the turn per frame, times 32767.
    pub curve_cos: i32,
    pub curve_sin: i32,
    pub hits_left: i32,
    /// Animation frame in 1/256 frames.
    pub frame: i32,
    /// Frames until a power charge or explosion disappears (zero for other enemies).
    pub lifetime: i32,
}

impl Enemy {
    pub fn frame_index(&self) -> usize {
        (self.frame >> 8) as usize
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Gate {
    pub x: i32,
    pub width: i32,
    pub height: i32,
}

/// The level timer shown at the top right.
#[derive(Clone, Debug, Default)]
pub struct Timer {
    pub x: i32,
    pub y: i32,
    pub old_x: i32,
    pub old_y: i32,
    pub background: Bitmap,
    pub old_background: Bitmap,
}

/// Which side an enemy enters from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// An enemy coming through one of the doors at the sides.
#[derive(Clone, Copy, Debug, Default)]
pub struct EnemyEntry {
    /// Frames until it appears; zero if none is entering.
    pub countdown: i32,
    pub kind: Option<EnemyKind>,
}

pub struct XQuest {
    pub vga: Vga,
    pub gfx: Graphics,
    pub rng: Rng,

    pub shared: SharedRef,
    pub mixer: MixerRef,
    pub keyboard: Keyboard,
    pub mouse: Mouse,
    /// Video frames since the start.
    pub frame_counter: i64,

    pub settings: Settings,
    pub progress: [Progress; 2],
    /// The player playing (0 or 1).
    pub player: usize,

    pub game_over: bool,
    pub level_finished: bool,
    pub ship_destroyed: bool,
    pub gate_moving: bool,
    pub no_mines: bool,

    /// Top left of the last window drawn.
    pub window_x: i32,
    pub window_y: i32,
    /// The screen under the message window.
    pub message_background: Option<(i32, i32, Bitmap)>,

    pub scroll_x: i32,
    pub scroll_y: i32,

    pub entering_left: EnemyEntry,
    pub entering_right: EnemyEntry,

    /// Frames since the start of this life or level.
    pub frame_count: i64,
    /// Gate movement in 1/64 pixels not yet applied.
    pub gate_move_count: i32,
    /// Enemy speed factor, 64 is normal.
    pub game_speed: i32,
    /// Steps left of the smart bomb screen flash.
    pub smart_bomb_flash: i32,
    /// Counts down from [`SHIP_DESTROYED_FRAMES`] while the ship explodes.
    pub ship_destroyed_count: i32,

    pub ship: Ship,
    pub enemies: Vec<Enemy>,
    pub missiles: Vec<Missile>,
    pub enemy_missiles: Vec<EnemyMissile>,
    pub enemy_mines: Vec<Rect>,
    /// The newest mine still needs drawing on the other page.
    pub enemy_mine_pending: bool,
    /// The current player's objects.
    pub objects: Vec<Object>,
    /// The other player's objects in a two player game.
    pub saved_objects: Vec<Object>,

    pub gates: [Gate; 2],
    pub timer: Timer,
    pub power_ups: PowerUps,
    /// Directions of the fragments of an exploding enemy, times 32767.
    pub fragment_directions: [(i32, i32); 15],

    pub demo_playing: bool,
    pub demo_recording: bool,
    pub demo: Demo,
    /// Frames of the demo played or recorded so far.
    pub demo_position: usize,
    pub demo_name: String,
    pub sounds_on: bool,
    pub show_timer: bool,
    pub stars: Stars,
}

impl XQuest {
    pub fn new(shared: SharedRef, mixer: MixerRef, seed: u32) -> Self {
        let gfx = Graphics::load();

        let ship = Ship {
            width: gfx.ship[0].width(),
            height: gfx.ship[0].height(),
            ..Default::default()
        };
        let gates = [&gfx.gate_left, &gfx.gate_right].map(|g| Gate {
            x: 0,
            width: g.width(),
            height: g.height(),
        });

        let fragment_directions = std::array::from_fn(|i| {
            let a = (i + 1) as f64 * 168.0 * std::f64::consts::PI / 180.0;
            (
                (a.cos() * 32767.0).round() as i32,
                (a.sin() * 32767.0).round() as i32,
            )
        });

        Self {
            vga: Vga::new(),
            gfx,
            rng: Rng::new(seed),
            shared,
            mixer,
            keyboard: Keyboard::default(),
            mouse: Mouse::default(),
            frame_counter: 0,
            settings: Settings::default(),
            progress: Default::default(),
            player: 0,
            game_over: false,
            level_finished: false,
            ship_destroyed: false,
            gate_moving: false,
            no_mines: false,
            window_x: 0,
            window_y: 0,
            message_background: None,
            scroll_x: 0,
            scroll_y: 0,
            entering_left: EnemyEntry::default(),
            entering_right: EnemyEntry::default(),
            frame_count: 0,
            gate_move_count: 0,
            game_speed: BASE_GAME_SPEED,
            smart_bomb_flash: 0,
            ship_destroyed_count: SHIP_DESTROYED_FRAMES,
            ship,
            enemies: Vec::new(),
            missiles: Vec::new(),
            enemy_missiles: Vec::new(),
            enemy_mines: Vec::new(),
            enemy_mine_pending: false,
            objects: Vec::new(),
            saved_objects: Vec::new(),
            gates,
            timer: Timer::default(),
            power_ups: PowerUps::default(),
            fragment_directions,
            demo_playing: false,
            demo_recording: false,
            demo: Demo::default(),
            demo_position: 0,
            demo_name: String::new(),
            sounds_on: true,
            show_timer: true,
            stars: Stars::default(),
        }
    }

    pub fn new_mixer() -> MixerRef {
        Arc::new(Mutex::new(Mixer::new(Arc::new(assets::sounds()))))
    }

    // ------------------------------------------------------------------------------------
    // The host.
    // ------------------------------------------------------------------------------------

    /// Waits for the next video frame: publishes the screen, gives control to the host and
    /// then applies the input it collected.
    pub async fn vsync(&mut self) {
        {
            let mut shared = self.shared.lock().unwrap();
            self.vga.render_rgba(&mut shared.frame);
            shared.frame_serial += 1;

            let p = &self.progress[self.player];
            shared.status = GameStatus {
                total_level: p.total_level,
                time_on_level: p.time_on_level,
                score: p.score,
                lives: p.lives,
                rand_seed: self.rng.seed,
            };
        }

        YieldNow::new().await;

        self.frame_counter += 1;

        let (events, motion) = {
            let mut shared = self.shared.lock().unwrap();
            (
                std::mem::take(&mut shared.events),
                std::mem::take(&mut shared.mouse_motion),
            )
        };

        for e in events {
            match e {
                InputEvent::Key { key, pressed, text } => self.keyboard.event(key, pressed, text),
                InputEvent::MouseButton { button, pressed } => {
                    self.mouse.button_event(button, pressed)
                }
            }
        }
        self.mouse.motion(motion.0, motion.1);
    }

    /// Shows the page just drawn and waits for the next frame.
    pub async fn page_flip(&mut self) {
        self.vga.page_flip(self.scroll_x, self.scroll_y);
        self.vsync().await;
    }

    pub fn set_mouse_grab(&mut self, grab: bool) {
        self.shared.lock().unwrap().wants_mouse_grab = grab;
    }

    pub fn play(&mut self, sound: Sound) {
        if self.sounds_on && self.settings.sound_enabled {
            self.mixer.lock().unwrap().play(sound);
        }
    }

    pub fn play_opt(&mut self, sound: Option<Sound>) {
        if let Some(s) = sound {
            self.play(s);
        }
    }

    // ------------------------------------------------------------------------------------
    // Accessors for the current player.
    // ------------------------------------------------------------------------------------

    pub fn progress(&mut self) -> &mut Progress {
        &mut self.progress[self.player]
    }

    pub fn player_settings(&self) -> &PlayerSettings {
        &self.settings.players[self.player]
    }

    pub fn level(&self) -> &'static Level {
        &LEVELS[(self.progress[self.player].level - 1) as usize]
    }

    fn base_game_speed(&self) -> i32 {
        let rank = self.progress[self.player].rank as usize;
        let factor = self.player_settings().difficulty.speed_factor() + RANK_SPEED_UP[rank];
        (BASE_GAME_SPEED as f64 * factor).round() as i32
    }

    // ------------------------------------------------------------------------------------
    // Input.
    // ------------------------------------------------------------------------------------

    /// Mouse buttons pressed since the last call.
    pub fn buttons_pressed(&mut self) -> Buttons {
        self.mouse.take_presses()
    }

    pub fn buttons_down(&self) -> Buttons {
        self.mouse.buttons_down()
    }

    /// Adds the player's mouse or keyboard input to a velocity. Mouse input is used if
    /// `always_mouse` is set or the player uses the mouse; keyboard input only if
    /// `allow_keyboard` is set and the player uses the keyboard.
    pub fn input_movement(
        &mut self,
        dx: &mut i32,
        dy: &mut i32,
        always_mouse: bool,
        allow_keyboard: bool,
    ) {
        use crate::core::settings::InputDevice;

        let p = *self.player_settings();

        if always_mouse || p.input_device == InputDevice::Mouse {
            let (mx, my) = self.mouse.take_motion();
            *dx = wrap16(*dx + mx * p.horizontal_speed / 64);
            *dy = wrap16(*dy + my * p.vertical_speed / 64);
        }

        if allow_keyboard && p.input_device == InputDevice::Keyboard {
            let k = &p.keys;
            let down = |keys: &[KeyCode]| keys.iter().any(|&key| self.keyboard.is_down(key));
            let step_x = 10 * p.horizontal_speed / 64;
            let step_y = 10 * p.vertical_speed / 64;

            if down(&[k.left, k.down_left, k.up_left]) {
                *dx -= step_x;
            }
            if down(&[k.right, k.down_right, k.up_right]) {
                *dx += step_x;
            }
            if down(&[k.up, k.up_left, k.up_right]) {
                *dy -= step_y;
            }
            if down(&[k.down, k.down_left, k.down_right]) {
                *dy += step_y;
            }
            if down(&[k.brake]) {
                *dx = (*dx as f64 / 1.2).round() as i32;
                *dy = (*dy as f64 / 1.2).round() as i32;
            }
        }
    }

    // ------------------------------------------------------------------------------------
    // Lives, pausing and the main game loop.
    // ------------------------------------------------------------------------------------

    pub fn start_game(&mut self) {
        self.player = 0;

        for p in self.progress.iter_mut() {
            *p = Progress {
                level: 1,
                total_level: 1,
                smart_bombs: 3,
                lives: 3,
                ..Default::default()
            };
        }

        self.game_over = false;
        self.smart_bomb_flash = 0;
        self.power_ups = PowerUps::default();
        self.ship_destroyed_count = SHIP_DESTROYED_FRAMES;
        self.entering_left = EnemyEntry::default();
        self.entering_right = EnemyEntry::default();
    }

    fn show_ready(&mut self) {
        let message = if self.settings.two_players {
            format!("PLAYER {} READY", self.player + 1)
        } else {
            "READY".to_string()
        };
        self.show_message(&message);
    }

    async fn new_life(&mut self) {
        self.progress().lives -= 1;

        if self.settings.two_players {
            let other = 1 - self.player;
            if self.progress[other].lives > 0 {
                self.player = other;
                std::mem::swap(&mut self.objects, &mut self.saved_objects);
            }
        }

        self.game_speed = self.base_game_speed();

        if self
            .progress
            .iter()
            .take(if self.settings.two_players { 2 } else { 1 })
            .all(|p| p.lives == 0)
        {
            self.game_over = true;
            return;
        }

        self.power_ups = PowerUps::default();
        self.ship_destroyed_count = SHIP_DESTROYED_FRAMES;
        self.start_level().await;
        self.show_ready();

        if self.demo_playing {
            self.wait_frames(frames(0.3)).await;
        } else {
            while !self.keyboard.take_any_key() && !self.buttons_pressed().any() {
                self.vsync().await;
            }
        }

        self.clear_input(false).await;
        self.remove_message();
        self.mouse.take_motion();
    }

    async fn pause(&mut self, message: &str) {
        self.progress().timing = false;
        self.show_message(message);
        self.wait_for_input(false).await;
        self.remove_message();
        self.progress().timing = true;
        self.mouse.take_motion();
    }

    async fn level_over(&mut self) {
        self.play(Sound::Phew);
        self.give_time_bonus().await;

        let p = self.progress();
        p.level += 1;
        p.total_level += 1;

        if p.level > LEVELS.len() as i32 {
            p.rank = (p.rank + 1).min(RANKS.len() as i32);
            self.show_rank().await;
            self.progress().level = 1;
        }

        self.set_up_level(self.player);
        self.start_level().await;
    }

    /// Plays a game, or the current demo if `demo_playing` is set.
    pub async fn game(&mut self) {
        self.stars.stop();
        self.start_game();
        self.rng.seed = crate::core::storage::random_seed();

        self.vga.set_double_buffer_mode(PAGE_HEIGHT);
        self.vga.palette = self.gfx.game_palette;

        if self.demo_playing || self.demo_recording {
            self.rng.seed = self.demo.seed;
        }
        self.demo_position = 0;
        self.set_mouse_grab(true);

        self.set_up_level(0);
        if self.settings.two_players {
            std::mem::swap(&mut self.objects, &mut self.saved_objects);
            self.set_up_level(1);
            std::mem::swap(&mut self.objects, &mut self.saved_objects);
        }

        self.start_level().await;
        self.show_ready();
        if self.demo_playing {
            self.wait_frames(frames(0.3)).await;
        } else {
            self.wait_for_input(false).await;
        }
        self.remove_message();

        while !self.game_over {
            self.progress().timing = true;
            self.frame_count = 0;

            while !(self.level_finished || self.ship_destroyed_count == 0 || self.game_over) {
                self.game_frame().await;
            }

            self.progress().timing = false;
            self.clear_enemies();

            if self.level_finished {
                self.level_over().await;
            } else if self.ship_destroyed {
                self.new_life().await;
            }
        }

        self.set_mouse_grab(false);
        self.game_over_screen().await;
    }

    async fn game_frame(&mut self) {
        self.erase_sprites();
        self.move_enemies();
        self.move_missiles();
        self.check_collisions();

        if self.ship_destroyed {
            if self.power_ups.active(PowerUpKind::Shield) {
                self.ship_destroyed = false;
            } else {
                let b = &mut self.ship.body;
                (b.old_x, b.old_y) = (b.x, b.y);

                if self.ship_destroyed_count == SHIP_DESTROYED_FRAMES - 1 {
                    self.play(Sound::Explosion);
                    let (x, y) = (self.ship.body.x, self.ship.body.y);
                    self.add_enemy(x, y, EnemyKind::Explosion);
                }
                self.ship_destroyed_count -= 1;
            }
        }

        if !self.ship_destroyed {
            self.move_ship();
        }

        if self.smart_bomb_flash > 1 && self.frame_count & 1 == 0 {
            self.smart_bomb_flash -= 1;
            self.vga
                .set_color(0, SMART_BOMB_FLASH[self.smart_bomb_flash as usize - 1]);
        }

        self.draw_sprites();

        if self.gate_moving {
            self.gate_move_count += self.level().gate_speed;
            if self.gate_move_count > 63 {
                self.move_gate();
            }
        }

        self.page_flip().await;
        self.frame_count += 1;
        if self.progress[self.player].timing {
            self.progress().time_on_level += 1;
        }

        if self.keyboard.take_any_key() {
            self.handle_game_keys().await;
        }

        let par = self.level().par_time as i64 * NOMINAL_FRAME_RATE as i64;
        let time = self.progress[self.player].time_on_level;
        if time > par {
            // Enemies speed up when the par time is exceeded, by 64 every 32 seconds.
            let extra = (time - par) * BASE_GAME_SPEED as i64 / (NOMINAL_FRAME_RATE as i64 * 32);
            self.game_speed = self.base_game_speed() + extra as i32;
        }
    }

    async fn handle_game_keys(&mut self) {
        if self.keyboard.take_press(KeyCode::Escape) && !self.demo_recording {
            self.clear_input(false).await;
            self.pause("QUIT?").await;
            self.game_over = self.keyboard.last_key_was(KeyCode::KeyY);
            if self.game_over {
                self.progress().score = 0;
            }
        }

        if self.keyboard.take_press(KeyCode::KeyP) {
            self.pause("PAUSE").await;
        }

        let volume_step =
            if self.keyboard.is_down(KeyCode::Equal) || self.keyboard.is_down(KeyCode::NumpadAdd) {
                4
            } else if self.keyboard.is_down(KeyCode::Minus)
                || self.keyboard.is_down(KeyCode::NumpadSubtract)
            {
                -4
            } else {
                0
            };
        if volume_step != 0 {
            self.settings.volume =
                (self.settings.volume + volume_step).clamp(0, crate::core::sound::MAX_VOLUME);
            self.mixer.lock().unwrap().set_volume(self.settings.volume);
        }

        if self.keyboard.take_press(KeyCode::KeyS) {
            self.sounds_on = !self.sounds_on;
        }
    }

    // ------------------------------------------------------------------------------------
    // Demos.
    // ------------------------------------------------------------------------------------

    /// The demo input for the current frame.
    fn demo_frame(&self) -> DemoFrame {
        self.demo
            .frames
            .get(self.demo_position.wrapping_sub(1))
            .copied()
            .unwrap_or_default()
    }

    pub async fn play_demo(&mut self, demo: Demo) {
        let saved_players = self.settings.players;
        let saved_two_players = self.settings.two_players;

        self.settings.players = demo.players;
        self.settings.two_players = demo.two_players;
        self.demo = demo;
        self.demo_playing = true;

        self.game().await;

        self.settings.players = saved_players;
        self.settings.two_players = saved_two_players;
    }
}
