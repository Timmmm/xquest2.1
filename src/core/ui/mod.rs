//! Text, windows, waiting for input, and the menus and other screens outside the game.

mod menu;
mod screens;

pub use menu::{MenuItem, MenuResult};

use crate::core::game::XQuest;
use crate::core::input::KeyCode;
use crate::core::vga::Page;

/// Colour added to menu text.
pub const TEXT_COLOR: u8 = 186;
/// Colour added to highlighted menu text; the next two colours cycle.
pub const HIGHLIGHT_COLOR: u8 = 188;

impl XQuest {
    // ------------------------------------------------------------------------------------
    // Waiting.
    // ------------------------------------------------------------------------------------

    pub async fn wait_frames(&mut self, count: i64) {
        for _ in 0..count {
            self.vsync().await;
        }
    }

    /// Waits up to `count` frames for a key or mouse button. Returns whether one was pressed.
    pub async fn wait_or_input(&mut self, count: i64) -> bool {
        let end = self.frame_counter + count;

        while !(self.keyboard.take_any_key()
            || self.buttons_pressed().any()
            || self.frame_counter > end)
        {
            self.vsync().await;
        }

        self.keyboard.clear_presses();
        self.frame_counter <= end
    }

    /// Animates the starfield for one frame.
    pub async fn star_field_step(&mut self) {
        self.stars.step(&mut self.vga, &mut self.rng);
        self.vsync().await;
    }

    /// One frame of waiting, with or without the starfield.
    async fn idle(&mut self, stars: bool) {
        if stars {
            self.star_field_step().await;
        } else {
            self.vsync().await;
        }
    }

    /// Waits for the mouse buttons to be released and forgets any key presses.
    pub async fn clear_input(&mut self, stars: bool) {
        if stars {
            self.star_field_step().await;
        }
        while self.buttons_pressed().any() || self.buttons_down().any() {
            self.idle(stars).await;
        }
        self.keyboard.clear_presses();
    }

    /// Waits for a key or mouse button press.
    pub async fn wait_for_input(&mut self, stars: bool) {
        self.clear_input(stars).await;
        if stars {
            self.star_field_step().await;
        }
        while !self.keyboard.take_any_key() && !self.buttons_pressed().any() {
            self.idle(stars).await;
        }
        self.clear_input(stars).await;
    }

    /// Waits for Y or N (Escape means no, Enter means `default`).
    pub async fn yes_or_no(&mut self, default: bool) -> bool {
        self.keyboard.clear_presses();

        let answer = loop {
            self.star_field_step().await;
            let k = &mut self.keyboard;
            if k.take_press(KeyCode::KeyY) {
                break true;
            }
            if k.take_any_press(&[KeyCode::KeyN, KeyCode::Escape]) {
                break false;
            }
            if k.take_press(KeyCode::Enter) {
                break default;
            }
        };

        self.clear_input(true).await;
        answer
    }

    // ------------------------------------------------------------------------------------
    // Text and windows.
    // ------------------------------------------------------------------------------------

    /// Draws text in the menu font, `color` being added to the font's pixels.
    pub fn print(&mut self, x: i32, y: i32, page: Page, color: u8, text: &str) {
        let mut dx = 0;
        for c in text.chars() {
            if let Some(glyph) = self.gfx.comix.glyph(c) {
                self.vga.draw_tinted(x + dx, y, color, page, glyph);
                dx += glyph.width;
            }
        }
    }

    pub fn text_width(&self, text: &str) -> i32 {
        self.gfx.comix.text_width(text)
    }

    pub fn print_centred(&mut self, centre: i32, y: i32, page: Page, color: u8, text: &str) {
        let x = centre - self.text_width(text) / 2;
        self.print(x, y + 1, page, color, text);
    }

    /// Clears a space in the starfield and prints centred text in it.
    pub fn print_over_stars(&mut self, centre: i32, y: i32, color: u8, text: &str) {
        let width = self.text_width(text);
        let x = centre - width / 2;
        self.clear_area(x - 1, y - 1, x + width + 5, y + 15);
        self.print(x, y, Page::Visible, color, text);
    }

