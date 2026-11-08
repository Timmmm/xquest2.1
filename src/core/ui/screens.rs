//! The title pages, help, high scores and game over screens.

use super::*;
use crate::core::enemies::EnemyKind;
use crate::core::game::frames;
use crate::core::settings::{HIGH_SCORES_PER_TABLE, HighScores, Score};
use crate::core::sound::Sound;
use crate::core::vga::Palette;

impl XQuest {
    /// Fades the palette to black over 65 frames.
    async fn fade_out(&mut self, palette: Palette) {
        for level in (0..=64u32).rev() {
            self.vga.palette = palette.map(|c| c.map(|v| ((v as u32 * level) >> 6) as u8));
            self.vsync().await;
        }
    }

    pub async fn title_pages(&mut self) {
        self.vga.set_single_page_mode();

        let pictures = [
            (
                self.gfx.title_logo.clone(),
                (80, 95),
                self.gfx.game_palette,
                frames(1.5),
            ),
            (
                self.gfx.title.clone(),
                (0, 0),
                self.gfx.title_palette,
                frames(3.0),
            ),
        ];

        for (picture, (x, y), palette, duration) in pictures {
            self.vga.palette = [[0; 3]; 256];
            self.vga.draw_opaque(x, y, Page::Page0, &picture);
            self.play(Sound::Gate);
            self.wait_frames(frames(0.1)).await;

            self.vga.palette = palette;
            self.clear_input(false).await;
            self.wait_or_input(duration).await;
            self.fade_out(palette).await;
        }
    }

    /// The menu backdrop: the logo over a moving starfield.
    pub async fn menu_screen(&mut self) {
        self.vga.palette = [[0; 3]; 256];
        self.vga.clear_page(Page::Page0);
        let logo = self.gfx.menu_logo.clone();
        self.vga.draw_opaque(0, 20, Page::Visible, &logo);

        self.stars.start(&mut self.rng);
        self.vga.palette = self.gfx.game_palette;
        self.clear_input(true).await;
    }

    // ------------------------------------------------------------------------------------
    // Text entry.
    // ------------------------------------------------------------------------------------

    /// Reads a line of text typed by the player, up to `max_length` characters and
    /// `max_width` pixels.
    pub async fn read_text(&mut self, x: i32, y: i32, max_length: usize, max_width: i32) -> String {
        let mut text = String::new();
        let page = Page::Visible;
        self.print(x, y, page, TEXT_COLOR, "_");
        self.clear_input(true).await;

        loop {
            while !self.keyboard.take_any_key() {
                self.star_field_step().await;
            }

            let has_room = |s: &XQuest, text: &str| {
                text.chars().count() < max_length && s.text_width(text) < max_width
            };

            if self.keyboard.take_any_press(&[
                KeyCode::Backspace,
                KeyCode::Delete,
                KeyCode::NumpadDecimal,
            ]) {
                text.pop();
            } else if let Some(c) = self.keyboard.last_char
                && has_room(self, &text)
                && self.gfx.comix.glyph(c).is_some_and(|g| g.width > 0)
            {
                text.push(c);
            }

            self.clear_area(x, y, x + 165, y + 14);
            let shown = if has_room(self, &text) {
                format!("{text}_")
            } else {
                text.clone()
            };
            self.print(x, y, page, TEXT_COLOR, &shown);

            if self.keyboard.last_key_was(KeyCode::Enter) {
                break;
            }
        }

        self.vga.fill_rect(x, y, x + 165, y + 14, page, 0);
        text
    }

    // ------------------------------------------------------------------------------------
    // High scores.
    // ------------------------------------------------------------------------------------

