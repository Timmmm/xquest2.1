//! The game state and the in-game logic, ported from `xquest.pas`.
//!
//! Procedures keep the names and structure of the original so the two can be compared side
//! by side. Variables that are 16 bit `integer`s in the original are stored as `i32` and
//! truncated with [`w16`] wherever the original arithmetic can overflow.

use std::sync::Arc;

use crate::core::data;
use crate::core::gfx::{self, Bitmap, EMissKind, EnemyKind, Graphics};
use crate::core::input::{Keyboard, Mouse, scan};
use crate::core::platform::{SharedRef, YieldNow};
use crate::core::sound::{Mixer, MixerRef};
use crate::core::storage;
use crate::core::tables::{LEVELS, Level, MAX_LEVEL, PROBS};
use crate::core::tp::{TpRandom, round, shl32, str_w, w16};
use crate::core::vga::Vga;

pub const MAX_MISSILES: usize = 50;
pub const MISSILE_LIFE: i32 = 300;
pub const MAX_OBJECTS: usize = 65;
pub const TICKS: i64 = 180;
pub const MAX_ENEMY_MISSILES: usize = 70;
pub const MAX_ENEMY_MINES: usize = 60;
pub const MAX_ENEMIES: usize = 40;
pub const MAX_GAME_CLOCKED: i32 = 5;
pub const MAX_SHIP_SPEED: i32 = 640;
pub const BASE_GAME_SPEED: i32 = 64;
pub const FRAME_RATE: i32 = 67;
pub const MAX_DEMO_FRAMES: usize = 13100;
pub const START_SHIP_DESTROYED_COUNT: i32 = 64;
pub const MAX_DIFF_LEVEL: i32 = 5;

pub const PAGE_HEIGHT: i32 = 320;
pub const PAGE_WIDTH: i32 = 392;
pub const PHYSICAL_PAGE_WIDTH: i32 = 320;
pub const SPLIT_SCREEN_LINE: i32 = 217;
pub const MAX_VISIBLE_PAGE_Y: i32 = PAGE_HEIGHT - SPLIT_SCREEN_LINE;
pub const MAX_VISIBLE_PAGE_X: i32 = PAGE_WIDTH - PHYSICAL_PAGE_WIDTH;
pub const SCREEN_V_BORDER: i32 = SPLIT_SCREEN_LINE / 2;
pub const SCREEN_H_BORDER: i32 = PHYSICAL_PAGE_WIDTH / 2 - 20;

pub const MIN_GATE_X: i32 = 20;
pub const MAX_GATE_X: i32 = PAGE_WIDTH - 30;
pub const TIMER_X: i32 = 280;
pub const TIMER_Y: i32 = 10;

pub const MINE_X_MIN: i32 = 11;
pub const MINE_X_MAX: i32 = PAGE_WIDTH - 21;
pub const MINE_Y_MIN: i32 = 14;
pub const MINE_Y_MAX: i32 = PAGE_HEIGHT - 21;

pub const ENEMY_X_MIN: i32 = 10;
pub const ENEMY_X_MAX: i32 = PAGE_WIDTH - 10;
pub const ENEMY_Y_MIN: i32 = 10;
pub const ENEMY_Y_MAX: i32 = PAGE_HEIGHT - 10;
pub const ENEMY_START_Y: i32 = PAGE_HEIGHT / 2 - 5;

pub const SHIP_MAX_X: i32 = PAGE_WIDTH - 11;
pub const SHIP_MAX_Y: i32 = PAGE_HEIGHT - 11;
pub const SHIP_MIN_X: i32 = 10;
pub const SHIP_MIN_Y: i32 = 10;
pub const SHIP_START_X: i32 = PAGE_WIDTH / 2;
pub const SHIP_START_Y: i32 = PAGE_HEIGHT / 2;

pub const SUPER_TIME_MIN: i32 = 5 * FRAME_RATE;
pub const SUPER_TIME_RAN: i32 = 5 * FRAME_RATE;

pub const GAME_CLOCKED_SPEED_UP: [f64; 6] = [0.0, 0.2, 0.5, 1.0, 1.5, 2.0];

pub const GAME_CLOCKED_NAMES: [&str; 5] =
    ["XQUEST WARRIOR", "XQUEST WARRIOR SUPREME", "XQUEST COMMANDER", "XQUEST WARLORD", "XQUEST GOD"];

pub const ENEMY_NAME: [&str; 19] = [
    "", "Explosion", "Grunger", "Zippo", "Zinger", "Vince", "", "Miner", "Meeby", "Retaliator", "Terrier",
    "Doinger", "Snipe", "Tribbler", "Tribble", "Buckshot", "Cluster", "Sticktight", "Repulsor",
];

pub const DIFF_NAME: [&str; 5] = ["Wimp   ", "Timid  ", "Average", "Tricky ", "Inhuman"];
pub const DIFF_NAME2: [&str; 5] = ["WIMP", "TIMID", "AVERAGE", "TRICKY", "INHUMAN"];
pub const PLAYER_STR: [&str; 2] = ["PLAYER 1", "PLAYER 2"];

