//! Text, windows, menus, title pages and high scores, ported from `xqinit.pas` and the menu
//! procedures of `xquest.pas`.

use crate::core::data;
use crate::core::game::*;
use crate::core::gfx::{Bitmap, planar_pbm};
use crate::core::input::scan;
use crate::core::sound::{COMPILED_MAXIMUM_SOUND_EFFECTS, MAX_SOUND_VOLUME};
use crate::core::storage;
use crate::core::tp::round;

pub const NO_ITEM_SELECTED: i32 = -1;
pub const MENU_TIME_OUT: i32 = -2;
pub const BASE_COLOR: u8 = 186;
pub const HIGH_LIGHT_COLOR: u8 = 188;

#[derive(Clone, Debug)]
pub enum MenuItemType {
    Item,
    SlideBar { min: i32, max: i32, value: i32 },
}

#[derive(Clone, Debug)]
pub struct MenuItem {
    pub name: String,
    pub mtype: MenuItemType,
}

impl MenuItem {
    pub fn item(name: &str) -> Self {
        Self { name: name.to_string(), mtype: MenuItemType::Item }
    }

    pub fn slide_bar(name: &str, min: i32, max: i32, value: i32) -> Self {
        Self { name: name.to_string(), mtype: MenuItemType::SlideBar { min, max, value } }
    }

    fn slider_mx(&self) -> i32 {
        match self.mtype {
            MenuItemType::SlideBar { min, max, value } => round((value - min) as f64 * 128.0 / (max - min) as f64 - 64.0),
            MenuItemType::Item => 0,
        }
    }

    pub fn value(&self) -> i32 {
        match self.mtype {
            MenuItemType::SlideBar { value, .. } => value,
            MenuItemType::Item => 0,
        }
    }

    pub fn set_value(&mut self, v: i32) {
        if let MenuItemType::SlideBar { value, .. } = &mut self.mtype {
            *value = v;
        }
    }
}

/// A high score table entry (`scoretype`).
#[derive(Clone, Debug)]
pub struct Score {
    pub score: i64,
    pub level: i32,
    pub name: String,
}

const SCORE_RECORD_SIZE: usize = 27;

impl Xq {
    // ------------------------------------------------------------------------------------
    // Input.
    // ------------------------------------------------------------------------------------

    pub fn input_movement(
        &mut self,
        delx: &mut i32,
        dely: &mut i32,
        check_input_type: bool,
        allow_mouse: bool,
        _slow_joy: bool,
        allow_keyboard: bool,
    ) {
        let pi = *self.pi();

        if !check_input_type || allow_mouse || pi.input_device == InputDevice::Mouse {
            let (mx, my) = self.mouse.mmotion();
            *delx = (*delx as i64 + (mx * pi.h_input_speed / 64) as i64) as i16 as i32;
            *dely = (*dely as i64 + (my * pi.v_input_speed / 64) as i64) as i16 as i32;
        }

        if check_input_type && pi.input_device == InputDevice::Keyboard && allow_keyboard {
            let k = &self.kb.keys;
            let key = |i: usize| k[pi.key_array[i] as usize];

            if key(LEFT_KEY) || key(DOWN_LEFT_KEY) || key(UP_LEFT_KEY) {
                *delx -= 10 * pi.h_input_speed / 64;
            }
            if key(RIGHT_KEY) || key(DOWN_RIGHT_KEY) || key(UP_RIGHT_KEY) {
                *delx += 10 * pi.h_input_speed / 64;
            }
            if key(UP_KEY) || key(UP_LEFT_KEY) || key(UP_RIGHT_KEY) {
                *dely -= 10 * pi.v_input_speed / 64;
            }
            if key(DOWN_KEY) || key(DOWN_LEFT_KEY) || key(DOWN_RIGHT_KEY) {
                *dely += 10 * pi.v_input_speed / 64;
            }
            if key(BRAKE_KEY) {
                *delx = round(*delx as f64 / 1.2);
                *dely = round(*dely as f64 / 1.2);
            }
        }
    }

    /// `ButtonPressed`: mouse buttons pressed since the last call.
    pub fn button_pressed(&mut self) -> i32 {
        let mut button = 0;
        if self.mouse.mbutpress(0) > 0 {
            button |= 1;
        }
        if self.mouse.mbutpress(1) > 0 {
            button |= 2;
        }
        button
    }

    /// `ButtonDown`: mouse buttons currently held.
    pub fn button_down(&mut self) -> i32 {
        self.mouse.buttons()
    }

    // ------------------------------------------------------------------------------------
    // Timing.
    // ------------------------------------------------------------------------------------

    pub async fn my_delay(&mut self, msec: i64) {
        let time = self.time_count;
        while (self.time_count as f64) < time as f64 + msec as f64 / 1000.0 * TICKS as f64 {
            self.vsync().await;
        }
    }

    /// Returns false if timed out, true if a key or button was pressed.
    pub async fn delay_or_event(&mut self, time_in_ticks: i64) -> bool {
        let temp = self.time_count + time_in_ticks;

        loop {
            if self.kb.keypressed() || self.button_pressed() != 0 || self.time_count > temp {
                break;
            }
            self.vsync().await;
        }

        self.kb.clear_key_presses();
        self.time_count <= temp
    }

    pub async fn star_field_step(&mut self) {
        let Self { stars, vga, rng, .. } = self;
        stars.step(vga, rng);
        self.vsync().await;
    }

    pub async fn clear_input_buffers(&mut self, stars: bool) {
        loop {
            if stars {
                self.star_field_step().await;
            }
            if self.button_pressed() == 0 && self.button_down() == 0 {
                break;
            }
            if !stars {
                self.vsync().await;
            }
        }
        self.kb.clear_key_presses();
    }

    pub async fn wait_for_event(&mut self, stars: bool) {
        self.clear_input_buffers(stars).await;
        loop {
            if stars {
                self.star_field_step().await;
            }
            if self.kb.keypressed() || self.button_pressed() != 0 {
                break;
            }
            if !stars {
                self.vsync().await;
            }
        }
        self.clear_input_buffers(stars).await;
    }

    pub async fn yes_no_stars(&mut self, default: u8) -> u8 {
        let mut ch = 0;
        self.kb.clear_key_presses();

        while ch == 0 {
            self.star_field_step().await;
            if self.kb.numkeypresses(scan::Y) > 0 {
                ch = b'Y';
            }
            if self.kb.numkeypresses(scan::N) > 0 {
                ch = b'N';
            }
            if self.kb.numkeypresses(scan::ESC) > 0 {
                ch = b'N';
            }
            if self.kb.numkeypresses(scan::ENTER) > 0 {
                ch = default;
            }
        }

        self.clear_input_buffers(true).await;
        ch
    }

    // ------------------------------------------------------------------------------------
    // Text and windows.
    // ------------------------------------------------------------------------------------