    /// Shows the high scores, adding the scores of the game just played (if `scores` is
    /// given), for up to `seconds`.
    pub async fn high_scores(&mut self, scores: Option<[i64; 2]>, seconds: i64) {
        self.vga.set_color(HIGHLIGHT_COLOR + 1, [55, 55, 55]);
        self.vga.set_color(HIGHLIGHT_COLOR + 2, [25, 25, 25]);

        let mut tables = HighScores::load();
        let players = if self.settings.two_players { 2 } else { 1 };

        for player in 0..players {
            let difficulty = self.settings.players[player].difficulty;
            self.clear_input(true).await;
            self.clear_area(10, 80, 310, 239);

            let score = scores.map_or(0, |s| s[player]);
            let level = self.progress[player].total_level;
            let table = tables.table_mut(difficulty);
            let place = table.iter().position(|s| score > s.score);
            if let Some(i) = place {
                table.insert(
                    i,
                    Score {
                        score,
                        level,
                        name: String::new(),
                    },
                );
                table.truncate(HIGH_SCORES_PER_TABLE);
            }

            let table = table.clone();
            for (i, entry) in table.iter().enumerate() {
                let y = 100 + i as i32 * 14;
                if Some(i) != place {
                    self.print(20, y, Page::Visible, TEXT_COLOR, &entry.name);
                }
                let level = entry.level.to_string();
                self.print(
                    300 - self.text_width(&level),
                    y,
                    Page::Visible,
                    TEXT_COLOR,
                    &level,
                );
                let points = entry.score.to_string();
                self.print(
                    270 - self.text_width(&points),
                    y,
                    Page::Visible,
                    TEXT_COLOR,
                    &points,
                );
            }

            match place {
                Some(i) => {
                    let message = if i == 0 {
                        "A NEW HIGH SCORE!!!"
                    } else {
                        "A TOP TEN SCORE!!!"
                    };
                    let message = if players == 2 {
                        format!("PLAYER {}: {message}", player + 1)
                    } else {
                        message.to_string()
                    };
                    self.print_centred(160, 80, Page::Visible, HIGHLIGHT_COLOR, &message);
                    self.play(if i == 0 {
                        Sound::WooHoo
                    } else {
                        Sound::AllRight
                    });
                    self.cheer().await;

                    let y = 100 + i as i32 * 14;
                    let mut name = self.read_text(20, y, 20, 160).await;
                    if name.is_empty() {
                        name = "Anonymous".to_string();
                    }
                    self.print(20, y, Page::Visible, TEXT_COLOR, &name);
                    tables.table_mut(difficulty)[i].name = name;
                    tables.save();
                }
                None => {
                    let title = format!("HALL OF FAME ({})", difficulty.name().to_uppercase());
                    self.print_centred(160, 80, Page::Visible, HIGHLIGHT_COLOR, &title);
                }
            }

            let limit = seconds * crate::core::game::NOMINAL_FRAME_RATE as i64;
            for _ in 0..=limit {
                self.star_field_step().await;
                if self.buttons_pressed().any() || self.keyboard.take_any_key() {
                    break;
                }
            }
        }

        self.clear_input(true).await;
    }

    /// Applause, layered if enough sounds can play at once.
    async fn cheer(&mut self) {
        let voices = self.settings.max_sounds;
        self.play(Sound::Applause);
        for extra in [2, 3] {
            for _ in 0..20 {
                self.star_field_step().await;
            }
            if voices > extra {
                self.play(Sound::Applause);
            }
        }
    }

    pub async fn game_over_screen(&mut self) {
        let (sx, sy) = (self.scroll_x, self.scroll_y);
        self.draw_window(110 + sx, 94 + sy, 210 + sx, 126 + sy, Page::Visible);
        let (wx, wy) = (self.window_x, self.window_y);
        self.draw_text(wx + 15, wy + 9, false, Page::Visible, "GAME OVER");
        self.wait_or_input(frames(2.0)).await;

        // A demo does not get a high score, and nor does a game that was quit.
        let scores = (!self.demo_playing).then(|| self.progress.map(|p| p.score));
        self.demo_playing = false;

        if self.demo_recording {
            crate::core::storage::save(&self.demo_name, &self.demo.to_ron());
            self.demo_recording = false;
            self.draw_window(90 + sx, 90 + sy, 230 + sx, 130 + sy, Page::Visible);
            let (wx, wy) = (self.window_x, self.window_y);
            self.draw_text(wx + 15, wy + 13, false, Page::Visible, "DEMO RECORDED");
            self.wait_or_input(frames(2.0)).await;
        }

        self.mouse.reset();
        self.vga.set_single_page_mode();
        (self.scroll_x, self.scroll_y) = (0, 0);
        self.menu_screen().await;
        self.high_scores(scores, 10).await;
    }

    // ------------------------------------------------------------------------------------
    // Help.
    // ------------------------------------------------------------------------------------

    fn help_page(&mut self, lines: &[(i32, &str)]) {
        self.clear_area(0, 90, 311, 230);
        for &(y, line) in lines {
            self.print_centred(160, y, Page::Visible, TEXT_COLOR, line);
        }
    }