    /// Clears part of the menu screen and the stars in it.
    pub fn clear_area(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        self.stars.forget(x1, y1, x2, y2);
        self.vga.fill_rect(x1, y1, x2, y2, Page::Visible, 0);
    }

    /// A triangle-split square for the corners of window borders.
    fn draw_corner(
        &mut self,
        x: i32,
        y: i32,
        size: i32,
        colors: (u8, u8),
        flipped: bool,
        page: Page,
    ) {
        for i in 0..size {
            for j in 0..size {
                let i2 = if flipped { size - i - 1 } else { i };
                let c = match i2.cmp(&j) {
                    std::cmp::Ordering::Less => colors.0,
                    std::cmp::Ordering::Greater => colors.1,
                    std::cmp::Ordering::Equal => ((colors.0 as i32 + colors.1 as i32) / 2) as u8,
                };
                self.vga.set_pixel(x + i, y + j, page, c);
            }
        }
    }

    /// A raised grey window from `(x1, y1)` to `(x2, y2)`.
    pub fn draw_window(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, page: Page) {
        const B: i32 = 2;

        self.vga.fill_rect(x1 + B, y1 + B, x2 - B, y2 - B, page, 14);
        self.vga.fill_rect(x1 + B, y1, x2 - B, y1 + B, page, 23);
        self.vga.fill_rect(x1 + B, y2 - B, x2 - B, y2, page, 10);
        self.vga.fill_rect(x1, y1 + B, x1 + B, y2 - B, page, 20);
        self.vga.fill_rect(x2 - B, y1 + B, x2, y2 - B, page, 12);
        self.draw_corner(x1, y1, B, (20, 23), false, page);
        self.draw_corner(x2 - B, y2 - B, B, (10, 12), false, page);
        self.draw_corner(x1, y2 - B, B, (10, 20), true, page);
        self.draw_corner(x2 - B, y1, B, (12, 23), true, page);
        (self.window_x, self.window_y) = (x1, y1);
    }

    fn draw_sunken_box(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, page: Page) {
        self.vga.line(x1, y1, x2, y1, page, 10);
        self.vga.line(x1, y2, x2, y2, page, 23);
        self.vga.line(x1, y1 + 1, x1, y2, page, 10);
        self.vga.line(x2, y1 + 1, x2, y2, page, 23);
    }

    /// Draws text in the fixed width status bar font, optionally in a sunken box.
    pub fn draw_text(&mut self, x: i32, y: i32, boxed: bool, page: Page, text: &str) {
        for (i, c) in text.chars().enumerate() {
            if let Some(glyph) = self.gfx.text_font.get(&c) {
                self.vga.draw_opaque(x + 8 * i as i32, y, page, glyph);
            }
        }

        if boxed {
            let len = text.chars().count() as i32;
            self.draw_sunken_box(x - 2, y - 1, x + len * 8 + 2, y + 14, page);
        }
    }

    pub fn draw_text_centred(&mut self, x: i32, y: i32, page: Page, text: &str) {
        let len = text.chars().count() as i32;
        self.draw_text(x - len * 4, y, false, page, text);
    }

    /// Shows a message in a window in the middle of the playing area.
    pub fn show_message(&mut self, text: &str) {
        let half = text.chars().count() as i32 * 4;
        let (x, y) = (160 + self.scroll_x - half - 10, 95 + self.scroll_y);

        let background = self.vga.capture(x, y, 2 * half + 24, 30, Page::Visible);
        self.message_background = Some((x, y, background));

        self.draw_window(x, y, 160 + self.scroll_x + half + 10, y + 30, Page::Visible);
        let (wx, wy) = (self.window_x, self.window_y);
        self.draw_text(wx + 10, wy + 8, false, Page::Visible, text);
    }

    pub fn remove_message(&mut self) {
        if let Some((x, y, background)) = self.message_background.take() {
            self.vga.draw_opaque(x, y, Page::Visible, &background);
        }
    }
}