pub const SMART_BOMB_PAL: [[u8; 3]; 12] = [
    [0, 0, 0],
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

/// Colours of the border "pipes".
pub const BCOLOR: [u8; 6] = [0, 10, 15, 25, 15, 10];

#[derive(Clone, Copy, Debug)]
pub struct DiffInfo {
    pub rebound: bool,
    pub speedfactor: f64,
    pub enemyfrequency: f64,
}

pub const DIFF_INFO: [DiffInfo; 5] = [
    DiffInfo { rebound: true, speedfactor: 0.7, enemyfrequency: 0.7 },
    DiffInfo { rebound: true, speedfactor: 1.0, enemyfrequency: 1.0 },
    DiffInfo { rebound: false, speedfactor: 1.0, enemyfrequency: 1.0 },
    DiffInfo { rebound: false, speedfactor: 1.5, enemyfrequency: 1.2 },
    DiffInfo { rebound: false, speedfactor: 2.0, enemyfrequency: 1.5 },
];

// Sound numbers.
pub const FIRE6: i32 = 1;
pub const FIRE5: i32 = 2;
pub const PHEW: i32 = 3;
pub const FIRE4: i32 = 4;
pub const FIRE: i32 = 5;
pub const BOING: i32 = 6;
pub const SQUELCH: i32 = 7;
pub const WOOHOO: i32 = 8;
pub const ALLRIGHT: i32 = 9;
pub const OHYEAH: i32 = 10;
pub const GETCRYSTAL: i32 = 11;
pub const EXPLOSN: i32 = 12;
pub const RETALIATE: i32 = 15;
pub const OW: i32 = 16;
pub const COUNTDOWN: i32 = 17;
pub const GATESOUND: i32 = 18;
pub const BARK: i32 = 20;
pub const APPLAUSE: i32 = 21;
pub const ENEMYENT: i32 = 22;
pub const MENUCLICK: i32 = 23;
pub const DOH: i32 = 24;
pub const REPULSE: i32 = 25;

pub const NO_SOUND_CARD: i32 = 0;
pub const SOUND_BLASTER: i32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputDevice {
    Mouse = 0,
    Joy = 1,
    Keyboard = 2,
}

// Indices into `PlayerInfo::key_array`.
pub const UP_KEY: usize = 0;
pub const DOWN_KEY: usize = 1;
pub const LEFT_KEY: usize = 2;
pub const RIGHT_KEY: usize = 3;
pub const UP_LEFT_KEY: usize = 4;
pub const UP_RIGHT_KEY: usize = 5;
pub const DOWN_LEFT_KEY: usize = 6;
pub const DOWN_RIGHT_KEY: usize = 7;
pub const BRAKE_KEY: usize = 8;
pub const FIRE_KEY: usize = 9;
pub const SMART_BOMB_KEY: usize = 10;

pub const JOY_A_BUTTON1: i32 = 16;
pub const JOY_A_BUTTON2: i32 = 32;

#[derive(Clone, Copy, Debug)]
pub struct PlayerInfo {
    pub input_device: InputDevice,
    pub h_input_speed: i32,
    pub v_input_speed: i32,
    pub joy_fire_button: i32,
    pub joy_smart_bomb_button: i32,
    pub mouse_fire_button: i32,
    pub mouse_smart_bomb_button: i32,
    pub diff_level: i32,
    pub key_array: [u8; 11],
}

impl Default for PlayerInfo {
    fn default() -> Self {
        Self {
            input_device: InputDevice::Mouse,
            h_input_speed: 64,
            v_input_speed: 64,
            joy_fire_button: JOY_A_BUTTON1,
            joy_smart_bomb_button: JOY_A_BUTTON2,
            mouse_fire_button: 1,
            mouse_smart_bomb_button: 2,
            diff_level: 2,
            key_array: [
                scan::KEYPAD8,
                scan::KEYPAD2,
                scan::KEYPAD4,
                scan::KEYPAD6,
                scan::KEYPAD7,
                scan::KEYPAD9,
                scan::KEYPAD1,
                scan::KEYPAD3,
                scan::KEYPAD5,
                scan::ENTER,
                scan::SPACE,
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GameInfo {
    pub level: i32,
    pub total_level: i32,
    pub num_smart_bombs: i32,
    pub lives: i32,
    pub game_clocked: i32,
    pub score: i64,
    pub new_man_score: i64,
    pub last_new_man_score: i64,
    pub time_on_level: i64,
    pub timing: bool,
    pub num_objects: i32,
    pub num_crystals: i32,
    pub num_mines: i32,
    pub num_smarts: i32,
    pub gate_move_pos: bool,
    pub attractor_x: i32,
    pub attractor_y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ObjType {
    #[default]
    Crys = 0,
    Mine = 1,
    Smart = 2,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ObjPoint {
    pub x: i32,
    pub y: i32,
    pub xbr: i32,
    pub ybr: i32,
    pub typ: ObjType,
    pub delete: bool,
    /// Index of the background buffer this entry points to.
    pub backgr: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Ship {
    pub sx: i32,
    pub sy: i32,
    pub delx: i32,
    pub dely: i32,
    pub x: i32,
    pub y: i32,
    pub xbr: i32,
    pub ybr: i32,
    pub oldx: i32,
    pub oldy: i32,
    pub ship_dir: i32,
    pub backgr: Bitmap,
    pub oldbackgr: Bitmap,
    pub width: i32,
    pub bmwidth: i32,
    pub height: i32,
    pub num_missiles: usize,
    pub miss_width: i32,
    pub miss_height: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Missile {
    pub x: i32,
    pub y: i32,
    pub xbr: i32,
    pub ybr: i32,
    pub sx: i32,
    pub sy: i32,
    pub delx: i32,
    pub dely: i32,
    pub oldx: i32,
    pub oldy: i32,
    pub time: i32,
    pub backgr: Bitmap,
    pub oldbackgr: Bitmap,
}

#[derive(Clone, Debug, Default)]
pub struct EMissile {
    pub x: i32,
    pub y: i32,
    pub xbr: i32,
    pub ybr: i32,
    pub sx: i32,
    pub sy: i32,
    pub delx: i32,
    pub dely: i32,
    pub oldx: i32,
    pub oldy: i32,
    /// Index into `emisskind`.
    pub mtyp: usize,
    pub backgr: Bitmap,
    pub oldbackgr: Bitmap,
}

#[derive(Clone, Debug, Default)]
pub struct Enemy {
    pub x: i32,
    pub y: i32,
    pub xbr: i32,
    pub ybr: i32,
    pub sx: i32,
    pub sy: i32,
    pub delx: i32,
    pub dely: i32,
    pub oldx: i32,
    pub oldy: i32,
    pub curvecos: i32,
    pub curvesin: i32,
    pub ntyp: usize,
    pub hit: i32,
    pub frame: i32,
    pub supertime: i32,
    pub backgr: Bitmap,
    pub oldbackgr: Bitmap,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
    pub xbr: i32,
    pub ybr: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Gate {
    pub x: i32,
    pub width: i32,
    pub bmwidth: i32,
    pub height: i32,
}

#[derive(Clone, Debug, Default)]
pub struct TimeRecord {
    pub x: i32,
    pub y: i32,
    pub oldx: i32,
    pub oldy: i32,
    pub backgr: Bitmap,
    pub oldbackgr: Bitmap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerUpType {
    Shield = 0,
    AimedFire = 1,
    RapidFire = 2,
    MultiFire = 3,
    AssFire = 4,
    HeavyFire = 5,
    Bounce = 6,
}

#[derive(Clone, Copy, Debug)]
pub struct PowerUp {
    pub value: i32,
    pub position: i32,
    pub time_min: i32,
    pub time_ran: i32,
}

const POWER_UPS: [PowerUp; 7] = [
    PowerUp { value: 0, position: 0, time_min: 10 * FRAME_RATE, time_ran: 15 * FRAME_RATE },
    PowerUp { value: 0, position: 0, time_min: 30 * FRAME_RATE, time_ran: 60 * FRAME_RATE },
    PowerUp { value: 0, position: 0, time_min: 60 * FRAME_RATE, time_ran: 90 * FRAME_RATE },
    PowerUp { value: 0, position: 0, time_min: 60 * FRAME_RATE, time_ran: 90 * FRAME_RATE },
    PowerUp { value: 0, position: 0, time_min: 60 * FRAME_RATE, time_ran: 90 * FRAME_RATE },
    PowerUp { value: 0, position: 0, time_min: 60 * FRAME_RATE, time_ran: 90 * FRAME_RATE },
    PowerUp { value: 0, position: 0, time_min: 30 * FRAME_RATE, time_ran: 60 * FRAME_RATE },
];

/// One frame of a recorded demo.
#[derive(Clone, Copy, Debug, Default)]
pub struct DemoRec {
    pub delx: i32,
    pub dely: i32,
    pub but: u8,
}

/// A demo file: a header followed by one [`DemoRec`] per frame.
#[derive(Clone, Debug, Default)]
pub struct Demo {
    pub rand_seed: u32,
    pub two_player: bool,
    pub player_info: [PlayerInfo; 2],
    pub frames: Vec<DemoRec>,
}

pub struct Xq {
    pub vga: Vga,
    pub g: Graphics,
    pub enemykind: Vec<EnemyKind>,
    /// `emisskind[1..6]` (index 0 unused).
    pub emisskind: Vec<EMissKind>,
    pub rng: TpRandom,

    pub shared: SharedRef,
    pub mixer: MixerRef,
    pub kb: Keyboard,
    pub mouse: Mouse,
    /// `TimeCount`, the 180 Hz timer.
    pub time_count: i64,

    pub player_info: [PlayerInfo; 2],
    pub game_info: [GameInfo; 2],
    pub player: usize,
    pub two_player: bool,

    pub game_over: bool,
    pub level_finished: bool,
    pub ship_destroyed: bool,
    pub gate_moving: bool,
    pub no_mines: bool,

    pub text_window_x: i32,
    pub text_window_y: i32,
    pub text_window_size: i32,
    pub win_background: Bitmap,

    pub num_enemy_mines: usize,
    pub emines_adding: bool,
    pub emines: [Point; MAX_ENEMY_MINES + 1],

    pub visible_page_x: i32,
    pub visible_page_y: i32,

    pub enemy_entering_left: i32,
    pub enemy_entering_right: i32,
    pub enemy_left_type: usize,
    pub enemy_right_type: usize,

    pub frame_count: i64,
    pub gate_move_count: i32,
    pub game_speed: i32,
    pub smart_bombed: i32,
    pub ship_destroyed_count: i32,

    /// `enemy[1..NumEnemies]` (index 0 unused).
    pub enemy: Vec<Enemy>,
    pub ship: Ship,
    /// `missiles[1..MaxMissiles]`.
    pub missiles: Vec<Missile>,
    /// `emissiles[1..MaxEnemyMissiles]`.
    pub emissiles: Vec<EMissile>,
    pub num_enemy_missiles: usize,

    /// `objects.pos[1..MaxObjects]`.
    pub objects: Vec<ObjPoint>,
    pub saved_objects: Vec<ObjPoint>,
    /// Object background buffers, referenced by `ObjPoint::backgr`.
    pub obj_backgr: Vec<Bitmap>,

    pub gate: [Gate; 2],
    pub time_record: TimeRecord,
    pub power_up: [PowerUp; 7],
    pub cost: [i32; 16],
    pub sint: [i32; 16],

    pub demo_mode: bool,
    pub recording: bool,
    pub demo: Demo,
    pub demo_ptr: usize,
    pub demo_name: String,
    pub saved_player_info: [PlayerInfo; 2],
    pub saved_two_player: bool,
    pub saved_rand_seed: u32,

    pub sounds_on: bool,
    pub sound_card: i32,
    pub sound_volume: i32,
    pub show_count_down: bool,

    pub start_bombs: i32,
    pub start_lives: i32,
    pub start_level: i32,

    pub stars: crate::core::stars::Stars,
}

impl Xq {
    pub fn new(shared: SharedRef, mixer: MixerRef, seed: u32) -> Self {
        let mut enemykind = gfx::load_enemy_kinds(data::ENM);
        let g = gfx::load_graphics(data::GFX, data::FNT, data::FNT2, &mut enemykind);

        let mut emisskind: Vec<EMissKind> = vec![EMissKind::default(); 7];
        for i in 1..=6 {
            emisskind[i].sprite = g.emiss[i - 1].clone();
        }

        let ship = Ship {
            width: g.ship[0].bm.width,
            bmwidth: g.ship[0].bm.bmwidth,
            height: g.ship[0].bm.height,
            miss_width: g.missile.bm.width,
            miss_height: g.missile.bm.height,
            ..Default::default()
        };

        let mut objects = vec![ObjPoint::default(); MAX_OBJECTS + 1];
        let mut saved_objects = vec![ObjPoint::default(); MAX_OBJECTS + 1];
        for j in 1..=MAX_OBJECTS {
            objects[j].backgr = j;
            saved_objects[j].backgr = MAX_OBJECTS + j;
        }

        let gate = [0, 1].map(|i| Gate {
            x: 0,
            width: g.gates[i].bm.width,
            bmwidth: g.gates[i].bm.bmwidth,
            height: g.gates[i].bm.height,
        });

        Self {
            vga: Vga::new(),
            g,
            enemykind,
            emisskind,
            rng: TpRandom::new(seed),
            shared,
            mixer,
            kb: Keyboard::default(),
            mouse: Mouse::default(),
            time_count: 0,
            player_info: [PlayerInfo::default(); 2],
            game_info: [GameInfo::default(); 2],
            player: 0,
            two_player: false,
            game_over: false,
            level_finished: false,
            ship_destroyed: false,
            gate_moving: false,
            no_mines: false,
            text_window_x: 0,
            text_window_y: 0,
            text_window_size: 0,
            win_background: Bitmap::default(),
            num_enemy_mines: 0,
            emines_adding: false,
            emines: [Point::default(); MAX_ENEMY_MINES + 1],
            visible_page_x: 0,
            visible_page_y: 0,
            enemy_entering_left: 0,
            enemy_entering_right: 0,
            enemy_left_type: 0,
            enemy_right_type: 0,
            frame_count: 0,
            gate_move_count: 0,
            game_speed: BASE_GAME_SPEED,
            smart_bombed: 0,
            ship_destroyed_count: START_SHIP_DESTROYED_COUNT,
            enemy: vec![Enemy::default()],
            ship,
            missiles: vec![Missile::default(); MAX_MISSILES + 1],
            emissiles: vec![EMissile::default(); MAX_ENEMY_MISSILES + 1],
            num_enemy_missiles: 0,
            objects,
            saved_objects,
            obj_backgr: vec![Bitmap::default(); 2 * MAX_OBJECTS + 1],
            gate,
            time_record: TimeRecord::default(),
            power_up: POWER_UPS,
            cost: [0; 16],
            sint: [0; 16],
            demo_mode: false,
            recording: false,
            demo: Demo::default(),
            demo_ptr: 0,
            demo_name: String::new(),
            saved_player_info: [PlayerInfo::default(); 2],
            saved_two_player: false,
            saved_rand_seed: 0,
            sounds_on: true,
            sound_card: SOUND_BLASTER,
            sound_volume: 96,
            show_count_down: true,
            start_bombs: 3,
            start_lives: 3,
            start_level: 1,
            stars: crate::core::stars::Stars::default(),
        }
    }

    pub fn new_mixer() -> MixerRef {
        let sounds = Arc::new(gfx::load_sounds(data::SND));
        Arc::new(std::sync::Mutex::new(Mixer::new(sounds)))
    }

    // ------------------------------------------------------------------------------------
    // Platform helpers.
    // ------------------------------------------------------------------------------------

    /// Waits for the vertical retrace: publishes the screen and gives the host one frame.
    pub async fn vsync(&mut self) {
        {
            let mut shared = self.shared.lock().unwrap();
            let Self { vga, .. } = self;
            vga.render_rgba(&mut shared.frame);
            shared.frame_serial += 1;

            let gi = &self.game_info[self.player];
            shared.status = crate::core::platform::GameStatus {
                total_level: gi.total_level,
                time_on_level: gi.time_on_level,
                score: gi.score,
                lives: gi.lives,
                rand_seed: self.rng.seed,
            };
        }

        YieldNow::new().await;

        self.time_count += TICKS / 60;
        self.poll_input();
    }

    /// `XPageFlip`: shows the hidden page and waits for the vertical retrace.
    pub async fn xpage_flip(&mut self) {
        self.vga.page_flip(self.visible_page_x, self.visible_page_y);
        self.vsync().await;
    }

    fn poll_input(&mut self) {
        let (events, dx, dy) = {
            let mut shared = self.shared.lock().unwrap();
            let events = std::mem::take(&mut shared.events);
            let (dx, dy) = (shared.mouse_dx, shared.mouse_dy);
            shared.mouse_dx = 0;
            shared.mouse_dy = 0;
            (events, dx, dy)
        };

        for e in events {
            match e {
                crate::core::platform::InputEvent::Key { scan, pressed } => self.kb.event(scan, pressed),
                crate::core::platform::InputEvent::MouseButton { button, pressed } => {
                    self.mouse.button_event(button, pressed)
                }
            }
        }

        self.mouse.motion(dx, dy);
    }

    pub fn set_mouse_grab(&mut self, grab: bool) {
        self.shared.lock().unwrap().wants_mouse_grab = grab;
    }

    pub fn play_sound(&mut self, s: i32) {
        if !self.sounds_on || self.sound_card != SOUND_BLASTER || s <= 0 || s > 25 {
            return;
        }
        self.mixer.lock().unwrap().add_sound(s as usize);
    }

    pub fn level(&self) -> &'static Level {
        &LEVELS[(self.game_info[self.player].level - 1) as usize]
    }

    pub fn gi(&mut self) -> &mut GameInfo {
        &mut self.game_info[self.player]
    }

    pub fn pi(&self) -> &PlayerInfo {
        &self.player_info[self.player]
    }

    fn base_game_speed(&self) -> i32 {
        let gi = &self.game_info[self.player];
        let pi = &self.player_info[self.player];
        round(
            BASE_GAME_SPEED as f64
                * (DIFF_INFO[pi.diff_level as usize].speedfactor + GAME_CLOCKED_SPEED_UP[gi.game_clocked as usize]),
        )
    }

    // ------------------------------------------------------------------------------------
    // Level set up.
    // ------------------------------------------------------------------------------------

    pub fn swap_objects(&mut self) {
        std::mem::swap(&mut self.objects, &mut self.saved_objects);
    }

    fn no_overlap(&self, player: usize, num_obj: usize) -> bool {
        let ow = self.g.objects[0].bm.width;
        let oh = self.g.objects[0].bm.height;
        let mut result = true;

        for i in 1..num_obj {
            for j in (i + 1)..=num_obj {
                let (a, b) = (&self.objects[i], &self.objects[j]);
                if (a.x - b.x).abs() < ow + 2 && (a.y - b.y).abs() < oh + 2 {
                    result = false;
                }
            }
        }

        let p = &self.objects[num_obj];
        if p.xbr + 8 >= SHIP_START_X
            && p.ybr + 8 >= SHIP_START_Y
            && p.x <= SHIP_START_X + self.ship.width + 8
            && p.y <= SHIP_START_Y + self.ship.height + 8
        {
            result = false;
        }

        let gi = &self.game_info[player];
        if gi.game_clocked > 0
            && p.xbr + 8 >= gi.attractor_x
            && p.ybr + 8 >= gi.attractor_y
            && p.x <= gi.attractor_x + 13 + 8
            && p.y <= gi.attractor_y + 13 + 8
        {
            result = false;
        }

        if p.y < PAGE_HEIGHT / 2 + 7 && p.ybr > PAGE_HEIGHT / 2 - 7 && (p.x < SHIP_MIN_X + 15 || p.xbr > SHIP_MAX_X - 15) {
            result = false;
        }

        if LEVELS[(gi.level - 1) as usize].gate_move == 0
            && p.y < SHIP_MIN_Y + 18
            && p.xbr > self.gate[0].x
            && p.x < self.gate[1].x
        {
            result = false;
        }

        result
    }

    pub fn setup_new_level(&mut self, player: usize) {
        let level = LEVELS[(self.game_info[player].level - 1) as usize];

        {
            let gi = &mut self.game_info[player];
            gi.new_man_score = if gi.total_level <= MAX_LEVEL as i32 {
                gi.last_new_man_score + level.newman
            } else {
                gi.last_new_man_score + LEVELS[MAX_LEVEL - 1].newman
            };
            gi.num_mines = level.nummine;
            gi.num_crystals = level.numcryst;
            gi.num_smarts = 0;
            gi.time_on_level = 0;
        }
        self.gate_move_count = 0;

        self.gate[0].x = (PAGE_WIDTH - level.gate_width) / 2 - self.gate[0].width;
        self.gate[1].x = (PAGE_WIDTH + level.gate_width) / 2;

        loop {
            let r = self.rng.real();
            if !(r < level.smartprob && self.game_info[player].num_smarts < level.maxsmart) {
                break;
            }
            self.game_info[player].num_smarts += 1;
        }

        {
            let gi = &mut self.game_info[player];
            gi.num_objects = gi.num_mines + gi.num_crystals + gi.num_smarts;
        }
        self.no_mines = self.game_info[player].num_mines == 0;

        loop {
            let ax = self.rng.int(PAGE_WIDTH - 120) + 60;
            let ay = self.rng.int(PAGE_HEIGHT - 120) + 60;
            self.game_info[player].attractor_x = ax;
            self.game_info[player].attractor_y = ay;
            if (ax - SHIP_START_X).abs() + (ay - SHIP_START_Y).abs() > 60 {
                break;
            }
        }

        let ow = self.g.objects[0].bm.width;
        let oh = self.g.objects[0].bm.height;
        let gi = self.game_info[player];

        for i in 1..=gi.num_objects as usize {
            loop {
                let x = self.rng.int(MINE_X_MAX - MINE_X_MIN) + MINE_X_MIN;
                let y = self.rng.int(MINE_Y_MAX - MINE_Y_MIN) + MINE_Y_MIN;
                let p = &mut self.objects[i];
                p.x = x;
                p.y = y;
                p.xbr = x + ow - 1;
                p.ybr = y + oh - 1;
                if self.no_overlap(player, i) {
                    break;
                }
            }

            let p = &mut self.objects[i];
            p.typ = if i as i32 <= gi.num_crystals {
                ObjType::Crys
            } else if i as i32 <= gi.num_mines + gi.num_crystals {
                ObjType::Mine
            } else {
                ObjType::Smart
            };
            p.delete = false;
        }

        let pi = &self.player_info[self.player];
        self.game_speed = round(
            BASE_GAME_SPEED as f64
                * (DIFF_INFO[pi.diff_level as usize].speedfactor + GAME_CLOCKED_SPEED_UP[gi.game_clocked as usize]),
        );
    }

    // ------------------------------------------------------------------------------------
    // Status bar.
    // ------------------------------------------------------------------------------------

    pub fn show_lives(&mut self) {
        let s = str_w((self.game_info[self.player].lives - 1) as i64, 2);
        let split = self.vga.split_offs;
        self.xtext(185, 5, true, split, &format!("  {s}"));
        let pic = self.g.ship_pic.clone();
        self.vga.put_pbm(185, 6, split, &pic);
    }

    pub fn show_smart_bombs(&mut self) {
        let s = str_w(self.game_info[self.player].num_smart_bombs as i64, 2);
        let split = self.vga.split_offs;
        self.xtext(230, 5, true, split, &format!("  {s}"));
        let pic = self.g.smart_pic.clone();
        self.vga.put_pbm(230, 6, split, &pic);
    }

    pub fn show_crystals(&mut self) {
        let s = str_w(self.game_info[self.player].num_crystals as i64, 2);
        let split = self.vga.split_offs;
        self.xtext(275, 5, true, split, &format!("  {s}"));
        let pic = self.g.crystal_pic.clone();
        self.vga.put_pbm(275, 6, split, &pic);
    }

    pub fn show_powerups(&mut self) {
        let split = self.vga.split_offs;
        let mut i = 85;

        for pu in 0..7 {
            if self.power_up[pu].value > 0 && i <= 157 {
                self.power_up[pu].position = i;
                let pic = self.g.powerups[pu].clone();
                self.vga.put_pbm(i, 5, split, &pic);
                i += 18;
            } else {
                self.power_up[pu].position = 0;
            }
        }

        if i <= 157 {
            self.vga.rect_fill(i, 5, 157 + 17, 20, split, 14);
        }
    }

    fn bonus_ship(&mut self) {
        let gi = &mut self.game_info[self.player];
        gi.last_new_man_score = gi.new_man_score;
        gi.new_man_score = if gi.total_level <= MAX_LEVEL as i32 {
            gi.last_new_man_score + LEVELS[(gi.level - 1) as usize].newman
        } else {
            gi.last_new_man_score + LEVELS[MAX_LEVEL - 1].newman
        };
        gi.lives += 1;

        self.show_lives();
        self.play_sound(WOOHOO);
    }

    pub fn add_to_score(&mut self, i: i64) {
        self.game_info[self.player].score += i;

        while self.game_info[self.player].score >= self.game_info[self.player].new_man_score {
            self.bonus_ship();
        }

        let s = str_w(self.game_info[self.player].score, 8);
        let split = self.vga.split_offs;
        self.xtext(10, 5, true, split, &s);
    }

    // ------------------------------------------------------------------------------------
    // Starting a level.
    // ------------------------------------------------------------------------------------

    pub async fn start_new_level(&mut self) {
        self.enemy.truncate(1);
        self.num_enemy_missiles = 0;
        self.num_enemy_mines = 0;
        self.emines_adding = false;
        self.level_finished = false;
        self.ship_destroyed = false;
        self.enemy_entering_left = 0;
        self.enemy_entering_right = 0;
        self.gate_moving = self.level().gate_move != 0;

        let (p0, p1) = (self.vga.page0_offs, self.vga.page1_offs);
        let lw = PAGE_WIDTH;
        let lh = self.vga.logical_height;
        self.vga.rect_fill(0, 0, lw, lh, p0, 0);
        self.vga.rect_fill(0, 0, lw, lh, p1, 0);

        self.visible_page_x = (PAGE_WIDTH - PHYSICAL_PAGE_WIDTH) / 2;
        self.visible_page_y = (lh - SPLIT_SCREEN_LINE) / 2;
        self.xpage_flip().await;

        let gl = self.gate[0];
        let gr = self.gate[1];

        for i in 1..=5 {
            let c = BCOLOR[i as usize];
            self.vga.line(10, 10 - i, gl.x, 10 - i, p0, c);
            self.vga.line(gr.x + gr.width, 10 - i, PAGE_WIDTH - 10, 10 - i, p0, c);
            self.vga.line(10, PAGE_HEIGHT - 5 - i, PAGE_WIDTH - 10, PAGE_HEIGHT - 5 - i, p0, c);
            self.vga.line(4 + i, 10, 4 + i, PAGE_HEIGHT - 9, p0, c);
            self.vga.line(PAGE_WIDTH - 11 + i, 10, PAGE_WIDTH - 11 + i, PAGE_HEIGHT - 9, p0, c);
        }

        self.vga.put_pbm(gl.x, 0, p0, &self.g.gates[0].bm);
        self.vga.put_pbm(gr.x, 0, p0, &self.g.gates[1].bm);
        if self.game_info[self.player].num_crystals > 0 {
            self.vga.line(gl.x + gl.width, 7, gr.x - 1, 7, p0, 54);
        }
        self.vga.put_pbm(0, 0, p0, &self.g.tl_corner);
        self.vga.put_pbm(PAGE_WIDTH - 10, 0, p0, &self.g.tr_corner);
        self.vga.put_pbm(PAGE_WIDTH - 10, PAGE_HEIGHT - 10, p0, &self.g.br_corner);
        self.vga.put_pbm(0, PAGE_HEIGHT - 10, p0, &self.g.bl_corner);
        self.vga.put_pbm(0, PAGE_HEIGHT / 2 - 10, p0, &self.g.l_enemy_gate[0]);
        self.vga.put_pbm(PAGE_WIDTH - 20, PAGE_HEIGHT / 2 - 10, p0, &self.g.r_enemy_gate[0]);

        let gi = self.game_info[self.player];
        if gi.game_clocked > 0 {
            self.vga.put_pbm(gi.attractor_x - 6, gi.attractor_y - 6, p0, &self.g.attractor);
        }

        for _ in 0..100 {
            let x = self.rng.int(SHIP_MAX_X - SHIP_MIN_X - 2) + SHIP_MIN_X + 1;
            let y = self.rng.int(SHIP_MAX_Y - SHIP_MIN_Y - 5) + SHIP_MIN_Y + 5;
            let c = self.rng.int(20) + 10;
            self.vga.put_pix(x, y, p0, c as u8);
        }
        for _ in 0..400 {
            let x = self.rng.int(SHIP_MAX_X - SHIP_MIN_X - 2) + SHIP_MIN_X + 1;
            let y = self.rng.int(SHIP_MAX_Y - SHIP_MIN_Y - 5) + SHIP_MIN_Y + 5;
            let c = self.rng.int(15) + 5;
            self.vga.put_pix(x, y, p0, c as u8);
        }

        // Delete any objects hit last run. This copies (rather than swaps) entries, so
        // two entries can end up sharing a background buffer, as in the original.
        let mut i = self.game_info[self.player].num_objects;
        while i >= 1 {
            if self.objects[i as usize].delete {
                let n = self.game_info[self.player].num_objects as usize;
                self.objects[i as usize] = self.objects[n];
                self.game_info[self.player].num_objects -= 1;
            }
            i -= 1;
        }

        {
            let s = &mut self.ship;
            s.x = SHIP_START_X;
            s.y = SHIP_START_Y;
            s.xbr = s.x + s.width - 1;
            s.ybr = s.y + s.height - 1;
            s.oldx = s.x;
            s.oldy = s.y;
            s.sx = s.x << 6;
            s.sy = s.y << 6;
            s.delx = 0;
            s.dely = 0;
            s.ship_dir = 0;
            s.num_missiles = 0;
            s.backgr = self.vga.get_pbm(s.x, s.y, s.bmwidth, s.height, p0);
            s.oldbackgr = self.vga.get_pbm(s.x, s.y, s.bmwidth, s.height, p0);
        }

        let obm = self.g.objects[0].bm.bmwidth;
        let oh = self.g.objects[0].bm.height;
        for i in 1..=self.game_info[self.player].num_objects as usize {
            let p = self.objects[i];
            self.obj_backgr[p.backgr] = self.vga.get_pbm(p.x, p.y, obm, oh, p0);
            self.vga.put_cbitmap(p.x, p.y, p0, &self.g.objects[p.typ as usize].bm);
        }

        self.vga.cp_vid_rect(0, 0, lw, lh, 0, 0, p0, p1);

        let split = self.vga.split_offs;
        self.xwindow(0, 0, 319, 24, 2, split);
        self.add_to_score(0);
        self.show_lives();
        self.show_smart_bombs();
        self.show_crystals();

        {
            let t = &mut self.time_record;
            t.x = TIMER_X + self.visible_page_x;
            t.y = TIMER_Y + self.visible_page_y;
            t.oldx = t.x;
            t.oldy = t.y;
            let sf = &self.g.small_font[0];
            t.backgr = self.vga.get_pbm(t.x, t.y, sf.bmwidth * 4, sf.height, p0);
            t.oldbackgr = self.vga.get_pbm(t.x, t.y, sf.bmwidth * 4, sf.height, p0);
        }
        self.show_powerups();

        self.mouse.mmotion();
        self.mouse.mbutpress(0);
        self.mouse.mbutpress(1);
    }

    // ------------------------------------------------------------------------------------
    // End of level.
    // ------------------------------------------------------------------------------------

    pub async fn give_bonus(&mut self) {
        self.xpage_flip().await;
        let vis = self.vga.visible_page_offs;
        let (ox, oy) = (self.ship.oldx, self.ship.oldy);
        let bg = self.ship.backgr.clone();
        self.vga.put_pbm(ox, oy, vis, &bg);

        let gi = self.game_info[self.player];
        let level = *self.level();
        let time_taken = gi.time_on_level / FRAME_RATE as i64;

        let (vx, vy) = (self.visible_page_x, self.visible_page_y);
        self.xwindow(50 + vx, 40 + vy, 270 + vx, 160 + vy, 2, vis);
        let (tx, ty) = (self.text_window_x, self.text_window_y);
        self.xtext(tx + 42, ty + 15, true, vis, &format!("LEVEL {} COMPLETED", gi.total_level));
        self.xtext(tx + 20, ty + 40, true, vis, &format!("TIME TAKEN: {} SECONDS", str_w(time_taken, 3)));
        self.xtext(tx + 20, ty + 65, true, vis, &format!("PAR       : {} SECONDS", str_w(level.time as i64, 3)));

        let mut bonus = (level.time as i64 - time_taken) * 500;
        if bonus < 0 {
            bonus = 0;
        }
        self.xtext(tx + 20, ty + 90, true, vis, "BONUS     :  0         ");

        let n = bonus / 500;
        if !self.delay_or_event(TICKS / 2).await {
            let mut i = 1;
            while i <= n {
                self.xtext(tx + 124, ty + 90, false, vis, &(i * 500).to_string());
                self.xtext(tx + 116, ty + 40, false, vis, &str_w(time_taken + i, 3));
                if i % 3 == 1 {
                    self.play_sound(COUNTDOWN);
                }
                if self.delay_or_event(TICKS / 12).await && i != n {
                    i = n - 1;
                }
                i += 1;
            }
        } else {
            self.xtext(tx + 124, ty + 90, false, vis, &bonus.to_string());
            self.xtext(tx + 116, ty + 40, false, vis, &str_w(time_taken + n, 3));
            if bonus > 0 {
                self.play_sound(COUNTDOWN);
            }
        }

        if bonus > 0 {
            if !self.delay_or_event(TICKS / 2).await {
                let mut i = n - 1;
                while i >= 0 {
                    self.xtext(tx + 124, ty + 90, false, vis, &format!("{}    ", i * 500));
                    self.add_to_score(500);
                    self.play_sound(COUNTDOWN);
                    if self.delay_or_event(TICKS / 12).await && i != 0 {
                        self.add_to_score((i - 1) * 500);
                        i = 1;
                    }
                    i -= 1;
                }
            } else {
                self.xtext(tx + 124, ty + 90, false, vis, "0    ");
                self.add_to_score(bonus);
                if bonus > 0 {
                    self.play_sound(COUNTDOWN);
                }
            }
        }

        self.delay_or_event(TICKS * 5 / 2).await;
    }

    pub async fn show_game_clocked_message(&mut self) {
        let vis = self.vga.visible_page_offs;
        let (vx, vy) = (self.visible_page_x, self.visible_page_y);
        self.xwindow(50 + vx, 40 + vy, 270 + vx, 160 + vy, 2, vis);
        let ty = self.text_window_y;
        self.xtext_center(160 + vx, ty + 15, false, vis, "CONGRATULATIONS");
        self.xtext_center(160 + vx, ty + 40, false, vis, "YOU HAVE ATTAINED THE");
        self.xtext_center(160 + vx, ty + 65, false, vis, "RANK OF");
        let name = GAME_CLOCKED_NAMES[(self.game_info[self.player].game_clocked - 1) as usize];
        self.xtext_center(160 + vx, ty + 90, false, vis, name);
        self.wait_for_event(false).await;
    }

    // ------------------------------------------------------------------------------------
    // Gates, enemies and missiles.
    // ------------------------------------------------------------------------------------

    fn move_gate(&mut self) {
        let mv = self.gate_move_count / 64;
        self.gate_move_count %= 64;

        let prob = self.level().gate_change_dir_prob;
        if self.rng.real() < prob {
            let gi = self.gi();
            gi.gate_move_pos = !gi.gate_move_pos;
        }

        if mv > 0 {
            if self.game_info[self.player].gate_move_pos {
                self.gate[0].x += mv;
                self.gate[1].x += mv;
                if self.gate[1].x > MAX_GATE_X {
                    self.gi().gate_move_pos = false;
                }
            } else {
                self.gate[0].x -= mv;
                self.gate[1].x -= mv;
                if self.gate[0].x < MIN_GATE_X {
                    self.gi().gate_move_pos = true;
                }
            }

            if self.game_info[self.player].num_crystals > 0 {
                let (gl, gr) = (self.gate[0], self.gate[1]);
                let (p0, p1) = (self.vga.page0_offs, self.vga.page1_offs);
                self.vga.line(gl.x + gl.width, 7, gr.x - 1, 7, p0, 54);
                self.vga.line(gl.x + gl.width, 7, gr.x - 1, 7, p1, 54);
            }
        }
    }

    fn fix_emines(&mut self, ox: i32, oy: i32, oxbr: i32, oybr: i32, page: i32) {
        for i in (1..=self.num_enemy_mines).rev() {
            let m = self.emines[i];
            if m.xbr >= ox && m.ybr >= oy && m.x <= oxbr && m.y <= oybr {
                self.vga.put_cbitmap(m.x, m.y, page, &self.g.emine.bm);
            }
        }
    }

    pub fn add_enemy(&mut self, xc: i32, yc: i32, i: usize) {
        if self.enemy.len() - 1 >= MAX_ENEMIES {
            return;
        }

        let k = &self.enemykind[i];
        let mut e = Enemy { ntyp: i, ..Default::default() };

        if !k.maxspeed {
            if k.speed > 0 {
                e.delx = w16((self.rng.int(k.speed) - k.speed2) * self.game_speed / 64);
                e.dely = w16((self.rng.int(k.speed) - k.speed2) * self.game_speed / 64);
            } else {
                e.delx = 0;
                e.dely = 0;
            }
            e.supertime = if i == 1 {
                self.enemykind[1].numframes * (256 / self.enemykind[1].framespeed)
            } else {
                0
            };
        } else {
            let theta = (self.rng.real() * 2.0 * std::f64::consts::PI - std::f64::consts::PI).cos();
            e.delx = w16(round(k.speed as f64 * theta) * self.game_speed / 64);
            e.dely = w16(round(k.speed as f64 * (1.0 - theta * theta)) * self.game_speed / 64);
        }

        if i == 0 {
            e.supertime = SUPER_TIME_MIN + self.rng.int(SUPER_TIME_RAN);
        }

        e.curvesin = w16(-k.curve2 + self.rng.int(k.curve));
        e.curvecos = round((1.0 - (e.curvesin as f64 / 32767.0).powi(2)).sqrt() * 32767.0);
        e.x = xc;
        e.y = yc;
        e.xbr = e.x + k.width - 1;
        e.ybr = e.y + k.height - 1;
        e.sx = w16(e.x << 6);
        e.sy = w16(e.y << 6);
        e.oldx = e.x;
        e.oldy = e.y;
        e.frame = 0;
        e.hit = k.hits;

        let hidden = self.vga.hidden_page_offs;
        e.backgr = self.vga.get_pbm(e.x, e.y, k.bmwidth, k.height, hidden);
        e.oldbackgr = self.vga.get_pbm(e.x, e.y, k.bmwidth, k.height, hidden);

        self.enemy.push(e);
    }

    fn num_enemies(&self) -> usize {
        self.enemy.len() - 1
    }

    fn add_enemy_missile(&mut self, en: usize) {
        if self.num_enemy_missiles >= MAX_ENEMY_MISSILES {
            return;
        }

        self.num_enemy_missiles += 1;
        let n = self.num_enemy_missiles;

        let e = &self.enemy[en];
        let k = &self.enemykind[e.ntyp];
        let mtyp = k.firetype as usize;
        let mk = &self.emisskind[mtyp];

        let sx = w16(e.sx + (k.width << 5));
        let sy = w16(e.sy + (k.width << 5));
        let x = (sx as u16 >> 6) as i32;
        let y = (sy as u16 >> 6) as i32;
        let (mspeed, soundnum, firedirect) = (mk.mspeed, mk.soundnum, mk.firedirect);
        let (mw, mh) = (mk.sprite.bm.width, mk.sprite.bm.height);

        let hidden = self.vga.hidden_page_offs;
        let backgr = self.vga.get_pbm(x, y, 1, mh, hidden);
        let oldbackgr = backgr.clone();

        self.play_sound(soundnum);

        let (delx, dely);
        if firedirect {
            let dx = self.ship.x - x;
            let dy = self.ship.y - y;
            let mut temp = if dx.abs() > dy.abs() { dx.abs() + dy.abs() / 2 } else { dy.abs() + dx.abs() / 2 };
            if temp == 0 {
                temp = 1;
            }
            delx = w16(dx * mspeed / temp * self.game_speed / 64);
            dely = w16(dy * mspeed / temp * self.game_speed / 64);
        } else {
            delx = w16((self.rng.int(mspeed) - (mspeed >> 1)) * self.game_speed / 64);
            dely = w16((self.rng.int(mspeed) - (mspeed >> 1)) * self.game_speed / 64);
        }

        let m = &mut self.emissiles[n];
        m.sx = sx;
        m.sy = sy;
        m.x = x;
        m.y = y;
        m.oldx = x;
        m.oldy = y;
        m.mtyp = mtyp;
        m.xbr = x + mw - 1;
        m.ybr = y + mh - 1;
        m.backgr = backgr;
        m.oldbackgr = oldbackgr;
        m.delx = delx;
        m.dely = dely;
    }

    fn explode(&mut self, en: usize) {
        let mut temp = MAX_ENEMY_MISSILES as i32 - self.num_enemy_missiles as i32;
        if temp > 15 {
            temp = 15;
        }

        for i in 1..=temp {
            self.num_enemy_missiles += 1;
            let n = self.num_enemy_missiles;

            let e = &self.enemy[en];
            let k = &self.enemykind[e.ntyp];
            let mtyp = k.firetype as usize;
            let mk = &self.emisskind[mtyp];

            let sx = w16(e.sx + (k.width << 5));
            let sy = w16(e.sy + (k.width << 5));
            let x = (sx as u16 >> 6) as i32;
            let y = (sy as u16 >> 6) as i32;
            let (mspeed, soundnum) = (mk.mspeed, mk.soundnum);
            let (mw, mh) = (mk.sprite.bm.width, mk.sprite.bm.height);

            let hidden = self.vga.hidden_page_offs;
            let backgr = self.vga.get_pbm(x, y, 1, mh, hidden);

            self.play_sound(soundnum);

            let delx = w16(self.cost[i as usize] * mspeed / 32768 * self.game_speed / 64);
            let dely = w16(self.sint[i as usize] * mspeed / 32768 * self.game_speed / 64);

            let m = &mut self.emissiles[n];
            m.sx = sx;
            m.sy = sy;
            m.x = x;
            m.y = y;
            m.oldx = x;
            m.oldy = y;
            m.mtyp = mtyp;
            m.xbr = x + mw - 1;
            m.ybr = y + mh - 1;
            m.oldbackgr = backgr.clone();
            m.backgr = backgr;
            m.delx = delx;
            m.dely = dely;
        }
    }

    pub fn delete_enemy(&mut self, i: usize) {
        let vis = self.vga.visible_page_offs;
        let (ox, oy) = (self.enemy[i].oldx, self.enemy[i].oldy);
        let bg = std::mem::take(&mut self.enemy[i].oldbackgr);
        self.vga.put_pbm(ox, oy, vis, &bg);

        let last = self.enemy.len() - 1;
        self.enemy.swap(i, last);
        self.enemy.pop();
    }

    fn enemy_destroyed(&mut self, i: usize, dx: i32, dy: i32, mut explosion: bool) {
        let heavy = self.power_up[PowerUpType::HeavyFire as usize].value;

        if heavy > 0 {
            self.enemy[i].hit = 0;
        } else {
            self.enemy[i].hit -= 1;
        }

        if self.enemy[i].hit > 0 {
            self.play_sound(DOH);
            return;
        }

        let ntyp = self.enemy[i].ntyp;
        let (shootback, rebounds, explodes, score, deathsound, tribbles) = {
            let k = &self.enemykind[ntyp];
            (k.shootback, k.rebounds, k.explodes, k.score, k.deathsound, k.tribbles)
        };

        if shootback {
            self.add_enemy_missile(i);
        }
        if rebounds && heavy <= 0 {
            self.play_sound(OW);
            explosion = false;
        }
        if explodes {
            self.explode(i);
        }
        self.add_to_score(score as i64);

        if explosion {
            if deathsound == EXPLOSN {
                let r = self.rng.int(3);
                self.play_sound(deathsound + r);
            } else {
                self.play_sound(deathsound);
            }

            if tribbles {
                let (x, y) = (self.enemy[i].x, self.enemy[i].y);
                for _ in 0..5 {
                    self.add_enemy(x, y, ntyp + 1);
                }
            }

            let (j, k) = (self.enemy[i].x, self.enemy[i].y);
            self.delete_enemy(i);
            self.add_enemy(j, k, 1);
        } else if rebounds && heavy <= 0 {
            let e = &mut self.enemy[i];
            e.delx = w16(3 * e.delx / 4 + dx / 4);
            e.dely = w16(3 * e.dely / 4 + dy / 4);
        } else {
            self.delete_enemy(i);
        }
    }

    fn fire_smart_bomb(&mut self) {
        if self.game_info[self.player].num_smart_bombs > 0 {
            let mut i = self.num_enemies();
            while i >= 1 {
                self.enemy[i].hit = 1;
                if self.enemy[i].ntyp > 1 {
                    self.enemy_destroyed(i, 0, 0, true);
                } else {
                    self.enemy_destroyed(i, 0, 0, false);
                }
                i -= 1;
            }

            let vis = self.vga.visible_page_offs;
            for i in (1..=self.num_enemy_missiles).rev() {
                let m = &self.emissiles[i];
                self.vga.put_pbm(m.oldx, m.oldy, vis, &m.oldbackgr);
            }
            self.num_enemy_missiles = 0;

            self.gi().num_smart_bombs -= 1;
            let [r, g, b] = SMART_BOMB_PAL[10];
            self.vga.set_rgb(0, r, g, b);
            self.smart_bombed = 11;
            self.play_sound(EXPLOSN);
        }

        self.show_smart_bombs();
    }

    // ------------------------------------------------------------------------------------
    // The ship.
    // ------------------------------------------------------------------------------------

    fn fire_missile(&mut self, deltax: i32, deltay: i32) {
        if self.ship.num_missiles >= MAX_MISSILES {
            return;
        }

        self.ship.num_missiles += 1;
        let s = &self.ship;
        let sx = w16(s.sx + (s.width << 5) - (s.miss_width << 4));
        let sy = w16(s.sy + (s.height << 5) - (s.miss_height << 4));
        let x = (sx as u16 >> 6) as i32;
        let y = (sy as u16 >> 6) as i32;
        let (mw, mh) = (s.miss_width, s.miss_height);

        let hidden = self.vga.hidden_page_offs;
        let backgr = self.vga.get_pbm(x, y, 1, mh, hidden);

        let m = &mut self.missiles[self.ship.num_missiles];
        m.sx = sx;
        m.sy = sy;
        m.x = x;
        m.y = y;
        m.xbr = x + mw - 1;
        m.ybr = y + mh - 1;
        m.delx = deltax;
        m.dely = deltay;
        m.oldx = x;
        m.oldy = y;
        m.time = 0;
        m.oldbackgr = backgr.clone();
        m.backgr = backgr;
    }

    fn shoot(&mut self, mut deltax: i32, mut deltay: i32) {
        const AIMED_FIRE_SPEED: i32 = 256;
        let shotdelta = std::f64::consts::PI / 180.0 * 10.0;

        if self.ship.num_missiles < MAX_MISSILES {
            self.play_sound(FIRE);
        }

        let n = self.num_enemies();
        if self.power_up[PowerUpType::AimedFire as usize].value > 0 && n > 0 {
            let mut j = 1;
            let mut mindist = i32::MAX as i64;
            for i in 1..=n {
                let dx = (self.ship.x - self.enemy[i].x) as i64;
                let dy = (self.ship.y - self.enemy[i].y) as i64;
                let dist = dx * dx + dy * dy;
                if dist < mindist {
                    mindist = dist;
                    j = i;
                }
            }

            let mut mindist = round((mindist as f64).sqrt());
            let framedelta = w16(mindist / (AIMED_FIRE_SPEED / 64));
            let e = &self.enemy[j];
            let exp = w16(e.x + w16(framedelta * e.delx) / 64);
            let eyp = w16(e.y + w16(framedelta * e.dely) / 64);
            if mindist == 0 {
                mindist = 1;
            }
            deltax = w16(AIMED_FIRE_SPEED * (exp - self.ship.x) / mindist);
            deltay = w16(AIMED_FIRE_SPEED * (eyp - self.ship.y) / mindist);
        }

        self.fire_missile(deltax, deltay);

        if self.power_up[PowerUpType::AssFire as usize].value > 0 {
            self.fire_missile(w16(-deltax), w16(-deltay));
        }

        if self.power_up[PowerUpType::MultiFire as usize].value > 0 {
            let r = ((deltax as f64).powi(2) + (deltay as f64).powi(2)).sqrt();
            let mut theta = if deltax != 0 {
                (deltay as f64 / deltax as f64).atan()
            } else if deltay > 0 {
                std::f64::consts::PI / 2.0
            } else {
                -std::f64::consts::PI / 2.0
            };
            if deltax < 0 {
                theta += 3.14159;
            }

            let a = (w16(round(r * (theta + shotdelta).cos())), w16(round(r * (theta + shotdelta).sin())));
            let b = (w16(round(r * (theta - shotdelta).cos())), w16(round(r * (theta - shotdelta).sin())));
            self.fire_missile(a.0, a.1);
            self.fire_missile(b.0, b.1);

            if self.power_up[PowerUpType::AssFire as usize].value > 0 {
                self.fire_missile(w16(-a.0), w16(-a.1));
                self.fire_missile(w16(-b.0), w16(-b.1));
            }
        }
    }

    fn decrement_power_ups(&mut self) {
        let split = self.vga.split_offs;

        for pu in 0..7 {
            if self.power_up[pu].value > 0 {
                self.power_up[pu].value -= 1;
                let p = self.power_up[pu];

                if p.value < 198 {
                    if p.value % 22 == 0 && p.position != 0 {
                        self.vga.rect_fill(p.position, 5, p.position + 17, 20, split, 14);
                    } else if p.value % 22 == 11 && p.position != 0 {
                        let pic = self.g.powerups[pu].clone();
                        self.vga.put_pbm(p.position, 5, split, &pic);
                    }
                }

                if p.value == 0 {
                    self.show_powerups();
                }
            }
        }
    }

    fn wall_collide_hor(mask: &[u32], height: i32, wallmask: i32) -> bool {
        (0..height as usize).any(|i| mask[i] & wallmask as u32 != 0)
    }

    fn wall_collide_ver(mask: &[u32], minheight: i32, maxheight: i32) -> bool {
        (minheight..=maxheight).any(|i| mask.get(i as usize).copied().unwrap_or(0) != 0)
    }

    fn shield_value(&self) -> i32 {
        self.power_up[PowerUpType::Shield as usize].value
    }

    fn check_wall_collisions(&mut self) {
        let diff = self.pi().diff_level;
        let dir = self.ship.ship_dir as usize;
        let mask = self.g.ship[dir].mask.clone();
        let shield = self.shield_value();
        let s = &mut self.ship;

        if s.xbr >= SHIP_MAX_X - 10 && Self::wall_collide_hor(&mask, s.height, shl32(1, 31 - (SHIP_MAX_X - s.x)).wrapping_sub(1)) {
            s.x = SHIP_MAX_X - s.width;
            if shield <= 0 && diff > 1 {
                self.ship_destroyed = true;
            } else {
                s.delx = -s.delx.abs();
            }
        }

        if s.x < SHIP_MIN_X && Self::wall_collide_hor(&mask, s.height, !(shl32(1, 31 - (SHIP_MIN_X - s.x - 1)).wrapping_sub(1))) {
            s.x = SHIP_MIN_X;
            if shield <= 0 && diff > 1 {
                self.ship_destroyed = true;
            } else {
                s.delx = s.delx.abs();
            }
        }

        if s.y < PAGE_HEIGHT / 2 + 7 && s.ybr > PAGE_HEIGHT / 2 - 7 {
            if s.x < SHIP_MIN_X + 6 {
                s.x = SHIP_MIN_X + 7;
                if shield <= 0 && diff > 1 {
                    self.ship_destroyed = true;
                } else {
                    s.delx = s.delx.abs();
                }
            }
            if s.xbr > SHIP_MAX_X - 6 {
                s.x = SHIP_MAX_X - 7 - s.width;
                if shield <= 0 && diff > 1 {
                    self.ship_destroyed = true;
                } else {
                    s.delx = -s.delx.abs();
                }
            }
        }

        let temp = s.x - (self.visible_page_x + PHYSICAL_PAGE_WIDTH - SCREEN_H_BORDER);
        if temp > 0 && self.visible_page_x < MAX_VISIBLE_PAGE_X {
            self.visible_page_x += temp / 20 + 1;
            if self.visible_page_x > MAX_VISIBLE_PAGE_X {
                self.visible_page_x = MAX_VISIBLE_PAGE_X;
            }
        }
        let temp = (self.visible_page_x + SCREEN_H_BORDER) - s.x;
        if temp > 0 && self.visible_page_x > 0 {
            self.visible_page_x -= temp / 20 + 1;
            if self.visible_page_x < 0 {
                self.visible_page_x = 0;
            }
        }

        if s.ybr > SHIP_MAX_Y && Self::wall_collide_ver(&mask, SHIP_MAX_Y - s.y + 1, s.height - 1) {
            s.y = SHIP_MAX_Y - s.height;
            if shield <= 0 && diff > 1 {
                self.ship_destroyed = true;
            } else {
                s.dely = -s.dely.abs();
            }
        }

        for gt in 0..2 {
            let g = self.gate[gt];
            let gmask = &self.g.gates[gt].mask;

            if g.x + g.width - 1 >= s.x && g.x <= s.xbr && g.height - 1 >= s.y {
                let hit = (s.x <= g.x && collide_bitmaps(&mask, s.height, s.x, s.y, gmask, g.height, g.x, 0))
                    || (g.x <= s.x && collide_bitmaps(gmask, g.height, g.x, 0, &mask, s.height, s.x, s.y));
                if hit {
                    s.y = g.height;
                    if shield <= 0 {
                        self.ship_destroyed = true;
                    } else {
                        s.dely = s.dely.abs();
                    }
                }
            }
        }

        if s.y < SHIP_MIN_Y {
            if s.x <= self.gate[0].x || s.xbr >= self.gate[1].x + self.gate[1].width {
                if Self::wall_collide_ver(&mask, 0, SHIP_MIN_Y - s.y - 1) {
                    s.y = SHIP_MIN_Y;
                    if shield <= 0 && diff > 1 {
                        self.ship_destroyed = true;
                    } else {
                        s.dely = s.dely.abs();
                    }
                }
            } else {
                if self.game_info[self.player].num_crystals > 0 && Self::wall_collide_ver(&mask, 0, 8 - s.y - 1) {
                    s.y = SHIP_MIN_Y;
                    if shield <= 0 {
                        self.ship_destroyed = true;
                    } else {
                        s.dely = s.dely.abs();
                    }
                }
                if s.y < 5 && !self.ship_destroyed {
                    self.level_finished = true;
                }
            }
        }

        let temp = s.y - (self.visible_page_y + SPLIT_SCREEN_LINE - SCREEN_V_BORDER);
        if temp > 0 && self.visible_page_y < MAX_VISIBLE_PAGE_Y {
            self.visible_page_y += temp / 20 + 1;
            if self.visible_page_y > MAX_VISIBLE_PAGE_Y {
                self.visible_page_y = MAX_VISIBLE_PAGE_Y;
            }
        }
        let temp = (self.visible_page_y + SCREEN_V_BORDER) - s.y;
        if temp > 0 && self.visible_page_y > 0 {
            self.visible_page_y -= temp / 20 + 1;
            if self.visible_page_y < 0 {
                self.visible_page_y = 0;
            }
        }
    }

    /// The repulsor and attractor force on the ship.
    fn repel(&mut self, deltax: i64, deltay: i64, sign: i64, sound: bool) {
        if deltax == 0 && deltay == 0 {
            return;
        }

        let modulus = if deltax.abs() > deltay.abs() {
            (deltax * deltax + deltay * deltay) * (deltax.abs() + deltay.abs() / 2)
        } else {
            (deltax * deltax + deltay * deltay) * (deltay.abs() + deltax.abs() / 2)
        };
        let modulus = modulus as i32 as i64;

        if modulus > 0 {
            if sound && modulus < 500000 && self.frame_count & 31 == 0 {
                self.play_sound(REPULSE);
            }
            self.ship.delx = w16((self.ship.delx as i64 + sign * ((8192 * deltax) as i32 as i64 / modulus)) as i32);
            self.ship.dely = w16((self.ship.dely as i64 + sign * ((8192 * deltay) as i32 as i64 / modulus)) as i32);
        }
    }

    fn demo_frame_initialise(&mut self) {
        self.demo_ptr += 1;
    }

    fn demo_rec(&self) -> DemoRec {
        self.demo.frames.get(self.demo_ptr - 1).copied().unwrap_or_default()
    }

    fn move_ship(&mut self) {
        if self.demo_mode {
            self.demo_frame_initialise();
            if self.demo_ptr > self.demo.frames.len() {
                // The original halts with "Demo file error".
                self.game_over = true;
                return;
            }
        } else if self.recording {
            self.demo_ptr += 1;
            self.demo.frames.push(DemoRec::default());
        }

        let (mut delx, mut dely) = (self.ship.delx, self.ship.dely);
        self.input_movement(&mut delx, &mut dely, true, false, false, true);
        self.ship.delx = delx;
        self.ship.dely = dely;

        if self.recording {
            let i = self.demo_ptr - 1;
            self.demo.frames[i] = DemoRec { delx: self.ship.delx, dely: self.ship.dely, but: 0 };
        } else if self.demo_mode {
            let r = self.demo_rec();
            self.ship.delx = r.delx;
            self.ship.dely = r.dely;
        }

        let temp = w16(self.ship.delx.abs() + self.ship.dely.abs());
        if temp > MAX_SHIP_SPEED {
            self.ship.delx = round(self.ship.delx as f64 / temp as f64 * MAX_SHIP_SPEED as f64);
            self.ship.dely = round(self.ship.dely as f64 / temp as f64 * MAX_SHIP_SPEED as f64);
        }

        for i in 1..=self.num_enemies() {
            if self.enemykind[self.enemy[i].ntyp].repulses {
                let dx = (self.ship.x - self.enemy[i].x) as i64;
                let dy = (self.ship.y - self.enemy[i].y) as i64;
                self.repel(dx, dy, 1, true);
            }
        }

        let gi = self.game_info[self.player];
        if gi.game_clocked > 0 {
            let dx = (self.ship.x - gi.attractor_x) as i64;
            let dy = (self.ship.y - gi.attractor_y) as i64;
            self.repel(dx, dy, -1, false);
        }

        {
            let s = &mut self.ship;
            s.sx = w16(s.sx + s.delx);
            s.sy = w16(s.sy + s.dely);
            s.oldx = s.x;
            s.oldy = s.y;
            s.x = s.sx / 64;
            s.y = s.sy / 64;
            s.xbr = s.x + s.width - 1;
            s.ybr = s.y + s.height - 1;

            let pi = std::f64::consts::PI;
            let mut theta = if s.dely != 0 {
                (s.delx as f64 / s.dely as f64).atan()
            } else if s.delx > 0 {
                pi / 2.0
            } else {
                -pi / 2.0
            };
            if s.dely < 0 {
                theta += 3.14159;
            }
            if theta < 0.0 {
                theta += 2.0 * pi;
            }

            s.ship_dir = round(theta / (2.0 * pi) * gfx::MAX_SHIP_PICS as f64);
            if s.ship_dir >= gfx::MAX_SHIP_PICS as i32 {
                s.ship_dir -= gfx::MAX_SHIP_PICS as i32;
            }
        }

        self.check_wall_collisions();
        self.decrement_power_ups();

        let but = self.button_pressed();
        let pi = *self.pi();
        let mut smartbut = but & (pi.mouse_smart_bomb_button | pi.joy_smart_bomb_button);
        let mut missbut = but & (pi.mouse_fire_button | pi.joy_fire_button);
        let but = self.button_down();

        if self.demo_mode {
            if (smartbut | missbut) != 0 {
                self.game_over = true;
                smartbut = 0;
                missbut = 0;
            }
        } else if self.kb.numkeypresses(pi.key_array[SMART_BOMB_KEY]) > 0 {
            smartbut = 1;
        }
        if self.kb.numkeypresses(pi.key_array[FIRE_KEY]) > 0 {
            missbut = 1;
        }

        let demo_but = if self.demo_mode { self.demo_rec().but } else { 0 };

        if missbut > 0 || (self.demo_mode && demo_but & 1 == 1) {
            if self.recording {
                let i = self.demo_ptr - 1;
                self.demo.frames[i].but |= 1;
            }
            self.shoot(w16(self.ship.delx << 1), w16(self.ship.dely << 1));
        } else if self.power_up[PowerUpType::RapidFire as usize].value > 0
            && self.frame_count & 3 == 0
            && ((but & (pi.mouse_fire_button | pi.joy_fire_button)) != 0 || (self.demo_mode && demo_but & 4 != 0))
        {
            if self.recording {
                let i = self.demo_ptr - 1;
                self.demo.frames[i].but |= 4;
            }
            self.shoot(w16(self.ship.delx << 1), w16(self.ship.dely << 1));
        }

        if smartbut > 0 || (self.demo_mode && demo_but & 2 == 2) {
            if self.recording {
                let i = self.demo_ptr - 1;
                self.demo.frames[i].but |= 2;
            }
            self.fire_smart_bomb();
        }
    }

    fn add_enemy_mine(&mut self, en: usize) {
        if self.emines_adding || self.num_enemy_mines >= MAX_ENEMY_MINES {
            return;
        }

        self.num_enemy_mines += 1;
        let e = &self.enemy[en];
        let w = self.enemykind[e.ntyp].width;
        let x = e.x + (w >> 1);
        let y = e.y + (w >> 1);
        let m = Point { x, y, xbr: x + self.g.emine.bm.width - 1, ybr: y + self.g.emine.bm.height - 1 };
        self.emines[self.num_enemy_mines] = m;

        let hidden = self.vga.hidden_page_offs;
        self.vga.put_cbitmap(x, y, hidden, &self.g.emine.bm);
        self.emines_adding = true;
        self.play_sound(SQUELCH);
    }

    fn set_enemy_gate(&mut self, left: bool, frame: usize) {
        let (p0, p1) = (self.vga.page0_offs, self.vga.page1_offs);
        if left {
            let pic = &self.g.l_enemy_gate[frame];
            self.vga.put_pbm(0, PAGE_HEIGHT / 2 - 10, p0, pic);
            self.vga.put_pbm(0, PAGE_HEIGHT / 2 - 10, p1, pic);
        } else {
            let pic = &self.g.r_enemy_gate[frame];
            self.vga.put_pbm(PAGE_WIDTH - 20, PAGE_HEIGHT / 2 - 10, p0, pic);
            self.vga.put_pbm(PAGE_WIDTH - 20, PAGE_HEIGHT / 2 - 10, p1, pic);
        }
    }

    fn move_enemies(&mut self) {
        let level_idx = (self.game_info[self.player].level - 1) as usize;
        let level = LEVELS[level_idx];
        let freq = DIFF_INFO[self.pi().diff_level as usize].enemyfrequency;

        for i in 0..=gfx::MAX_ENEMY_KINDS {
            if (self.num_enemies() as i32) < level.maxenemies
                && self.rng.real() < level.erelease * freq
                && self.rng.int(100) < PROBS[level_idx][i]
            {
                if self.rng.int(2) == 1 && self.enemy_entering_left <= 0 {
                    self.enemy_entering_left = 80;
                    self.enemy_left_type = i;
                    self.set_enemy_gate(true, 1);
                } else if self.enemy_entering_right <= 0 {
                    self.enemy_entering_right = 80;
                    self.enemy_right_type = i;
                    self.set_enemy_gate(false, 1);
                }
            }
        }

        if self.enemy_entering_left > 0 {
            self.enemy_entering_left -= 1;
            if self.enemy_entering_left & 7 == 0 {
                self.set_enemy_gate(true, ((self.enemy_entering_left >> 3) % 5 + 1) as usize);
            }
            if self.enemy_entering_left == 0 {
                self.add_enemy(15, ENEMY_START_Y, self.enemy_left_type);
                self.set_enemy_gate(true, 0);
                self.play_sound(ENEMYENT);
            }
        }

        if self.enemy_entering_right > 0 {
            self.enemy_entering_right -= 1;
            if self.enemy_entering_right & 7 == 0 {
                self.set_enemy_gate(false, ((self.enemy_entering_right >> 3) % 5 + 1) as usize);
            }
            if self.enemy_entering_right == 0 {
                let w = self.enemykind[self.enemy_right_type].width;
                self.add_enemy(PAGE_WIDTH - w - 16, ENEMY_START_Y, self.enemy_right_type);
                self.set_enemy_gate(false, 0);
                self.play_sound(ENEMYENT);
            }
        }

        let n = self.num_enemies();
        for i in 1..=n {
            let k = self.enemykind[self.enemy[i].ntyp].clone_params();

            if k.fires && self.rng.real() < k.fireprob {
                self.add_enemy_missile(i);
            }
            if k.laysmines && self.rng.real() < k.fireprob {
                self.add_enemy_mine(i);
            }

            let gs = self.game_speed;
            let (shx, shy, sdx, sdy) = (self.ship.x, self.ship.y, self.ship.delx, self.ship.dely);

            self.enemy[i].frame += k.framespeed;
            if self.enemy[i].frame >= (k.numframes + 1) << 8 {
                self.enemy[i].frame = 0;
            }

            if (k.follows && self.rng.real() < k.follow) && !self.ship_destroyed {
                let e = &mut self.enemy[i];
                let dx = shx - e.x;
                let dy = shy - e.y;
                let mut temp = w16(dx.abs() + dy.abs());
                if temp == 0 {
                    temp = 1;
                }
                e.delx = w16(dx * k.speed / temp * gs / 64);
                e.dely = w16(dy * k.speed / temp * gs / 64);
            } else if self.rng.real() < k.changedir {
                if k.zoom && w16(sdx.abs() + sdy.abs()) < 60 {
                    self.play_sound(BARK);
                    let e = &mut self.enemy[i];
                    let dx = w16(shx - e.x);
                    let dy = w16(shy - e.y);
                    let mut temp = w16(dx.abs() + dy.abs());
                    if temp == 0 {
                        temp = 1;
                    }
                    // All 16 bit integer arithmetic, which can overflow.
                    e.delx = w16(w16(w16(dx * k.speed) / temp) * gs) / 48;
                    e.dely = w16(w16(w16(dy * k.speed) / temp) * gs) / 48;
                } else if !k.maxspeed {
                    let a = self.rng.int(k.speed);
                    let b = self.rng.int(k.speed);
                    let e = &mut self.enemy[i];
                    e.delx = w16((a - k.speed2) * gs / 64);
                    e.dely = w16((b - k.speed2) * gs / 64);
                } else {
                    let theta = (self.rng.real() * 2.0 * std::f64::consts::PI - std::f64::consts::PI).cos();
                    let e = &mut self.enemy[i];
                    e.delx = w16(round(k.speed as f64 * theta) * gs / 64);
                    e.dely = w16(round(k.speed as f64 * (1.0 - theta * theta)) * gs / 64);
                }
            }

            if k.curves {
                if self.rng.real() < k.changecurve {
                    let cs = w16(-k.curve2 + self.rng.int(k.curve));
                    let e = &mut self.enemy[i];
                    e.curvesin = cs;
                    e.curvecos = round((1.0 - (cs as f64 / 32767.0).powi(2)).sqrt() * 32767.0);
                }

                let e = &mut self.enemy[i];
                let temp = e.delx;
                let ltemp = e.delx as i64 * e.curvecos as i64 - e.dely as i64 * e.curvesin as i64;
                let ltemp = ltemp as i32 as i64;
                e.delx = w16(if ltemp > 0 { (ltemp + 16384) / 32767 } else { (ltemp - 16384) / 32767 } as i32);
                let ltemp = e.dely as i64 * e.curvecos as i64 + temp as i64 * e.curvesin as i64;
                let ltemp = ltemp as i32 as i64;
                e.dely = w16(if ltemp > 0 { (ltemp + 16384) / 32767 } else { (ltemp - 16384) / 32767 } as i32);
            }

            let e = &mut self.enemy[i];
            e.sx = w16(e.sx + e.delx);
            e.sy = w16(e.sy + e.dely);
            e.oldx = e.x;
            e.oldy = e.y;
            e.x = e.sx / 64;
            e.y = e.sy / 64;
            e.xbr = e.x + k.width - 1;
            e.ybr = e.y + k.height - 1;

            if e.x < ENEMY_X_MIN {
                e.x = ENEMY_X_MIN;
                e.delx = e.delx.abs();
            }
            if e.xbr > ENEMY_X_MAX {
                e.x = ENEMY_X_MAX - k.width;
                e.delx = -e.delx.abs();
            }
            if e.y < ENEMY_Y_MIN {
                e.y = ENEMY_Y_MIN;
                e.dely = e.dely.abs();
            }
            if e.ybr > ENEMY_Y_MAX {
                e.y = ENEMY_Y_MAX - k.height;
                e.dely = -e.dely.abs();
            }
            if e.y < PAGE_HEIGHT / 2 + 7 && e.ybr > PAGE_HEIGHT / 2 - 7 {
                if e.x < ENEMY_X_MIN + 7 {
                    e.delx = e.delx.abs();
                }
                if e.xbr > ENEMY_X_MAX - 7 {
                    e.delx = -e.delx.abs();
                }
            }
        }

        let mut i = self.num_enemies();
        while i >= 1 {
            if self.enemy[i].supertime > 0 {
                self.enemy[i].supertime -= 1;
                if self.enemy[i].supertime <= 0 {
                    self.enemy_destroyed(i, 0, 0, false);
                }
            }
            i -= 1;
        }
    }

    fn update_missiles(&mut self) {
        let bounce = self.power_up[PowerUpType::Bounce as usize].value;
        let (mw, mh) = (self.ship.miss_width, self.ship.miss_height);

        for i in 1..=self.ship.num_missiles {
            let m = &mut self.missiles[i];
            m.oldx = m.x;
            m.oldy = m.y;
            m.sx = w16(m.sx + m.delx);
            m.x = (m.sx as u16 >> 6) as i32;
            m.xbr = m.x + mw;
            m.sy = w16(m.sy + m.dely);
            m.y = (m.sy as u16 >> 6) as i32;
            m.ybr = m.y + mh;
            m.time += 1;
        }

        let vis = self.vga.visible_page_offs;
        let mut cur = self.ship.num_missiles;
        while cur >= 1 {
            let m = &mut self.missiles[cur];
            let mut delete = false;

            if m.x < 10 {
                m.x = 11;
                if bounce == 0 { delete = true } else { m.delx = w16(-m.delx) }
            } else if m.xbr > PAGE_WIDTH - 10 {
                m.x = PAGE_WIDTH - 11;
                if bounce == 0 { delete = true } else { m.delx = w16(-m.delx) }
            } else if m.y < 10 {
                m.y = 11;
                if bounce == 0 { delete = true } else { m.dely = w16(-m.dely) }
            } else if m.ybr > PAGE_HEIGHT - 10 {
                m.y = PAGE_HEIGHT - 11;
                if bounce == 0 { delete = true } else { m.dely = w16(-m.dely) }
            } else if m.time > MISSILE_LIFE {
                delete = true;
            }

            if delete {
                let (ox, oy) = (m.oldx, m.oldy);
                let bg = m.oldbackgr.clone();
                self.vga.put_pbm(ox, oy, vis, &bg);
                let last = self.ship.num_missiles;
                self.missiles.swap(cur, last);
                self.ship.num_missiles -= 1;
            }

            cur -= 1;
        }

        for i in 1..=self.num_enemy_missiles {
            let m = &mut self.emissiles[i];
            let k = &self.emisskind[m.mtyp];
            m.oldx = m.x;
            m.oldy = m.y;
            m.sx = w16(m.sx + m.delx);
            m.x = (m.sx as u16 >> 6) as i32;
            m.xbr = m.x + k.sprite.bm.width;
            m.sy = w16(m.sy + m.dely);
            m.y = (m.sy as u16 >> 6) as i32;
            m.ybr = m.y + k.sprite.bm.height;
        }

        let mut cur = self.num_enemy_missiles;
        while cur >= 1 {
            let m = &mut self.emissiles[cur];
            let rebound = self.emisskind[m.mtyp].rebound;
            let out_x = m.x < 10 || m.xbr > PAGE_WIDTH - 10;
            let out_y = !out_x && (m.y < 10 || m.ybr > PAGE_HEIGHT - 10);

            if out_x || out_y {
                if rebound {
                    if out_x {
                        m.delx = w16(-m.delx);
                    } else {
                        m.dely = w16(-m.dely);
                    }
                    self.play_sound(BOING);
                } else {
                    let (ox, oy) = (m.oldx, m.oldy);
                    let bg = m.oldbackgr.clone();
                    self.vga.put_pbm(ox, oy, vis, &bg);
                    let last = self.num_enemy_missiles;
                    self.emissiles.swap(cur, last);
                    self.num_enemy_missiles -= 1;
                }
            }

            cur -= 1;
        }
    }

    fn object_hit(&mut self, i: usize) {
        let p = self.objects[i];

        match p.typ {
            ObjType::Crys => {
                self.add_to_score(200);
                self.play_sound(GETCRYSTAL);
                self.gi().num_crystals -= 1;
                self.show_crystals();

                if self.game_info[self.player].num_crystals == 0 {
                    if self.level().bonus_level {
                        self.level_finished = true;
                        let vis = self.vga.visible_page_offs;
                        let bg = self.obj_backgr[p.backgr].clone();
                        self.vga.put_pbm(p.x, p.y, vis, &bg);
                    } else {
                        let (gl, gr) = (self.gate[0], self.gate[1]);
                        let (vis, hid) = (self.vga.visible_page_offs, self.vga.hidden_page_offs);
                        self.vga.rect_fill(gl.x + gl.width, 0, gr.x, 11, vis, 0);
                        self.vga.rect_fill(gl.x + gl.width, 0, gr.x, 11, hid, 0);
                        self.play_sound(GATESOUND);
                    }
                }
            }
            ObjType::Mine => {
                self.ship_destroyed = true;
                self.add_enemy(p.x, p.y, 1);
                self.play_sound(EXPLOSN);
            }
            ObjType::Smart => {
                self.gi().num_smart_bombs += 1;
                self.show_smart_bombs();
                self.play_sound(ALLRIGHT);
                self.gi().num_smarts -= 1;
            }
        }

        let hid = self.vga.hidden_page_offs;
        let bg = self.obj_backgr[p.backgr].clone();
        self.vga.put_pbm(p.x, p.y, hid, &bg);
        if self.num_enemy_mines > 0 {
            self.fix_emines(p.x, p.y, p.xbr, p.ybr, hid);
        }
        self.objects[i].delete = true;
    }

    fn give_power_up(&mut self, pu: PowerUpType, special_given: &mut bool) {
        let p = self.power_up[pu as usize];
        if p.value <= 0 {
            let r = self.rng.int(p.time_ran);
            self.power_up[pu as usize].value = p.value + r + p.time_min;
            self.show_powerups();
            *special_given = true;
        }
    }

    fn check_collisions(&mut self) {
        let mut special_given = false;
        let dir = self.ship.ship_dir as usize;
        let smask = self.g.ship[dir].mask.clone();
        let sh = self.ship.height;

        if !self.ship_destroyed {
            let mut i = self.num_enemies();
            while i >= 1 {
                let e = &self.enemy[i];
                let ntyp = e.ntyp;

                if ntyp != 1 {
                    let k = &self.enemykind[ntyp];
                    let s = &self.ship;
                    let emask = &k.frames[(e.frame >> 8) as usize].mask;

                    if e.xbr >= s.x
                        && e.ybr >= s.y
                        && e.x <= s.xbr
                        && e.y <= s.ybr
                        && ((s.x <= e.x && collide_bitmaps(&smask, sh, s.x, s.y, emask, k.height, e.x, e.y))
                            || (e.x < s.x && collide_bitmaps(emask, k.height, e.x, e.y, &smask, sh, s.x, s.y)))
                    {
                        if ntyp > 1 && !self.ship_destroyed {
                            self.ship_destroyed = true;
                            let (dx, dy) = (self.ship.delx, self.ship.dely);
                            self.enemy_destroyed(i, dx, dy, true);
                        } else if ntyp == 0 {
                            loop {
                                match self.rng.int(22) {
                                    0..=2 => self.give_power_up(PowerUpType::RapidFire, &mut special_given),
                                    3..=5 => self.give_power_up(PowerUpType::MultiFire, &mut special_given),
                                    6..=8 => self.give_power_up(PowerUpType::HeavyFire, &mut special_given),
                                    9..=11 => self.give_power_up(PowerUpType::AssFire, &mut special_given),
                                    12..=13 => self.give_power_up(PowerUpType::AimedFire, &mut special_given),
                                    14..=15 => self.give_power_up(PowerUpType::Bounce, &mut special_given),
                                    16..=17 => {
                                        if !self.no_mines {
                                            if self.shield_value() == 0 {
                                                self.power_up[PowerUpType::Shield as usize].value = 1;
                                            }
                                            for j in 1..=self.game_info[self.player].num_objects as usize {
                                                if self.objects[j].typ == ObjType::Mine {
                                                    self.object_hit(j);
                                                }
                                            }
                                            self.no_mines = true;
                                            self.smart_bombed = 11;
                                            special_given = true;
                                        }
                                    }
                                    18 => {
                                        let p = self.power_up[PowerUpType::Shield as usize];
                                        let r = self.rng.int(p.time_ran);
                                        self.power_up[PowerUpType::Shield as usize].value = p.value + r + p.time_min;
                                        self.show_powerups();
                                        special_given = true;
                                    }
                                    _ => {
                                        if self.gate_moving {
                                            self.gate_moving = false;
                                            let (vis, hid) = (self.vga.visible_page_offs, self.vga.hidden_page_offs);
                                            for gt in 0..2 {
                                                let gx = self.gate[gt].x;
                                                self.vga.put_pbm(gx, 0, vis, &self.g.gates[gt].bm);
                                                self.vga.put_pbm(gx, 0, hid, &self.g.gates[gt].bm);
                                            }
                                            special_given = true;
                                        }
                                    }
                                }
                                if special_given {
                                    break;
                                }
                            }
                            self.enemy_destroyed(i, 0, 0, false);
                            self.play_sound(OHYEAH);
                        }
                    }
                }

                i -= 1;
            }

            for i in 1..=self.num_enemy_mines {
                let m = self.emines[i];
                let s = &self.ship;
                let em = &self.g.emine;
                if m.xbr >= s.x
                    && m.ybr >= s.y
                    && m.x <= s.xbr
                    && m.y <= s.ybr
                    && ((s.x <= m.x && collide_bitmaps(&smask, sh, s.x, s.y, &em.mask, em.bm.height, m.x, m.y))
                        || (m.x < s.x && collide_bitmaps(&em.mask, em.bm.height, m.x, m.y, &smask, sh, s.x, s.y)))
                {
                    self.ship_destroyed = true;
                }
            }

            let mut i = self.game_info[self.player].num_objects;
            while i >= 1 {
                let p = self.objects[i as usize];
                let s = &self.ship;
                let o = &self.g.objects[p.typ as usize];
                if p.xbr >= s.x
                    && p.ybr >= s.y
                    && p.x <= s.xbr
                    && p.y <= s.ybr
                    && ((s.x <= p.x && collide_bitmaps(&smask, sh, s.x, s.y, &o.mask, o.bm.height, p.x, p.y))
                        || (p.x < s.x && collide_bitmaps(&o.mask, o.bm.height, p.x, p.y, &smask, sh, s.x, s.y)))
                {
                    self.object_hit(i as usize);
                }
                i -= 1;
            }
        }

        let mmask = self.g.missile.mask.clone();
        let mh = self.ship.miss_height;
        for i in 1..=self.ship.num_missiles {
            let mut j = self.num_enemies();
            while j >= 1 {
                let e = &self.enemy[j];
                if e.ntyp != 1 {
                    let k = &self.enemykind[e.ntyp];
                    let emask = &k.frames[(e.frame >> 8) as usize].mask;
                    let m = &self.missiles[i];

                    if m.xbr > e.x
                        && m.x < e.xbr
                        && m.ybr > e.y
                        && m.y < e.ybr
                        && ((m.x <= e.x && collide_bitmaps(&mmask, mh, m.x, m.y, emask, k.height, e.x, e.y))
                            || (e.x < m.x && collide_bitmaps(emask, k.height, e.x, e.y, &mmask, mh, m.x, m.y)))
                    {
                        let (dx, dy) = (m.delx, m.dely);
                        self.enemy_destroyed(j, dx, dy, true);
                        self.missiles[i].time = 10000;
                    }
                }
                j -= 1;
            }
        }

        let vis = self.vga.visible_page_offs;
        let mut i = self.num_enemy_missiles;
        while i >= 1 {
            let m = &self.emissiles[i];
            let k = &self.emisskind[m.mtyp].sprite;
            let s = &self.ship;
            if m.xbr >= s.x
                && m.ybr >= s.y
                && m.x <= s.xbr
                && m.y <= s.ybr
                && ((s.x <= m.x && collide_bitmaps(&smask, sh, s.x, s.y, &k.mask, k.bm.height, m.x, m.y))
                    || (m.x < s.x && collide_bitmaps(&k.mask, k.bm.height, m.x, m.y, &smask, sh, s.x, s.y)))
            {
                self.ship_destroyed = true;
                let (ox, oy) = (m.oldx, m.oldy);
                let bg = m.oldbackgr.clone();
                self.vga.put_pbm(ox, oy, vis, &bg);
                let last = self.num_enemy_missiles;
                self.emissiles.swap(i, last);
                self.num_enemy_missiles -= 1;
            }
            i -= 1;
        }
    }

    // ------------------------------------------------------------------------------------
    // Drawing.
    // ------------------------------------------------------------------------------------

    fn erase_sprites(&mut self) {
        let hid = self.vga.hidden_page_offs;

        self.vga.put_pbm(self.ship.oldx, self.ship.oldy, hid, &self.ship.backgr);

        for i in (1..=self.ship.num_missiles).rev() {
            let m = &self.missiles[i];
            self.vga.put_pbm(m.oldx, m.oldy, hid, &m.backgr);
        }

        for i in (1..=self.num_enemies()).rev() {
            let e = &self.enemy[i];
            self.vga.put_pbm(e.oldx, e.oldy, hid, &e.backgr);
        }

        for i in (1..=self.num_enemy_missiles).rev() {
            let m = &self.emissiles[i];
            self.vga.put_pbm(m.oldx, m.oldy, hid, &m.backgr);
        }

        if self.gate_moving {
            for gt in 0..2 {
                let g = self.gate[gt];
                self.vga.rect_fill(g.x, 0, g.x + g.width, g.height, hid, 0);
            }
        }

        if self.show_count_down {
            let t = &self.time_record;
            self.vga.put_pbm(t.oldx, t.oldy, hid, &t.backgr);
        }

        let mut i = self.game_info[self.player].num_objects;
        while i >= 1 {
            if self.objects[i as usize].delete {
                let p = self.objects[i as usize];
                let bg = self.obj_backgr[p.backgr].clone();
                self.vga.put_pbm(p.x, p.y, hid, &bg);
                if self.num_enemy_mines > 0 {
                    self.fix_emines(p.x, p.y, p.xbr, p.ybr, hid);
                }
                let n = self.game_info[self.player].num_objects as usize;
                self.objects.swap(i as usize, n);
                self.gi().num_objects -= 1;
            }
            i -= 1;
        }

        if self.emines_adding {
            let m = self.emines[self.num_enemy_mines];
            self.vga.put_cbitmap(m.x, m.y, hid, &self.g.emine.bm);
            self.emines_adding = false;
        }
    }

    fn display_time(&mut self) {
        if !self.show_count_down {
            return;
        }

        let gi = self.game_info[self.player];
        let mut secs = self.level().time as i64 * FRAME_RATE as i64 - gi.time_on_level;
        if secs < 0 {
            return;
        }

        let bits = secs % FRAME_RATE as i64;
        let bits = (bits as f64 * 100.0 / FRAME_RATE as f64).trunc() as i64;
        secs /= FRAME_RATE as i64;
        let mins = secs / 60;
        secs %= 60;

        let str2 = |i: i64| format!("{:02}", i);
        let mut s = str2(bits);
        if mins > 0 || secs > 0 {
            let temp = if mins > 0 { str2(secs) } else { secs.to_string() };
            s = format!("{temp}:{s}");
            if mins > 0 {
                s = format!("{mins}:{s}");
            }
        }
        let s = format!("{:>7}", s);

        let hid = self.vga.hidden_page_offs;
        let (x, y) = (self.time_record.x, self.time_record.y);
        for (i, ch) in s.bytes().enumerate().take(7) {
            if ch.is_ascii_digit() {
                self.vga.put_cbitmap(x + i as i32 * 5, y, hid, &self.g.small_font[(ch - b'0') as usize]);
            }
        }
    }

    fn draw_sprites(&mut self) {
        let hid = self.vga.hidden_page_offs;

        if self.show_count_down {
            let t = &mut self.time_record;
            t.oldx = t.x;
            t.oldy = t.y;
            t.x = (self.visible_page_x + TIMER_X).min(PAGE_WIDTH - 46);
            t.y = self.visible_page_y + TIMER_Y;
            t.backgr = self.vga.get_pbm(t.x, t.y, 9, self.g.small_font[0].height, hid);
            std::mem::swap(&mut t.backgr, &mut t.oldbackgr);
        }

        if w16(self.ship.y) >= 0 {
            let s = &mut self.ship;
            s.backgr = self.vga.get_pbm(s.x, s.y, s.bmwidth, s.height, hid);
            std::mem::swap(&mut s.backgr, &mut s.oldbackgr);
        }

        let mh = self.ship.miss_height;
        for i in (1..=self.ship.num_missiles).rev() {
            let m = &mut self.missiles[i];
            m.backgr = self.vga.get_pbm(m.x, m.y, 1, mh, hid);
            std::mem::swap(&mut m.backgr, &mut m.oldbackgr);
        }

        for i in (1..=self.num_enemies()).rev() {
            let e = &mut self.enemy[i];
            let k = &self.enemykind[e.ntyp];
            e.backgr = self.vga.get_pbm(e.x, e.y, k.bmwidth, k.height, hid);
            std::mem::swap(&mut e.backgr, &mut e.oldbackgr);
        }

        for i in (1..=self.num_enemy_missiles).rev() {
            let m = &mut self.emissiles[i];
            let h = self.emisskind[m.mtyp].sprite.bm.height;
            m.backgr = self.vga.get_pbm(m.x, m.y, 1, h, hid);
            std::mem::swap(&mut m.backgr, &mut m.oldbackgr);
        }

        if self.gate_moving {
            for gt in 0..2 {
                self.vga.put_pbm(self.gate[gt].x, 0, hid, &self.g.gates[gt].bm);
            }
        }

        if w16(self.ship.y) >= 0 && self.ship_destroyed_count > START_SHIP_DESTROYED_COUNT - 2 {
            let s = &self.ship;
            self.vga.put_cbitmap(s.x, s.y, hid, &self.g.ship[s.ship_dir as usize].bm);
        }

        for i in (1..=self.ship.num_missiles).rev() {
            let m = &self.missiles[i];
            self.vga.put_cbitmap(m.x, m.y, hid, &self.g.missile.bm);
        }

        for i in (1..=self.num_enemies()).rev() {
            let e = &self.enemy[i];
            let k = &self.enemykind[e.ntyp];
            let frame = ((e.frame >> 8) & 0xFF) as usize;
            if let Some(f) = k.frames.get(frame) {
                self.vga.put_cbitmap(e.x, e.y, hid, &f.bm);
            }
        }

        for i in (1..=self.num_enemy_missiles).rev() {
            let m = &self.emissiles[i];
            self.vga.put_cbitmap(m.x, m.y, hid, &self.emisskind[m.mtyp].sprite.bm);
        }

        self.display_time();
    }

    // ------------------------------------------------------------------------------------
    // Lives, pausing and the main game loop.
    // ------------------------------------------------------------------------------------

    async fn new_life(&mut self) {
        self.gi().lives -= 1;

        if self.two_player {
            if self.player == 0 {
                if self.game_info[1].lives > 0 {
                    self.player = 1;
                    self.swap_objects();
                }
            } else if self.game_info[0].lives > 0 {
                self.player = 0;
                self.swap_objects();
            }
        }

        self.game_speed = self.base_game_speed();

        if self.game_info[0].lives == 0 && (!self.two_player || self.game_info[1].lives == 0) {
            self.game_over = true;
        } else {
            for pu in self.power_up.iter_mut() {
                pu.value = 0;
                pu.position = 0;
            }
            self.ship_destroyed_count = START_SHIP_DESTROYED_COUNT;
            self.start_new_level().await;

            if self.two_player {
                self.text_window(&format!("{} READY", PLAYER_STR[self.player]));
            } else {
                self.text_window("READY");
            }

            if self.demo_mode {
                self.my_delay(300).await;
            } else {
                loop {
                    if self.kb.keypressed() || self.button_pressed() != 0 {
                        break;
                    }
                    self.vsync().await;
                }
            }

            self.clear_input_buffers(false).await;
            self.remove_text_window();
            self.mouse.mmotion();
        }
    }

    async fn pause_game(&mut self, message: &str) {
        self.gi().timing = false;
        self.text_window(message);
        self.wait_for_event(false).await;
        self.remove_text_window();
        self.gi().timing = true;
        self.mouse.mmotion();
    }

    fn clear_enemies(&mut self) {
        for i in (1..=self.num_enemies()).rev() {
            self.delete_enemy(i);
        }
    }

    async fn level_over(&mut self) {
        self.play_sound(PHEW);
        self.give_bonus().await;

        let gi = self.gi();
        gi.level += 1;
        gi.total_level += 1;

        if gi.level > MAX_LEVEL as i32 {
            if gi.game_clocked < MAX_GAME_CLOCKED {
                gi.game_clocked += 1;
            }
            self.show_game_clocked_message().await;
            self.gi().level = 1;
        }

        self.setup_new_level(self.player);
        self.start_new_level().await;
    }

    pub async fn game(&mut self) {
        self.stars.finished();
        self.initialise_variables();

        let (p0, p1) = (self.vga.page0_offs, self.vga.page1_offs);
        let lh = self.vga.logical_height;
        self.vga.rect_fill(0, 0, PAGE_WIDTH, lh, p0, 0);
        self.vga.rect_fill(0, 0, PAGE_WIDTH, lh, p1, 0);

        self.vga.set_mode_split_double_buffer(PAGE_HEIGHT);
        self.vga.put_pal_struc(&data::palette());

        if self.demo_mode || self.recording {
            self.rng.seed = self.saved_rand_seed;
        }
        self.demo_ptr = 0;
        self.set_mouse_grab(true);

        self.setup_new_level(0);
        if self.two_player {
            self.swap_objects();
            self.setup_new_level(1);
            self.swap_objects();
        }

        self.start_new_level().await;

        if self.two_player {
            self.text_window(&format!("{} READY", PLAYER_STR[self.player]));
        } else {
            self.text_window("READY");
        }
        if self.demo_mode {
            self.my_delay(300).await;
        } else {
            self.wait_for_event(false).await;
        }
        self.remove_text_window();

        loop {
            self.gi().timing = true;
            self.frame_count = 0;

            loop {
                self.erase_sprites();
                self.move_enemies();
                self.update_missiles();
                self.check_collisions();

                if self.ship_destroyed {
                    if self.shield_value() > 0 {
                        self.ship_destroyed = false;
                    } else {
                        self.ship.oldx = self.ship.x;
                        self.ship.oldy = self.ship.y;
                        if self.ship_destroyed_count == START_SHIP_DESTROYED_COUNT - 1 {
                            self.play_sound(EXPLOSN);
                            let (x, y) = (self.ship.x, self.ship.y);
                            self.add_enemy(x, y, 1);
                        }
                        self.ship_destroyed_count -= 1;
                    }
                }

                if !self.ship_destroyed {
                    self.move_ship();
                }

                if self.smart_bombed > 1 && self.frame_count & 1 == 0 {
                    self.smart_bombed -= 1;
                    let [r, g, b] = SMART_BOMB_PAL[self.smart_bombed as usize];
                    self.vga.set_rgb(0, r, g, b);
                }

                self.draw_sprites();

                if self.gate_moving {
                    self.gate_move_count += self.level().gate_move;
                    if self.gate_move_count > 63 {
                        self.move_gate();
                    }
                }

                self.xpage_flip().await;
                self.frame_count += 1;
                if self.game_info[self.player].timing {
                    self.gi().time_on_level += 1;
                }

                if self.kb.keypressed() {
                    if self.kb.numkeypresses(scan::ESC) > 0 && !self.recording {
                        self.clear_input_buffers(false).await;
                        self.set_mouse_grab(false);
                        self.pause_game("QUIT?").await;
                        self.set_mouse_grab(true);
                        self.game_over = self.kb.last_key_hit.eq_ignore_ascii_case(&b'y');
                        if self.game_over {
                            self.gi().score = 0;
                        }
                    }
                    if self.kb.numkeypresses(scan::P) > 0 {
                        self.set_mouse_grab(false);
                        self.pause_game("PAUSE").await;
                        self.set_mouse_grab(true);
                    }
                    if self.kb.keys[scan::EQUAL as usize] || self.kb.keys[scan::KEYPAD_PLUS as usize] {
                        self.sound_volume = (self.sound_volume + 4).min(crate::core::sound::MAX_SOUND_VOLUME);
                        self.mixer.lock().unwrap().set_volume(self.sound_volume);
                    }
                    if self.kb.keys[scan::MINUS as usize] || self.kb.keys[scan::KEYPAD_MINUS as usize] {
                        self.sound_volume = (self.sound_volume - 4).max(0);
                        self.mixer.lock().unwrap().set_volume(self.sound_volume);
                    }
                    if self.kb.numkeypresses(scan::S) > 0 {
                        self.sounds_on = !self.sounds_on;
                    }
                }

                let gi = self.game_info[self.player];
                let level = *self.level();
                if gi.time_on_level > level.time as i64 * FRAME_RATE as i64 {
                    if level.bonus_level {
                        self.level_finished = true;
                    }
                    self.game_speed = self.base_game_speed()
                        + ((gi.time_on_level - level.time as i64 * FRAME_RATE as i64) * BASE_GAME_SPEED as i64
                            / (FRAME_RATE as i64 * 32)) as i32;
                }

                if self.level_finished || self.ship_destroyed_count == 0 || self.game_over {
                    break;
                }
            }

            self.gi().timing = false;
            self.clear_enemies();

            if self.level_finished {
                self.level_over().await;
            } else if self.ship_destroyed {
                self.new_life().await;
            }

            if self.game_over {
                break;
            }
        }

        self.set_mouse_grab(false);
        self.end_screen().await;
    }

    pub fn initialise_variables(&mut self) {
        self.rng.seed = storage::random_seed();
        self.player = 0;

        for pl in 0..2 {
            let gi = &mut self.game_info[pl];
            gi.time_on_level = 0;
            gi.timing = false;
            gi.level = self.start_level;
            gi.total_level = gi.level;
            gi.score = 0;
            gi.last_new_man_score = gi.score;
            gi.num_smart_bombs = self.start_bombs;
            gi.lives = self.start_lives;
            gi.game_clocked = 0;
        }

        self.game_over = false;
        self.smart_bombed = 0;
        for pu in self.power_up.iter_mut() {
            pu.value = 0;
            pu.position = 0;
        }
        self.ship_destroyed_count = START_SHIP_DESTROYED_COUNT;
        self.enemy_entering_left = 0;
        self.enemy_entering_right = 0;

        for i in 1..=15 {
            let a = i as f64 * 168.0 * std::f64::consts::PI / 180.0;
            self.cost[i] = round(a.cos() * 32767.0);
            self.sint[i] = round(a.sin() * 32767.0);
        }

        let kinds = [
            (120, FIRE6, false, false),
            (150, FIRE5, false, false),
            (200, RETALIATE, false, true),
            (150, 0, true, false),
            (150, FIRE4, false, true),
            (170, 0, false, false),
        ];
        for (i, (mspeed, soundnum, rebound, firedirect)) in kinds.into_iter().enumerate() {
            let k = &mut self.emisskind[i + 1];
            k.mspeed = mspeed;
            k.soundnum = soundnum;
            k.rebound = rebound;
            k.firedirect = firedirect;
        }
    }
}

trait CloneParams {
    fn clone_params(&self) -> EnemyParams;
}

/// The scalar fields of an [`EnemyKind`], copied so the enemy list can be mutated.
#[derive(Clone, Copy)]
struct EnemyParams {
    speed: i32,
    speed2: i32,
    curve: i32,
    curve2: i32,
    fires: bool,
    follows: bool,
    curves: bool,
    laysmines: bool,
    zoom: bool,
    maxspeed: bool,
    fireprob: f64,
    changedir: f64,
    changecurve: f64,
    follow: f64,
    width: i32,
    height: i32,
    numframes: i32,
    framespeed: i32,
}

impl CloneParams for EnemyKind {
    fn clone_params(&self) -> EnemyParams {
        EnemyParams {
            speed: self.speed,
            speed2: self.speed2,
            curve: self.curve,
            curve2: self.curve2,
            fires: self.fires,
            follows: self.follows,
            curves: self.curves,
            laysmines: self.laysmines,
            zoom: self.zoom,
            maxspeed: self.maxspeed,
            fireprob: self.fireprob,
            changedir: self.changedir,
            changecurve: self.changecurve,
            follow: self.follow,
            width: self.width,
            height: self.height,
            numframes: self.numframes,
            framespeed: self.framespeed,
        }
    }
}

/// `CollideBitmaps`: pixel collision between two masks whose bounding boxes overlap.
/// The first mask must be the leftmost one.
#[allow(clippy::too_many_arguments)]
pub fn collide_bitmaps(mask1: &[u32], height1: i32, x1: i32, y1: i32, mask2: &[u32], height2: i32, x2: i32, y2: i32) -> bool {
    let shift = (x2 - x1) as u32;
    let dy = y2 - y1;

    let (start1, start2, rows) = if dy >= 0 {
        (dy, 0, (height1 - dy).min(height2))
    } else {
        (0, -dy, (height2 + dy).min(height1))
    };

    for r in 0..rows.max(0) {
        let a = mask1.get((start1 + r) as usize).copied().unwrap_or(0);
        let b = mask2.get((start2 + r) as usize).copied().unwrap_or(0);
        let shifted = if shift < 32 { a << shift } else { 0 };
        if shifted & b != 0 {
            return true;
        }
    }

    false
}