    /// Whether the help was left with Escape.
    fn escaped(&self) -> bool {
        self.keyboard.last_key_was(KeyCode::Escape)
    }

    pub async fn help(&mut self) {
        let page = Page::Visible;

        self.help_page(&[
            (95, "Use the mouse or keyboard to move"),
            (110, "the ship     around. Collect the"),
            (125, "gems    and avoid the mines    ."),
            (140, "Button 1 fires your bullets, while"),
            (155, "button 2 or the spacebar activates"),
            (170, "a SmartBomb if you have any."),
            (215, "Hit a button or key to continue."),
        ]);
        let g = &self.gfx;
        let (ship, crystal, mine) = (
            g.ship[0].bitmap.clone(),
            g.crystal.bitmap.clone(),
            g.mine.bitmap.clone(),
        );
        self.vga.draw_transparent(118, 111, page, &ship);
        self.vga.draw_transparent(95, 126, page, &crystal);
        self.vga.draw_transparent(240, 126, page, &mine);
        self.wait_for_input(true).await;
        if self.escaped() {
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
        ]);
        let smart_bomb = self.gfx.smart_bomb.bitmap.clone();
        self.vga.draw_transparent(207, 96, page, &smart_bomb);
        self.animate_until_input(EnemyKind::PowerCharge, 187, 110)
            .await;
        if self.escaped() {
            return;
        }

        self.help_page(&[
            (95, "There are many types of nasties just"),
            (110, "waiting to do some damage..."),
        ]);
        self.enemy_parade().await;
        if self.escaped() {
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
        ]);
        self.wait_for_input(true).await;
        if self.escaped() {
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
        ]);
        self.wait_for_input(true).await;
    }

    async fn animate_until_input(&mut self, kind: EnemyKind, x: i32, y: i32) {
        let mut frame = 0;

        loop {
            let info = kind.info();
            frame += info.frame_speed;
            if frame >= (info.frames as i32) << 8 {
                frame = 0;
            }

            let sprite = self.gfx.enemy(kind)[(frame >> 8) as usize].bitmap.clone();
            let (w, h) = (sprite.width, sprite.height);
            self.stars.forget(x, y, x + w, y + h);
            self.vga.draw_transparent(x, y, Page::Visible, &sprite);
            self.star_field_step().await;
            self.vga.fill_rect(x, y, x + w, y + h, Page::Visible, 0);

            if self.buttons_pressed().any() || self.keyboard.take_any_key() {
                break;
            }
        }
    }

    /// Each kind of enemy flies across the screen in turn, with its name and score.
    async fn enemy_parade(&mut self) {
        let kinds: Vec<EnemyKind> = EnemyKind::ALL
            .into_iter()
            .filter(|k| {
                !matches!(
                    k,
                    EnemyKind::PowerCharge
                        | EnemyKind::Explosion
                        | EnemyKind::Unused
                        | EnemyKind::Tribble
                )
            })
            .collect();
        let (mut k, mut x, mut frame) = (0, 0, 0);

        loop {
            x += 2;
            let kind = kinds[k];
            let info = kind.info();
            let sprite = self.gfx.enemy(kind)[(frame >> 8) as usize].bitmap.clone();
            let (w, h) = (sprite.width, sprite.height);

            self.stars.forget(x, 140, x + w, 140 + h);
            self.vga.draw_transparent(x, 140, Page::Visible, &sprite);
            self.star_field_step().await;
            frame += info.frame_speed;
            if frame >= (info.frames as i32) << 8 {
                frame = 0;
            }
            self.vga.fill_rect(x, 140, x + w, 140 + h, Page::Visible, 0);

            if x > 310 {
                x = 0;
                k = (k + 1) % kinds.len();
                frame = 0;
            }
            if x == 120 {
                self.clear_area(80, 170, 240, 210);
                let kind = kinds[k];
                self.print_centred(160, 170, Page::Visible, TEXT_COLOR, kind.info().name);
                self.print_centred(
                    160,
                    195,
                    Page::Visible,
                    TEXT_COLOR,
                    &format!("{} points", kind.info().score),
                );
            }
            if x == 200 {
                self.clear_area(80, 170, 240, 210);
            }

            if self.buttons_pressed().any() || self.keyboard.take_any_key() {
                break;
            }
        }
    }
}