    fn comix(&self, c: u8) -> Option<&Bitmap> {
        self.g.comix.get(c as usize).and_then(|b| b.as_ref())
    }

    pub fn xprintf(&mut self, x: i32, y: i32, page: i32, color: u8, s: &str) {
        let mut dx = 0;
        for c in s.bytes() {
            if let Some(bm) = self.g.comix.get(c as usize).and_then(|b| b.as_ref()) {
                self.vga.put_font_pbm(x + dx, y, color, page, bm);
                dx += bm.width;
            }
        }
    }

    pub fn length_of_comix_font_string(&self, s: &str) -> i32 {
        s.bytes().map(|c| self.comix(c).map(|b| b.width).unwrap_or(0)).sum()
    }

    pub fn xprintf_center(&mut self, center: i32, y: i32, page: i32, color: u8, s: &str) {
        let x = center - self.length_of_comix_font_string(s) / 2;
        self.xprintf(x, y + 1, page, color, s);
    }

    pub fn xprintf_center_stars(&mut self, center: i32, y: i32, page: i32, color: u8, s: &str) {
        let width = self.length_of_comix_font_string(s);
        let x = center - width / 2;
        self.stars.reset(x - 1, y - 1, x + width + 5, y + 15);
        let vis = self.vga.visible_page_offs;
        self.vga.rect_fill(x - 1, y - 1, x + width + 5, y + 15, vis, 0);
        self.xprintf(x, y, page, color, s);
    }

    fn xcorner1(&mut self, x: i32, y: i32, s: i32, c1: i32, c2: i32, page: i32) {
        for i in 0..s {
            for j in 0..s {
                let c = if i < j { c1 } else if i > j { c2 } else { (c1 + c2) / 2 };
                self.vga.put_pix(x + i, y + j, page, c as u8);
            }
        }
    }

    fn xcorner2(&mut self, x: i32, y: i32, s: i32, c1: i32, c2: i32, page: i32) {
        for i in 0..s {
            for j in 0..s {
                let c = if s - i - 1 < j { c1 } else if s - i - 1 > j { c2 } else { (c1 + c2) / 2 };
                self.vga.put_pix(x + i, y + j, page, c as u8);
            }
        }
    }

    pub fn xwindow(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, bs: i32, page: i32) {
        self.vga.rect_fill(x1 + bs, y1 + bs, x2 - bs, y2 - bs, page, 14);
        self.vga.rect_fill(x1 + bs, y1, x2 - bs, y1 + bs, page, 23);
        self.vga.rect_fill(x1 + bs, y2 - bs, x2 - bs, y2, page, 10);
        self.vga.rect_fill(x1, y1 + bs, x1 + bs, y2 - bs, page, 20);
        self.vga.rect_fill(x2 - bs, y1 + bs, x2, y2 - bs, page, 12);
        self.xcorner1(x1, y1, bs, 20, 23, page);
        self.xcorner1(x2 - bs, y2 - bs, bs, 10, 12, page);
        self.xcorner2(x1, y2 - bs, bs, 10, 20, page);
        self.xcorner2(x2 - bs, y1, bs, 12, 23, page);
        self.text_window_x = x1;
        self.text_window_y = y1;
    }

    fn draw_box(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, page: i32) {
        self.vga.line(x1, y1, x2, y1, page, 10);
        self.vga.line(x1, y2, x2, y2, page, 23);
        self.vga.line(x1, y1 + 1, x1, y2, page, 10);
        self.vga.line(x2, y1 + 1, x2, y2, page, 23);
    }

    pub fn xtext(&mut self, x: i32, y: i32, boxed: bool, page: i32, s: &str) {
        for (i, c) in s.bytes().enumerate() {
            let f = data::fontmap(c);
            if f != 0 {
                self.vga.put_pbm(x + 8 * i as i32, y, page, &self.g.font[f]);
            }
        }
        if boxed {
            self.draw_box(x - 2, y - 1, x + s.len() as i32 * 8 + 2, y + 14, page);
        }
    }

    pub fn xtext_center(&mut self, x: i32, y: i32, boxed: bool, page: i32, s: &str) {
        self.xtext(x - s.len() as i32 * 4, y, boxed, page, s);
    }

    pub fn text_window(&mut self, text: &str) {
        let l = text.len() as i32 * 4;
        self.text_window_size = l;
        let vis = self.vga.visible_page_offs;
        let (vx, vy) = (self.visible_page_x, self.visible_page_y);

        self.win_background = self.vga.get_pbm(160 + vx - l - 10, 95 + vy, (2 * l + 20) / 4 + 1, 30, vis);
        self.xwindow(160 + vx - l - 10, 95 + vy, 160 + vx + l + 10, 125 + vy, 2, vis);
        let (tx, ty) = (self.text_window_x, self.text_window_y);
        self.xtext(tx + 10, ty + 8, false, vis, text);
    }

    pub fn remove_text_window(&mut self) {
        let vis = self.vga.visible_page_offs;
        let bg = std::mem::take(&mut self.win_background);
        self.vga.put_pbm(160 + self.visible_page_x - self.text_window_size - 10, 95 + self.visible_page_y, vis, &bg);
    }

    // ------------------------------------------------------------------------------------
    // Title pages and menus.
    // ------------------------------------------------------------------------------------

    async fn fade_out(&mut self, pal: &[u8], first: u8, count: u8) {
        let mut pal2 = vec![0u8; 770];
        pal2[0] = first;
        pal2[1] = count;

        for a in (0..=64u32).rev() {
            for b in 2..770 {
                pal2[b] = ((pal[b] as u32 * a) >> 6) as u8;
            }
            self.vga.put_pal_struc(&pal2);
            self.vsync().await;
        }
    }

    pub async fn title_page(&mut self) {
        self.vga.set_mode_no_split_screen();
        self.vga.put_pal_raw(&[[0; 3]; 256], 256, 0);

        let p0 = self.vga.page0_offs;
        self.vga.put_pbm(80, 95, p0, &planar_pbm(data::TITLE0));
        self.play_sound(GATESOUND);
        self.my_delay(100).await;

        let palette = data::palette();
        self.vga.put_pal_struc(&palette);
        self.clear_input_buffers(false).await;
        self.delay_or_event(3 * TICKS / 2).await;
        self.fade_out(&palette, palette[0], palette[1]).await;

        let half = data::TITLE.len() / 2;
        self.vga.put_pbm(0, 0, p0, &planar_pbm(&data::TITLE[..half]));
        self.vga.put_pbm(0, 120, p0, &planar_pbm(&data::TITLE[half..]));
        self.play_sound(GATESOUND);
        self.my_delay(100).await;

        let title_palette = data::title_palette();
        self.vga.put_pal_struc(&title_palette);
        self.clear_input_buffers(false).await;
        self.delay_or_event(TICKS * 3).await;
        self.fade_out(&title_palette, 0, 255).await;
    }

