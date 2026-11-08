//! The menus.

use super::*;
use crate::core::demo::Demo;
use crate::core::input::MouseButton;
use crate::core::settings::{InputDevice, Settings};
use crate::core::sound::{MAX_VOICES, MAX_VOLUME};
use crate::core::storage;

#[derive(Clone, Debug)]
pub enum MenuItem {
    Item(String),
    Slider {
        name: String,
        min: i32,
        max: i32,
        value: i32,
    },
}

impl MenuItem {
    pub fn item(name: impl Into<String>) -> Self {
        MenuItem::Item(name.into())
    }

    pub fn slider(name: impl Into<String>, min: i32, max: i32, value: i32) -> Self {
        MenuItem::Slider {
            name: name.into(),
            min,
            max,
            value,
        }
    }

    pub fn value(&self) -> i32 {
        match self {
            MenuItem::Slider { value, .. } => *value,
            MenuItem::Item(_) => 0,
        }
    }

    /// The horizontal mouse position, -64 to 64, that corresponds to a slider's value.
    fn slider_position(&self) -> i32 {
        match *self {
            MenuItem::Slider {
                min, max, value, ..
            } => ((value - min) as f64 * 128.0 / (max - min) as f64 - 64.0).round() as i32,
            MenuItem::Item(_) => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuResult {
    Selected(usize),
    Cancelled,
    TimedOut,
}

/// Recorded demos are stored with this extension.
const DEMO_EXTENSION: &str = ".demo";
const BUILTIN_DEMO: &str = "XQuest";

impl XQuest {
    fn draw_menu_item(&mut self, item: &MenuItem, y: i32, highlight: bool) {
        let color = if highlight {
            HIGHLIGHT_COLOR
        } else {
            TEXT_COLOR
        };

        match item {
            MenuItem::Item(name) => self.print_centred(160, y, Page::Visible, color, name),
            MenuItem::Slider {
                name,
                min,
                max,
                value,
            } => {
                let x = 160 - self.text_width(name);
                self.print(x, y, Page::Visible, color, name);

                let fill = (100.0 * (value - min) as f64 / (max - min) as f64).round() as i32;
                let (x, page) = (170, Page::Visible);
                self.vga.fill_rect(x - 1, y - 1, x + 101, y + 12, page, 27);
                self.vga.fill_rect(x + fill, y, x + 100, y + 11, page, 0);
                self.vga.fill_rect(x, y, x + fill, y + 11, page, 22);
            }
        }
    }

    /// Shows a menu over the starfield. Items are chosen with the mouse (moving it up and
    /// down, left button to select, right to cancel) or the keyboard. Sliders are set by
    /// moving the mouse with the left button held, or with the left and right arrow keys.
    pub async fn menu(
        &mut self,
        items: &mut [MenuItem],
        initial: usize,
        timeout_seconds: i64,
    ) -> MenuResult {
        /// The highlight colour pulses through these colours.
        const PULSE: [[u8; 3]; 13] = [
            [45, 45, 45],
            [40, 40, 48],
            [35, 35, 51],
            [30, 30, 54],
            [25, 25, 57],
            [20, 20, 60],
            [15, 15, 63],
            [20, 20, 60],
            [25, 25, 57],
            [30, 30, 54],
            [35, 35, 51],
            [40, 40, 48],
            [45, 45, 45],
        ];

        self.clear_input(true).await;
        let timeout = timeout_seconds * crate::core::game::NOMINAL_FRAME_RATE as i64;
        let top = (7 - items.len() as i32) * 10 + 80;
        let item_y = |i: usize| top + i as i32 * 20;

        self.clear_area(0, 80, 320, 240);

        let mut selected = initial;
        let mut frame_count = 0;
        let mut pulse = 0;
        let mut my = 0;

        for (i, item) in items.iter().enumerate() {
            self.draw_menu_item(item, item_y(i), i == selected);
        }
        let mut mx = items[selected].slider_position();

        let (pressed, enter) = loop {
            for (up, keys) in [
                (true, [KeyCode::ArrowUp, KeyCode::Numpad8]),
                (false, [KeyCode::ArrowDown, KeyCode::Numpad2]),
            ] {
                let moved = if up { my < -40 } else { my > 40 };
                if moved || self.keyboard.take_any_press(&keys) {
                    let can_move = if up {
                        selected > 0
                    } else {
                        selected < items.len() - 1
                    };
                    if can_move {
                        pulse = PULSE.len() * 3 / 2;
                        self.draw_menu_item(&items[selected], item_y(selected), false);
                        selected = if up { selected - 1 } else { selected + 1 };
                        self.play(crate::core::sound::Sound::MenuClick);
                    }
                    mx = items[selected].slider_position();
                    my = 0;
                }
            }

            self.draw_menu_item(&items[selected].clone(), item_y(selected), true);

            let held = loop {
                pulse += 1;
                if pulse >= PULSE.len() * 3 {
                    pulse = 1;
                }
                let c = PULSE[pulse / 3];
                self.vga.set_color(HIGHLIGHT_COLOR + 1, c);
                self.vga.set_color(HIGHLIGHT_COLOR + 2, c.map(|v| v / 2));

                self.star_field_step().await;
                frame_count += 1;
                self.input_movement(&mut mx, &mut my, true, false);
                let held = self.buttons_down();

                if held.any()
                    || self.keyboard.take_any_key()
                    || !(-40..=40).contains(&my)
                    || frame_count > timeout
                {
                    break held;
                }
            };

            if let MenuItem::Slider {
                min, max, value, ..
            } = &mut items[selected]
            {
                if held.left {
                    *value = ((mx + 64) as f64 * (*max - *min) as f64 / 128.0 + *min as f64).round()
                        as i32;
                }
                let step = ((*max - *min) as f64 / 50.0).round() as i32 + 1;
                if self
                    .keyboard
                    .take_any_press(&[KeyCode::ArrowLeft, KeyCode::Numpad4])
                {
                    *value -= step;
                } else if self
                    .keyboard
                    .take_any_press(&[KeyCode::ArrowRight, KeyCode::Numpad6])
                {
                    *value += step;
                }
                *value = (*value).clamp(*min, *max);
                mx = mx.clamp(-64, 64);
            }

            let pressed = self.buttons_pressed();
            let enter = self.keyboard.take_press(KeyCode::Enter);
            let is_item = matches!(items[selected], MenuItem::Item(_));

            if pressed.right
                || (pressed.left && is_item)
                || self.keyboard.take_press(KeyCode::Escape)
                || enter
                || frame_count > timeout
            {
                break (pressed, enter);
            }
        };

        self.draw_menu_item(&items[selected].clone(), item_y(selected), false);

        if pressed.left || enter {
            MenuResult::Selected(selected)
        } else if frame_count > timeout {
            MenuResult::TimedOut
        } else {
            MenuResult::Cancelled
        }
    }

    // ------------------------------------------------------------------------------------
    // The main menu.
    // ------------------------------------------------------------------------------------

    /// The whole program: the title pages, then the main menu until the player quits.
    pub async fn run(mut self) {
        self.settings = Settings::load();
        self.apply_sound_settings();

        self.vga.palette = self.gfx.game_palette;
        self.title_pages().await;
        self.main_menu().await;
        self.settings.save();

        self.shared.lock().unwrap().finished = true;
    }

    fn apply_sound_settings(&mut self) {
        let mut mixer = self.mixer.lock().unwrap();
        mixer.set_volume(self.settings.volume);
        mixer.max_voices = self.settings.max_sounds;
        if !self.settings.sound_enabled {
            mixer.stop_all();
        }
    }

    async fn main_menu(&mut self) {
        let mut items = [
            "Start Game",
            "Help",
            "Hall of Fame",
            "Options",
            "Demo",
            "Quit",
        ]
        .map(MenuItem::item);
        self.menu_screen().await;
        let mut selected = 0;

        loop {
            let result = self.menu(&mut items, selected, 30).await;

            match result {
                MenuResult::TimedOut => {
                    self.play_demo(crate::core::assets::builtin_demo()).await;
                    selected = 0;
                }
                MenuResult::Selected(i) => {
                    selected = i;
                    match i {
                        0 => self.game().await,
                        1 => self.help().await,
                        2 => self.high_scores(None, 120).await,
                        3 => self.options_menu().await,
                        4 => self.demo_menu().await,
                        _ => {}
                    }
                }
                MenuResult::Cancelled => {}
            }

            if matches!(result, MenuResult::Selected(5) | MenuResult::Cancelled) {
                self.clear_area(10, 90, 230, 310);
                self.print_over_stars(160, 120, TEXT_COLOR, "Are you sure you");
                self.print_over_stars(160, 140, TEXT_COLOR, "want to quit?");
                if self.yes_or_no(false).await {
                    break;
                }
            }
        }
    }

    // ------------------------------------------------------------------------------------
    // Options.
    // ------------------------------------------------------------------------------------

    async fn options_menu(&mut self) {
        let mut items = vec![
            MenuItem::item(""),
            MenuItem::item("Sound Setup"),
            MenuItem::slider("Horizontal Sensitivity", 4, 256, 64),
            MenuItem::slider(" Vertical Sensitivity ", 4, 256, 64),
            MenuItem::item(""),
            MenuItem::item(""),
            MenuItem::item("Input Device Setup"),
        ];
        let mut selected = 0;

        loop {
            let p = *self.player_settings();
            items[0] = MenuItem::item(match (self.settings.two_players, self.player) {
                (false, _) => "One Player",
                (true, 0) => "Two Players: Player One",
                (true, _) => "Two Players: Player Two",
            });
            items[2] = MenuItem::slider("Horizontal Sensitivity", 4, 256, p.horizontal_speed);
            items[3] = MenuItem::slider(" Vertical Sensitivity ", 4, 256, p.vertical_speed);
            items[4] = MenuItem::item(format!("Difficulty: {}", p.difficulty.name()));
            let device = p.input_device;
            items[5] = MenuItem::item(match device {
                InputDevice::Mouse => "Input Device: Mouse",
                InputDevice::Keyboard => "Input Device: Keyboard",
            });

            let result = self.menu(&mut items, selected, 60).await;
            let mut new_device = device;

            match result {
                MenuResult::Selected(1) => self.sound_menu().await,
                MenuResult::Selected(4) => {
                    let p = &mut self.settings.players[self.player];
                    p.difficulty = p.difficulty.next();
                }
                MenuResult::Selected(5) => {
                    new_device = match device {
                        InputDevice::Mouse => InputDevice::Keyboard,
                        InputDevice::Keyboard => InputDevice::Mouse,
                    };
                }
                MenuResult::Selected(6) => {
                    self.clear_area(20, 80, 300, 230);
                    match device {
                        InputDevice::Mouse => self.set_up_mouse(130).await,
                        InputDevice::Keyboard => self.set_up_keyboard(130).await,
                    }
                }
                _ => {}
            }

            // The sensitivities and input device are kept for the player whose options were
            // shown, before switching players.
            let p = &mut self.settings.players[self.player];
            p.horizontal_speed = items[2].value();
            p.vertical_speed = items[3].value();
            p.input_device = new_device;

            if result == MenuResult::Selected(0) {
                if !self.settings.two_players {
                    self.settings.two_players = true;
                } else if self.player == 0 {
                    self.player = 1;
                } else {
                    self.settings.two_players = false;
                    self.player = 0;
                }
            }

            if let MenuResult::Selected(i) = result {
                selected = i;
            } else {
                break;
            }
        }

        self.player = 0;
        self.settings.save();
    }

    async fn sound_menu(&mut self) {
        let mut selected = 0;

        loop {
            let s = &self.settings;
            let mut items = if s.sound_enabled {
                vec![
                    MenuItem::item("Sound: On"),
                    MenuItem::slider("Sound Volume", 0, MAX_VOLUME, s.volume),
                    MenuItem::item(format!("Max. Sounds: {}", s.max_sounds)),
                ]
            } else {
                vec![MenuItem::item("Sound: Off")]
            };

            let result = self.menu(&mut items, selected, 60).await;

            if let Some(volume) = items.get(1) {
                self.settings.volume = volume.value();
            }

            match result {
                MenuResult::Selected(0) => {
                    self.settings.sound_enabled = !self.settings.sound_enabled
                }
                MenuResult::Selected(2) => {
                    self.settings.max_sounds = self.settings.max_sounds % MAX_VOICES + 1
                }
                _ => {}
            }

            self.apply_sound_settings();

            match result {
                MenuResult::Selected(i) => selected = i,
                _ => break,
            }
        }
    }

    async fn set_up_mouse(&mut self, y: i32) {
        for (prompt, is_fire) in [
            ("Press fire button", true),
            ("Press smartbomb button", false),
        ] {
            self.print_over_stars(160, y, TEXT_COLOR, prompt);

            let pressed = loop {
                self.star_field_step().await;
                let pressed = self.buttons_pressed();
                if pressed.any() || self.keyboard.take_press(KeyCode::Escape) {
                    break pressed;
                }
            };
            if !pressed.any() {
                return;
            }

            let button = if pressed.left {
                MouseButton::Left
            } else {
                MouseButton::Right
            };
            let p = &mut self.settings.players[self.player];
            if is_fire {
                p.fire_button = button;
            } else {
                p.smart_bomb_button = button;
            }
        }

        while self.buttons_down().any() {
            self.star_field_step().await;
        }
    }

    async fn set_up_keyboard(&mut self, y: i32) {
        self.print_over_stars(
            160,
            y,
            TEXT_COLOR,
            "Press the key you want for the following:",
        );

        let names: Vec<&str> = self.settings.players[self.player]
            .keys
            .named_mut()
            .map(|(name, _)| name)
            .to_vec();
        for (i, name) in names.into_iter().enumerate() {
            self.print_over_stars(160, y + 20, TEXT_COLOR, &format!("    {name}    "));

            while !self.keyboard.take_any_key() {
                self.star_field_step().await;
            }
            if self.keyboard.last_key_was(KeyCode::Escape) {
                return;
            }

            if let Some(key) = self.keyboard.last_key {
                *self.settings.players[self.player].keys.named_mut()[i].1 = key;
            }
        }

        self.clear_input(true).await;
    }

    // ------------------------------------------------------------------------------------
    // Demos.
    // ------------------------------------------------------------------------------------

    async fn demo_menu(&mut self) {
        let mut items = [MenuItem::item("Play Demo"), MenuItem::item("Record Demo")];

        match self.menu(&mut items, 0, 60).await {
            MenuResult::Selected(0) => {
                self.print_over_stars(110, 220, TEXT_COLOR, "Demo file name: ");
                let Some(name) = self.choose_demo(170, 220).await else {
                    return;
                };

                let demo = if name == BUILTIN_DEMO {
                    Some(crate::core::assets::builtin_demo())
                } else {
                    storage::load(&format!("{name}{DEMO_EXTENSION}"))
                        .and_then(|s| Demo::from_ron(&s))
                };

                match demo {
                    Some(demo) => self.play_demo(demo).await,
                    None => {
                        self.print_over_stars(
                            160,
                            220,
                            TEXT_COLOR,
                            &format!("       Cannot open file '{name}'        "),
                        );
                        self.wait_for_input(true).await;
                    }
                }
            }
            MenuResult::Selected(1) => {
                self.print_over_stars(110, 220, TEXT_COLOR, "Demo file name: ");
                let name = self.read_text(170, 220, 8, 120).await;
                if name.is_empty() {
                    return;
                }

                let file = format!("{name}{DEMO_EXTENSION}");
                if storage::load(&file).is_some() {
                    self.print_over_stars(
                        160,
                        220,
                        TEXT_COLOR,
                        &format!("File '{name}' exists. Overwrite? (y/n) "),
                    );
                    if !self.yes_or_no(false).await {
                        return;
                    }
                }

                self.demo_name = file;
                self.demo = Demo {
                    seed: storage::random_seed(),
                    two_players: self.settings.two_players,
                    players: self.settings.players,
                    frames: Vec::new(),
                };
                self.demo_recording = true;
                self.game().await;
            }
            _ => {}
        }
    }

    /// Lets the player pick a demo with the arrow keys or mouse buttons. Returns `None` if
    /// they press Escape.
    async fn choose_demo(&mut self, x: i32, y: i32) -> Option<String> {
        let mut names = vec![BUILTIN_DEMO.to_string()];
        names.extend(
            storage::list(DEMO_EXTENSION)
                .into_iter()
                .filter_map(|n| n.strip_suffix(DEMO_EXTENSION).map(String::from)),
        );

        let mut i = 0;
        loop {
            self.clear_area(x, y, x + 165, y + 14);
            let name = names[i].clone();
            self.print(x, y, Page::Visible, TEXT_COLOR, &name);

            let pressed = loop {
                self.star_field_step().await;
                let pressed = self.buttons_pressed();
                if self.keyboard.take_any_key() || pressed.any() {
                    break pressed;
                }
            };

            let k = &mut self.keyboard;
            if k.take_any_press(&[
                KeyCode::ArrowLeft,
                KeyCode::ArrowUp,
                KeyCode::Numpad8,
                KeyCode::Numpad4,
            ]) {
                i = (i + names.len() - 1) % names.len();
            }
            if k.take_any_press(&[
                KeyCode::ArrowRight,
                KeyCode::ArrowDown,
                KeyCode::Numpad2,
                KeyCode::Numpad6,
            ]) || pressed.left
            {
                i = (i + 1) % names.len();
            }

            if self
                .keyboard
                .take_any_press(&[KeyCode::Enter, KeyCode::Escape])
                || pressed.right
            {
                break;
            }
        }

        self.vga.fill_rect(x, y, x + 165, y + 14, Page::Visible, 0);
        (!self.keyboard.last_key_was(KeyCode::Escape)).then(|| names[i].clone())
    }
}