    pub async fn menu_screen_setup(&mut self) {
        self.vga.put_pal_raw(&[[0; 3]; 256], 256, 0);
        let p0 = self.vga.page0_offs;
        let lh = self.vga.logical_height;
        self.vga.rect_fill(0, 0, PAGE_WIDTH, lh, p0, 0);

        let vis = self.vga.visible_page_offs;
        self.vga.put_pbm(0, 20, vis, &planar_pbm(data::STARTPIC));

        let Self { stars, rng, .. } = self;
        stars.init(rng);
        self.vga.put_pal_struc(&data::palette());
        self.clear_input_buffers(true).await;
    }

    fn display_menu_item(&mut self, items: &[MenuItem], menu_y: i32, i: usize, highlight: bool) {
        let ypos = menu_y + i as i32 * 20;
        let vis = self.vga.visible_page_offs;
        let color = if highlight { HIGH_LIGHT_COLOR } else { BASE_COLOR };
        let item = &items[i];

        match item.mtype {
            MenuItemType::Item => self.xprintf_center(160, ypos, vis, color, &item.name),
            MenuItemType::SlideBar { min, max, value } => {
                let x = 160 - self.length_of_comix_font_string(&item.name);
                self.xprintf(x, ypos, vis, color, &item.name);
                self.xdraw_slide_bar(170, ypos, vis, 22, 27, min, max, value);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn xdraw_slide_bar(&mut self, x: i32, y: i32, page: i32, color: u8, bcolor: u8, min: i32, max: i32, value: i32) {
        let w = round(100.0 * (value - min) as f64 / (max - min) as f64);
        self.vga.rect_fill(x - 1, y - 1, x + 101, y + 12, page, bcolor);
        self.vga.rect_fill(x + w, y, x + 100, y + 11, page, 0);
        self.vga.rect_fill(x, y, x + w, y + 11, page, color);
    }

    /// `XMenu`. Returns the selected item, [`NO_ITEM_SELECTED`] or [`MENU_TIME_OUT`].
    pub async fn xmenu(&mut self, items: &mut [MenuItem], initial_selection: i32, time_out: i64) -> i32 {
        const COLOR_CYCLE_PAL: [[u8; 3]; 13] = [
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
        const MAX_COLOR_CYCLE: usize = 13;

        self.clear_input_buffers(true).await;
        let time_out = time_out * FRAME_RATE as i64;
        let num = items.len();
        let menu_y = (7 - num as i32) * 10 + 80;

        self.stars.reset(0, 80, 320, 240);
        let vis = self.vga.visible_page_offs;
        self.vga.rect_fill(0, 80, 320, 240, vis, 0);

        let mut selected = if initial_selection > 0 { initial_selection as usize } else { 0 };
        let mut frame_count: i64 = 0;
        let mut color_cycle: usize = 0;
        let (mut mx, mut my) = (0, 0);

        for i in 0..num {
            self.display_menu_item(items, menu_y, i, i == selected);
            if i == selected {
                if let MenuItemType::SlideBar { .. } = items[i].mtype {
                    mx = items[i].slider_mx();
                }
            }
        }

        let mut but;
        let mut enter;

        loop {
            if my < -40 || self.kb.numkeypresses(scan::UARROW) as i32 + self.kb.numkeypresses(scan::KEYPAD8) as i32 > 0 {
                if selected > 0 {
                    color_cycle = MAX_COLOR_CYCLE * 3 / 2;
                    self.display_menu_item(items, menu_y, selected, false);
                    selected -= 1;
                    self.play_sound(MENUCLICK);
                }
                mx = items[selected].slider_mx();
                my = 0;
            }

            if my > 40 || self.kb.numkeypresses(scan::DARROW) as i32 + self.kb.numkeypresses(scan::KEYPAD2) as i32 > 0 {
                if selected < num - 1 {
                    color_cycle = MAX_COLOR_CYCLE * 3 / 2;
                    self.display_menu_item(items, menu_y, selected, false);
                    selected += 1;
                    self.play_sound(MENUCLICK);
                }
                mx = items[selected].slider_mx();
                my = 0;
            }

            self.display_menu_item(items, menu_y, selected, true);

            loop {
                color_cycle += 1;
                if color_cycle >= MAX_COLOR_CYCLE * 3 {
                    color_cycle = 1;
                }
                let c = COLOR_CYCLE_PAL[color_cycle / 3];
                self.vga.set_rgb(HIGH_LIGHT_COLOR + 1, c[0], c[1], c[2]);
                self.vga.set_rgb(HIGH_LIGHT_COLOR + 2, c[0] / 2, c[1] / 2, c[2] / 2);

                self.star_field_step().await;
                frame_count += 1;
                self.input_movement(&mut mx, &mut my, true, true, true, false);
                but = self.button_down();

                if but != 0 || self.kb.keypressed() || my < -40 || my > 40 || frame_count > time_out {
                    break;
                }
            }

            if self.kb.numkeypresses(scan::J) > 0 {
                self.initialise_joystick(220).await;
            }

            if let MenuItemType::SlideBar { min, max, value } = &mut items[selected].mtype {
                if but & (1 | JOY_A_BUTTON1) != 0 {
                    *value = round((mx + 64) as f64 * (*max - *min) as f64 / 128.0 + *min as f64);
                }
                let step = round((*max - *min) as f64 / 50.0) + 1;
                if self.kb.numkeypresses(scan::LARROW) as i32 + self.kb.numkeypresses(scan::KEYPAD4) as i32 > 0 {
                    *value -= step;
                } else if self.kb.numkeypresses(scan::RARROW) as i32 + self.kb.numkeypresses(scan::KEYPAD6) as i32 > 0 {
                    *value += step;
                }
                *value = (*value).clamp(*min, *max);
                mx = mx.clamp(-64, 64);
            }

            but = self.button_pressed();
            enter = self.kb.numkeypresses(scan::ENTER) as i32;

            let is_item = matches!(items[selected].mtype, MenuItemType::Item);
            if but & (2 | JOY_A_BUTTON2) != 0
                || (but & (1 | JOY_A_BUTTON1) != 0 && is_item)
                || self.kb.numkeypresses(scan::ESC) > 0
                || enter > 0
                || frame_count > time_out
            {
                break;
            }
        }

        let result = if but & (1 | JOY_A_BUTTON1) != 0 || enter > 0 {
            selected as i32
        } else if frame_count > time_out {
            MENU_TIME_OUT
        } else {
            NO_ITEM_SELECTED
        };

        self.display_menu_item(items, menu_y, selected, false);
        result
    }

    pub async fn get_string(&mut self, x: i32, y: i32, max_length: usize, max_screen_length: i32, page: i32, color: u8) -> String {
        let mut s = String::new();
        self.xprintf(x, y, page, color, "_");
        self.clear_input_buffers(true).await;

        loop {
            loop {
                self.star_field_step().await;
                if self.kb.keypressed() {
                    break;
                }
            }

            let deletes = self.kb.numkeypresses(scan::DEL) as i32
                + self.kb.numkeypresses(scan::KEYPAD_DEL) as i32
                + self.kb.numkeypresses(scan::BACKSPACE) as i32;
            let c = self.kb.last_key_hit;

            if deletes > 0 {
                s.pop();
            } else if c != 13
                && c != 27
                && s.len() < max_length
                && self.length_of_comix_font_string(&s) < max_screen_length
                && self.comix(c).map(|b| b.width > 0).unwrap_or(false)
            {
                s.push(c as char);
            }

            self.stars.reset(x, y, x + 165, y + 14);
            self.vga.rect_fill(x, y, x + 165, y + 14, page, 0);
            if s.len() < max_length && self.length_of_comix_font_string(&s) < max_screen_length {
                self.xprintf(x, y, page, color, &format!("{s}_"));
            } else {
                self.xprintf(x, y, page, color, &s);
            }

            if self.kb.last_key_hit == 13 {
                break;
            }
        }

        self.vga.rect_fill(x, y, x + 165, y + 14, page, 0);
        s
    }

    /// `GetFileName`: chooses one of the demo files. Returns `None` if there are none and an
    /// empty string if Escape was pressed.
    pub async fn get_file_name(&mut self, x: i32, y: i32, page: i32, color: u8) -> Option<String> {
        let mut names = vec!["XQUEST.DMO".to_string()];
        for n in storage::list(".dmo") {
            if !n.eq_ignore_ascii_case("xquest.dmo") {
                names.push(n.to_uppercase());
            }
        }

        let mut work = 0usize;
        let n = names.len();

        loop {
            self.stars.reset(x, y, x + 165, y + 14);
            self.vga.rect_fill(x, y, x + 165, y + 14, page, 0);
            let name = names[work].clone();
            self.xprintf(x, y, page, color, &name);

            let mut i;
            loop {
                self.star_field_step().await;
                i = self.button_pressed();
                if self.kb.keypressed() || i != 0 {
                    break;
                }
            }

            let prev = self.kb.numkeypresses(scan::LARROW) as i32
                + self.kb.numkeypresses(scan::UARROW) as i32
                + self.kb.numkeypresses(scan::KEYPAD8) as i32
                + self.kb.numkeypresses(scan::KEYPAD4) as i32;
            if prev > 0 {
                work = (work + n - 1) % n;
            }
            let next = self.kb.numkeypresses(scan::RARROW) as i32
                + self.kb.numkeypresses(scan::DARROW) as i32
                + self.kb.numkeypresses(scan::KEYPAD2) as i32
                + self.kb.numkeypresses(scan::KEYPAD6) as i32;
            if next > 0 {
                work = (work + n - 1) % n;
            }
            if i & 1 != 0 {
                work = (work + 1) % n;
            }

            if self.kb.numkeypresses(scan::ENTER) > 0 || self.kb.numkeypresses(scan::ESC) > 0 || i & 2 != 0 {
                break;
            }
        }

        self.vga.rect_fill(x, y, x + 165, y + 14, page, 0);
        if self.kb.last_key_hit == 27 { Some(String::new()) } else { Some(names[work].clone()) }
    }

    // ------------------------------------------------------------------------------------
    // High scores.
    // ------------------------------------------------------------------------------------

    fn load_scores() -> Option<Vec<Vec<Score>>> {
        let d = storage::load("xquest.scr")?;
        if d.len() < 50 * SCORE_RECORD_SIZE {
            return None;
        }

        Some(
            (0..5)
                .map(|i| {
                    (0..10)
                        .map(|j| {
                            let r = &d[(i * 10 + j) * SCORE_RECORD_SIZE..];
                            let score = i32::from_le_bytes([r[0], r[1], r[2], r[3]]) as i64;
                            let level = u16::from_le_bytes([r[4], r[5]]) as i32;
                            let len = (r[6] as usize).min(20);
                            let name = String::from_utf8_lossy(&r[7..7 + len]).to_string();
                            Score { score, level, name }
                        })
                        .collect()
                })
                .collect(),
        )
    }

    fn save_scores(sc: &[Vec<Score>]) {
        let mut d = Vec::new();
        for row in sc {
            for s in row {
                d.extend_from_slice(&(s.score as i32).to_le_bytes());
                d.extend_from_slice(&(s.level as u16).to_le_bytes());
                let name = s.name.as_bytes();
                let len = name.len().min(20);
                d.push(len as u8);
                let mut field = [0u8; 20];
                field[..len].copy_from_slice(&name[..len]);
                d.extend_from_slice(&field);
            }
        }
        storage::save("xquest.scr", &d);
    }

    pub async fn hi_scores(&mut self, seconds: i64) {
        self.vga.set_rgb(HIGH_LIGHT_COLOR + 1, 55, 55, 55);
        self.vga.set_rgb(HIGH_LIGHT_COLOR + 2, 25, 25, 25);

        let mut sc = match Self::load_scores() {
            Some(sc) => sc,
            None => {
                let sc = vec![vec![Score { score: 0, level: 1, name: "XQuest".to_string() }; 10]; 5];
                Self::save_scores(&sc);
                sc
            }
        };

        for pl in 0..2 {
            if pl != 0 && !self.two_player {
                continue;
            }

            let diff = self.player_info[pl].diff_level as usize;
            self.clear_input_buffers(true).await;
            self.stars.reset(10, 80, 310, 239);
            let vis = self.vga.visible_page_offs;
            self.vga.rect_fill(10, 80, 310, 239, vis, 0);

            let mut k = 0;
            for i in 1..=10 {
                let score = self.game_info[pl].score;
                if score > sc[diff][i - 1].score {
                    k = i;
                    for j in ((i + 1)..=10).rev() {
                        sc[diff][j - 1] = sc[diff][j - 2].clone();
                    }
                    sc[diff][i - 1].score = score;
                    sc[diff][i - 1].level = self.game_info[pl].total_level;
                    self.game_info[pl].score = 0;
                } else {
                    let name = sc[diff][i - 1].name.clone();
                    self.xprintf(20, 86 + i as i32 * 14, vis, BASE_COLOR, &name);
                }
                let s = sc[diff][i - 1].level.to_string();
                let w = self.length_of_comix_font_string(&s);
                self.xprintf(300 - w, 86 + i as i32 * 14, vis, BASE_COLOR, &s);
                let s = sc[diff][i - 1].score.to_string();
                let w = self.length_of_comix_font_string(&s);
                self.xprintf(270 - w, 86 + i as i32 * 14, vis, BASE_COLOR, &s);
            }

            if k != 0 {
                let msg = if k == 1 { "A NEW HIGH SCORE!!!" } else { "A TOP TEN SCORE!!!" };
                let msg = if self.two_player { format!("PLAYER {}: {msg}", pl + 1) } else { msg.to_string() };
                self.xprintf_center(160, 80, vis, HIGH_LIGHT_COLOR, &msg);
                self.play_sound(if k == 1 { WOOHOO } else { ALLRIGHT });

                self.play_sound(APPLAUSE);
                for _ in 0..20 {
                    self.star_field_step().await;
                }
                if self.mixer.lock().unwrap().max_sound_effects > 2 {
                    self.play_sound(APPLAUSE);
                }
                for _ in 0..20 {
                    self.star_field_step().await;
                }
                if self.mixer.lock().unwrap().max_sound_effects > 3 {
                    self.play_sound(APPLAUSE);
                }

                let mut name = self.get_string(20, 86 + k as i32 * 14, 20, 160, vis, BASE_COLOR).await;
                if name.is_empty() {
                    name = "Anonymous".to_string();
                }
                self.xprintf(20, 86 + k as i32 * 14, vis, BASE_COLOR, &name);
                sc[diff][k - 1].name = name;
                Self::save_scores(&sc);
            } else {
                self.xprintf_center(160, 80, vis, HIGH_LIGHT_COLOR, &format!("HALL OF FAME ({})", DIFF_NAME2[diff]));
            }

            let mut k = 0;
            loop {
                self.star_field_step().await;
                k += 1;
                if self.button_pressed() != 0 || self.kb.keypressed() || k > seconds * FRAME_RATE as i64 {
                    break;
                }
            }
        }

        self.clear_input_buffers(true).await;
    }

    // ------------------------------------------------------------------------------------
    // Help.
    // ------------------------------------------------------------------------------------

    async fn help_page(&mut self, lines: &[(i32, &str)]) {
        self.stars.reset(0, 90, 310, 230);
        let vis = self.vga.visible_page_offs;
        self.vga.rect_fill(0, 90, 311, 230, vis, 0);
        for &(y, s) in lines {
            self.xprintf_center(160, y, vis, BASE_COLOR, s);
        }
    }

    async fn animate_until_event(&mut self, kind: usize, x: i32, y: i32) {
        let mut frame = 0;
        let vis = self.vga.visible_page_offs;

        loop {
            let k = &self.enemykind[kind];
            frame += k.framespeed;
            if frame >= (k.numframes + 1) << 8 {
                frame = 0;
            }
            let (w, h) = (k.width, k.height);
            let bm = k.frames[(frame >> 8) as usize].bm.clone();

            self.stars.reset(x, y, x + w, y + h);
            self.vga.put_cbitmap(x, y, vis, &bm);
            self.star_field_step().await;
            self.vga.rect_fill(x, y, x + w, y + h, vis, 0);

            if self.button_pressed() != 0 || self.kb.keypressed() {
                break;
            }
        }
    }

    pub async fn show_help(&mut self) {
        let vis = self.vga.visible_page_offs;

        self.help_page(&[
            (95, "Use the mouse or joystick to move"),
            (110, "the ship     around. Collect the"),
            (125, "gems    and avoid the mines    ."),
            (140, "Button 1 fires your bullets, while"),
            (155, "button 2 or the spacebar activates"),
            (170, "a SmartBomb if you have any."),
            (215, "Hit a button or key to continue."),
        ])
        .await;
        self.vga.put_cbitmap(118, 111, vis, &self.g.ship[0].bm);
        self.vga.put_cbitmap(95, 126, vis, &self.g.objects[0].bm);
        self.vga.put_cbitmap(240, 126, vis, &self.g.objects[1].bm);
        self.wait_for_event(true).await;
        if self.kb.last_key_hit == 27 {
            return;
        }

        self.help_page(&[
            (95, "Collect extra SmartBombs     and look"),
            (110, "out for PowerCharges     which will"),
            (125, "give you special powerups! Watch out,"),
            (140, "though, PowerCharges are fragile and"),
            (155, "will shatter if you shoot them."),
            (170, "If the going gets too tough hit 'p'"),
            (185, "to pause the game."),
            (215, "Hit a button or key to continue."),
        ])
        .await;
        self.vga.put_cbitmap(207, 96, vis, &self.g.objects[2].bm);
        self.animate_until_event(0, 187, 110).await;
        if self.kb.last_key_hit == 27 {
            return;
        }

        self.help_page(&[(95, "There are many types of nasties just"), (110, "waiting to do some damage...")]).await;

        let (mut i, mut j, mut frame) = (2usize, 0, 0);
        loop {
            j += 2;
            let k = &self.enemykind[i];
            let (w, h) = (k.width, k.height);
            let bm = k.frames[(frame >> 8) as usize].bm.clone();

            self.stars.reset(j, 140, j + w, 140 + h);
            self.vga.put_cbitmap(j, 140, vis, &bm);
            self.star_field_step().await;

            let k = &self.enemykind[i];
            frame += k.framespeed;
            if frame >= (k.numframes + 1) << 8 {
                frame = 0;
            }
            self.vga.rect_fill(j, 140, j + w, 140 + h, vis, 0);

            if j > 310 {
                j = 0;
                i += 1;
                if i > crate::core::gfx::MAX_ENEMY_KINDS {
                    i = 2;
                }
                if i == 6 {
                    i += 1;
                }
                if i == 14 {
                    i += 1;
                }
                frame = 0;
            }
            if j == 120 {
                self.stars.reset(80, 170, 240, 210);
                self.vga.rect_fill(80, 170, 240, 210, vis, 0);
                self.xprintf_center(160, 170, vis, BASE_COLOR, ENEMY_NAME[i]);
                let s = format!("{} points", self.enemykind[i].score);
                self.xprintf_center(160, 195, vis, BASE_COLOR, &s);
            }
            if j == 200 {
                self.stars.reset(80, 170, 240, 210);
                self.vga.rect_fill(80, 170, 240, 210, vis, 0);
            }

            if self.button_pressed() != 0 || self.kb.keypressed() {
                break;
            }
        }
        if self.kb.last_key_hit == 27 {
            return;
        }

        self.help_page(&[
            (95, "When all the gems have been"),
            (110, "collected the gate will open and you"),
            (125, "can go on to the next level. If you"),
            (140, "complete the level in less than the"),
            (155, "par time you will be awarded a time"),
            (170, "bonus. If your score gets high enough"),
            (185, "you may also get extra lives."),
            (215, "Hit a button or key to continue."),
        ])
        .await;
        self.wait_for_event(true).await;
        if self.kb.last_key_hit == 27 {
            return;
        }

        self.help_page(&[
            (95, "The status bar at the bottom of the"),
            (110, "screen shows your score, the number"),
            (125, "of lives and smartbombs you have left,"),
            (140, "the number of gems left, and what"),
            (155, "powerups you currently possess."),
            (185, "Now go and get those gems!"),
            (215, "Hit a button or key to return to the menu."),
        ])
        .await;
        self.wait_for_event(true).await;
    }

    // ------------------------------------------------------------------------------------
    // Input device set up.
    // ------------------------------------------------------------------------------------

    pub async fn initialise_joystick(&mut self, y: i32) {
        let vis = self.vga.visible_page_offs;
        self.xprintf_center_stars(160, y, vis, BASE_COLOR, "Joystick not found");
        self.player_info[0].input_device = InputDevice::Mouse;
        self.player_info[1].input_device = InputDevice::Mouse;
        self.wait_for_event(true).await;
    }

    pub async fn initialise_mouse(&mut self, y: i32) {
        let vis = self.vga.visible_page_offs;

        self.xprintf_center_stars(160, y, vis, BASE_COLOR, "Press fire button");
        let mut i;
        loop {
            self.star_field_step().await;
            i = self.button_pressed();
            if i & 3 != 0 || self.kb.numkeypresses(scan::ESC) > 0 {
                break;
            }
        }
        if self.kb.last_key_hit == 27 {
            return;
        }
        self.player_info[self.player].mouse_fire_button = i;

        self.xprintf_center_stars(160, y, vis, BASE_COLOR, "Press smartbomb button");
        loop {
            self.star_field_step().await;
            i = self.button_pressed();
            if i & 3 != 0 || self.kb.numkeypresses(scan::ESC) > 0 {
                break;
            }
        }
        if self.kb.last_key_hit == 27 {
            return;
        }
        self.player_info[self.player].mouse_smart_bomb_button = i;

        loop {
            self.star_field_step().await;
            if self.button_down() == 0 {
                break;
            }
        }
    }

    pub async fn initialise_keyboard(&mut self, y: i32) {
        const KEY_NAME_LIST: [&str; 11] =
            ["Up", "Down", "Left", "Right", "Up Left", "Up Right", "Down Left", "Down Right", "Brake", "Fire", "Smart Bomb"];
        let vis = self.vga.visible_page_offs;

        self.xprintf_center_stars(160, y, vis, BASE_COLOR, "Press the key you want for the following:");
        for (k, name) in KEY_NAME_LIST.iter().enumerate() {
            self.xprintf_center_stars(160, y + 20, vis, BASE_COLOR, &format!("    {name}    "));
            loop {
                self.star_field_step().await;
                if self.kb.keypressed() {
                    break;
                }
            }
            if self.kb.last_key_hit == 27 {
                return;
            }
            self.player_info[self.player].key_array[k] = self.kb.last_scan_code;
        }
        self.clear_input_buffers(true).await;
    }

    // ------------------------------------------------------------------------------------
    // Configuration.
    // ------------------------------------------------------------------------------------

    pub fn read_defaults(&mut self) -> bool {
        let Some(d) = storage::load("xquest.cfg") else {
            return false;
        };
        let text = String::from_utf8_lossy(&d);
        let lines: Vec<Vec<i32>> = text
            .lines()
            .map(|l| l.split_whitespace().map_while(|t| t.parse::<i32>().ok()).collect())
            .collect();
        let get = |line: usize, i: usize| lines.get(line).and_then(|l| l.get(i)).copied();

        if let Some(v) = get(0, 0) {
            self.sound_volume = v;
        }
        self.two_player = get(1, 0) == Some(2);

        for pl in 0..2 {
            let base = 2 + pl * 11;
            let p = &mut self.player_info[pl];
            p.h_input_speed = get(base + 2, 0).unwrap_or(64);
            p.v_input_speed = get(base + 3, 0).unwrap_or(64);
            p.diff_level = get(base + 4, 0).unwrap_or(2).clamp(0, MAX_DIFF_LEVEL - 1);
            p.input_device = match get(base + 5, 0).unwrap_or(0) & 0xFF {
                1 => InputDevice::Joy,
                2 => InputDevice::Keyboard,
                _ => InputDevice::Mouse,
            };
            p.mouse_fire_button = get(base + 6, 0).unwrap_or(1);
            p.mouse_smart_bomb_button = get(base + 7, 0).unwrap_or(2);
            p.joy_fire_button = get(base + 8, 0).unwrap_or(JOY_A_BUTTON1);
            p.joy_smart_bomb_button = get(base + 9, 0).unwrap_or(JOY_A_BUTTON2);
            for k in 0..11 {
                if let Some(v) = get(base + 10, k) {
                    p.key_array[k] = v as u8;
                }
            }
        }

        if let Some(v) = get(27, 0) {
            self.sound_card = v;
        }
        if let Some(v) = get(31, 0) {
            self.mixer.lock().unwrap().max_sound_effects = v.clamp(0, COMPILED_MAXIMUM_SOUND_EFFECTS);
        }

        true
    }

    pub fn write_defaults(&self) {
        let mut f = String::new();
        f += &format!("{:4} Sound Volume\n", self.sound_volume);
        f += if self.two_player { "   2 Number of Players\n" } else { "   1 Number of Players\n" };

        for pl in 0..2 {
            let p = &self.player_info[pl];
            f += "\n";
            f += if pl == 0 { "Player One\n" } else { "Player Two\n" };
            f += &format!("{:4} Horizontal Input Sensitivity\n", p.h_input_speed);
            f += &format!("{:4} Vertical Input Sensitivity\n", p.v_input_speed);
            f += &format!("{:4} Difficulty Level\n", p.diff_level);
            f += &format!("{:4} InputDevice\n", p.input_device as i32);
            f += &format!("{:4} Mouse Fire Button\n", p.mouse_fire_button);
            f += &format!("{:4} Mouse Smartbomb Button\n", p.mouse_smart_bomb_button);
            f += &format!("{:4} Joystick Fire Button\n", p.joy_fire_button);
            f += &format!("{:4} Joystick Smartbomb Button\n", p.joy_smart_bomb_button);
            for k in p.key_array {
                f += &format!("{k} ");
            }
            f += " Keys\n";
        }

        f += "\n";
        f += "0 0 0 0 0 0 0 0 Joystick calibration values\n";
        f += "   0 Joystick calibrated?\n";
        f += &format!("{:4} Sound card\n", self.sound_card);
        f += " 544 Port\n";
        f += "   5 IRQ\n";
        f += "   1 DMA\n";
        f += &format!("{:4} Maximum simultaneous sounds\n", self.mixer.lock().unwrap().max_sound_effects);

        storage::save("xquest.cfg", f.as_bytes());
    }

    // ------------------------------------------------------------------------------------
    // Demos.
    // ------------------------------------------------------------------------------------

    pub fn parse_demo(d: &[u8]) -> Option<Demo> {
        if d.len() < 79 {
            return None;
        }

        let i16_at = |o: usize| i16::from_le_bytes([d[o], d[o + 1]]) as i32;
        let mut demo = Demo {
            rand_seed: u32::from_le_bytes([d[0], d[1], d[2], d[3]]),
            two_player: d[4] == 1,
            ..Default::default()
        };

        for pl in 0..2 {
            let o = 5 + pl * 37;
            let p = &mut demo.player_info[pl];
            p.input_device = match d[o] {
                1 => InputDevice::Joy,
                2 => InputDevice::Keyboard,
                _ => InputDevice::Mouse,
            };
            p.h_input_speed = i16_at(o + 1);
            p.v_input_speed = i16_at(o + 3);
            p.joy_fire_button = i16_at(o + 5);
            p.joy_smart_bomb_button = i16_at(o + 7);
            p.mouse_fire_button = i16_at(o + 9);
            p.mouse_smart_bomb_button = i16_at(o + 11);
            p.diff_level = i16_at(o + 13).clamp(0, MAX_DIFF_LEVEL - 1);
            for k in 0..11 {
                p.key_array[k] = i16_at(o + 15 + k * 2) as u8;
            }
        }

        let mut o = 79;
        while o + 5 <= d.len() {
            demo.frames.push(DemoRec { delx: i16_at(o), dely: i16_at(o + 2), but: d[o + 4] });
            o += 5;
        }

        Some(demo)
    }

    pub fn serialise_demo(demo: &Demo) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&demo.rand_seed.to_le_bytes());
        d.push(demo.two_player as u8);

        for p in &demo.player_info {
            d.push(p.input_device as u8);
            for v in [
                p.h_input_speed,
                p.v_input_speed,
                p.joy_fire_button,
                p.joy_smart_bomb_button,
                p.mouse_fire_button,
                p.mouse_smart_bomb_button,
                p.diff_level,
            ] {
                d.extend_from_slice(&(v as i16).to_le_bytes());
            }
            for k in p.key_array {
                d.extend_from_slice(&(k as i16).to_le_bytes());
            }
        }

        for f in &demo.frames {
            d.extend_from_slice(&(f.delx as i16).to_le_bytes());
            d.extend_from_slice(&(f.dely as i16).to_le_bytes());
            d.push(f.but);
        }

        d
    }

    pub fn set_demo_file(&mut self, name: &str) -> bool {
        let data = if name.eq_ignore_ascii_case("xquest.dmo") {
            storage::load(name).unwrap_or_else(|| data::DEMO.to_vec())
        } else {
            match storage::load(name).or_else(|| storage::load(&name.to_lowercase())) {
                Some(d) => d,
                None => return false,
            }
        };

        let Some(demo) = Self::parse_demo(&data) else {
            return false;
        };

        self.saved_rand_seed = demo.rand_seed;
        self.saved_two_player = self.two_player;
        self.two_player = demo.two_player;
        self.demo = demo;
        true
    }

    pub async fn play_demo(&mut self) {
        self.saved_player_info = self.player_info;
        self.player_info = self.demo.player_info;
        self.demo_mode = true;
        self.game().await;
        self.player_info = self.saved_player_info;
        self.two_player = self.saved_two_player;
    }

    async fn demo_menu(&mut self) {
        let mut menu = vec![MenuItem::item("Play Demo"), MenuItem::item("Record Demo")];
        let j = self.xmenu(&mut menu, 0, 60).await;
        let vis = self.vga.visible_page_offs;

        match j {
            0 => {
                self.xprintf_center_stars(110, 220, vis, BASE_COLOR, "Demo file name: ");
                match self.get_file_name(170, 220, vis, BASE_COLOR).await {
                    Some(name) if name.is_empty() => {}
                    Some(name) if self.set_demo_file(&name) => self.play_demo().await,
                    other => {
                        if other.is_none() {
                            self.xprintf_center_stars(160, 220, vis, BASE_COLOR, "        No demo files found.        ");
                        } else {
                            let msg = format!("       Cannot open file '{}'        ", other.unwrap());
                            self.xprintf_center_stars(160, 220, vis, BASE_COLOR, &msg);
                        }
                        self.set_demo_file("xquest.dmo");
                        self.wait_for_event(true).await;
                    }
                }
            }
            1 => {
                self.xprintf_center_stars(110, 220, vis, BASE_COLOR, "Demo file name: ");
                let name = self.get_string(170, 220, 8, 120, vis, BASE_COLOR).await;
                let stem = name.split('.').next().unwrap_or("").to_string();
                let mut file = format!("{stem}.dmo");

                if storage::load(&file).is_some() {
                    let msg = format!("File '{file}' exists. Overwrite? (y/n) ");
                    self.xprintf_center_stars(160, 220, vis, BASE_COLOR, &msg);
                    if self.yes_no_stars(b'N').await == b'N' {
                        file = ".dmo".to_string();
                    }
                }

                if file != ".dmo" {
                    self.demo_name = file;
                    self.saved_rand_seed = storage::random_seed();
                    self.demo = Demo {
                        rand_seed: self.saved_rand_seed,
                        two_player: self.two_player,
                        player_info: self.player_info,
                        frames: Vec::new(),
                    };
                    self.recording = true;
                    self.game().await;
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------------------------------
    // Option menus.
    // ------------------------------------------------------------------------------------

    async fn sound_options_menu(&mut self) {
        let mut j = 0;
        let mut tmp_sound_card = self.sound_card;
        let (mut addr, mut irq, mut dma) = (0x220, 5, 1);

        loop {
            let max_effects = self.mixer.lock().unwrap().max_sound_effects;
            let mut menu = if tmp_sound_card == SOUND_BLASTER {
                vec![
                    MenuItem::item("Sound Blaster"),
                    MenuItem::slide_bar("Sound Volume", 0, 128, self.sound_volume),
                    MenuItem::item(&format!("Port: {addr:X}")),
                    MenuItem::item(&format!("IRQ: {irq}")),
                    MenuItem::item(&format!("DMA: {dma}")),
                    MenuItem::item(&format!("Max. Sounds: {max_effects}")),
                ]
            } else {
                vec![MenuItem::item("No Soundcard")]
            };

            j = self.xmenu(&mut menu, j, 60).await;

            match j {
                0 => {
                    tmp_sound_card = if tmp_sound_card == SOUND_BLASTER { NO_SOUND_CARD } else { SOUND_BLASTER };
                }
                2 => {
                    addr += 16;
                    if addr > 0x280 {
                        addr = 0x210;
                    }
                }
                3 => {
                    irq += 1;
                    if irq > 15 {
                        irq = 2;
                    }
                }
                4 => {
                    dma += 1;
                    if dma > 3 {
                        dma = 0;
                    }
                }
                5 => {
                    let mut m = self.mixer.lock().unwrap();
                    m.max_sound_effects += 1;
                    if m.max_sound_effects > COMPILED_MAXIMUM_SOUND_EFFECTS {
                        m.max_sound_effects = 1;
                    }
                }
                _ => {}
            }

            if menu.len() > 1 {
                self.sound_volume = menu[1].value().clamp(0, MAX_SOUND_VOLUME);
            }
            if j == NO_ITEM_SELECTED {
                break;
            }
        }

        self.mixer.lock().unwrap().set_volume(self.sound_volume);
        self.sound_card = tmp_sound_card;
        if self.sound_card != SOUND_BLASTER {
            self.mixer.lock().unwrap().stop_all();
        }
    }

    async fn options_menu(&mut self) {
        const CONTROL_OPTION: [&str; 3] = ["Input Device: Mouse", "Input Device: Joystick", "Input Device: Keyboard"];
        let mut j = 0;
        let mut menu = vec![
            MenuItem::item("One Player"),
            MenuItem::item("Sound Setup"),
            MenuItem::slide_bar("Horizontal Sensitivity", 4, 256, 64),
            MenuItem::slide_bar(" Vertical Sensitivity ", 4, 256, 64),
            MenuItem::item("Difficulty:"),
            MenuItem::item("Input:"),
            MenuItem::item("Input Device Setup"),
        ];

        loop {
            let pi = self.player_info[self.player];
            menu[2].set_value(pi.h_input_speed);
            menu[3].set_value(pi.v_input_speed);
            menu[4].name = format!("Difficulty: {}", DIFF_NAME[pi.diff_level as usize]);
            menu[0].name = if !self.two_player {
                "One Player".to_string()
            } else if self.player == 0 {
                "Two Players: Player One".to_string()
            } else {
                "Two Players: Player Two".to_string()
            };
            let mut control = pi.input_device;
            menu[5].name = CONTROL_OPTION[control as usize].to_string();

            j = self.xmenu(&mut menu, j, 60).await;

            match j {
                1 => self.sound_options_menu().await,
                4 => {
                    let p = &mut self.player_info[self.player];
                    p.diff_level = (p.diff_level + 1) % MAX_DIFF_LEVEL;
                }
                5 => {
                    control = match control {
                        InputDevice::Keyboard => InputDevice::Mouse,
                        InputDevice::Mouse => InputDevice::Joy,
                        InputDevice::Joy => InputDevice::Keyboard,
                    };
                    // There is no joystick.
                    if control == InputDevice::Joy {
                        control = InputDevice::Keyboard;
                    }
                }
                6 => {
                    self.stars.reset(20, 80, 320 - 20, 230);
                    let vis = self.vga.visible_page_offs;
                    self.vga.rect_fill(20, 80, 320 - 20, 230, vis, 0);
                    match control {
                        InputDevice::Mouse => self.initialise_mouse(130).await,
                        InputDevice::Joy => self.initialise_joystick(130).await,
                        InputDevice::Keyboard => self.initialise_keyboard(130).await,
                    }
                }
                0 => {
                    let p = &mut self.player_info[self.player];
                    p.h_input_speed = menu[2].value();
                    p.v_input_speed = menu[3].value();
                    p.input_device = control;
                    if !self.two_player {
                        self.two_player = true;
                    } else if self.player == 0 {
                        self.player = 1;
                    } else {
                        self.two_player = false;
                        self.player = 0;
                    }
                }
                _ => {}
            }

            if j != 0 {
                let p = &mut self.player_info[self.player];
                p.h_input_speed = menu[2].value();
                p.v_input_speed = menu[3].value();
                p.input_device = control;
            }

            if j == NO_ITEM_SELECTED {
                break;
            }
        }

        self.player = 0;
        self.write_defaults();
    }

    // ------------------------------------------------------------------------------------
    // Game over and the main menu.
    // ------------------------------------------------------------------------------------

    pub async fn end_screen(&mut self) {
        let vis = self.vga.visible_page_offs;
        let (vx, vy) = (self.visible_page_x, self.visible_page_y);
        self.xwindow(110 + vx, 94 + vy, 210 + vx, 126 + vy, 2, vis);
        let (tx, ty) = (self.text_window_x, self.text_window_y);
        self.xtext(tx + 15, ty + 9, false, vis, "GAME OVER");
        self.delay_or_event(2 * TICKS).await;

        if self.demo_mode {
            self.game_info[0].score = -10;
            self.game_info[1].score = -10;
            self.demo_mode = false;
        }

        if self.recording {
            storage::save(&self.demo_name, &Self::serialise_demo(&self.demo));
            self.recording = false;
            self.xwindow(90 + vx, 90 + vy, 230 + vx, 130 + vy, 2, vis);
            let (tx, ty) = (self.text_window_x, self.text_window_y);
            self.xtext(tx + 15, ty + 13, false, vis, "DEMO RECORDED");
            self.delay_or_event(2 * TICKS).await;
        }

        self.mouse.reset();
        let (p0, p1) = (self.vga.page0_offs, self.vga.page1_offs);
        let lh = self.vga.logical_height;
        self.vga.rect_fill(0, 0, PAGE_WIDTH, lh, p0, 0);
        self.vga.rect_fill(0, 0, PAGE_WIDTH, lh, p1, 0);
        self.vga.set_mode_no_split_screen();
        self.visible_page_x = 0;
        self.visible_page_y = 0;
        self.menu_screen_setup().await;
        self.hi_scores(10).await;
    }

    async fn run_game(&mut self) {
        let mut menu = vec![
            MenuItem::item("Start Game"),
            MenuItem::item("Help"),
            MenuItem::item("Hall of Fame"),
            MenuItem::item("Options"),
            MenuItem::item("Demo"),
            MenuItem::item("Quit"),
        ];

        self.menu_screen_setup().await;
        let mut i = 0;

        loop {
            i = self.xmenu(&mut menu, i, 30).await;
            let mut ch = 0;

            match i {
                MENU_TIME_OUT => {
                    if self.set_demo_file("xquest.dmo") {
                        self.play_demo().await;
                    }
                    i = 0;
                }
                0 => self.game().await,
                1 => self.show_help().await,
                2 => self.hi_scores(120).await,
                3 => self.options_menu().await,
                4 => self.demo_menu().await,
                _ => {}
            }

            if i == 5 || i == NO_ITEM_SELECTED {
                self.stars.reset(10, 90, 230, 310);
                let vis = self.vga.visible_page_offs;
                self.vga.rect_fill(10, 90, 230, 310, vis, 0);
                self.xprintf_center_stars(160, 120, vis, BASE_COLOR, "Are you sure you");
                self.xprintf_center_stars(160, 140, vis, BASE_COLOR, "want to quit?");
                ch = self.yes_no_stars(b'N').await;
            }

            if (i == 5 || i == NO_ITEM_SELECTED) && ch == b'Y' {
                break;
            }
        }
    }

    /// The main program.
    pub async fn run(mut self) {
        let have_defaults = self.read_defaults();
        if !have_defaults {
            self.sound_card = SOUND_BLASTER;
        }
        let volume = self.sound_volume;
        self.mixer.lock().unwrap().set_volume(volume);

        self.initialise_variables();
        self.vga.set_mode_split_double_buffer(PAGE_HEIGHT);
        self.vga.put_pal_struc(&data::palette());
        self.title_page().await;

        self.run_game().await;
        self.write_defaults();

        self.shared.lock().unwrap().finished = true;
    }
}
