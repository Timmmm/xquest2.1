//! Setting up levels, the status bar and the end of level bonus.

use super::*;

const BORDER_COLORS: [u8; 5] = [10, 15, 25, 15, 10];
/// Colour of the barrier across the exit gate while crystals remain.
const BARRIER_COLOR: u8 = 54;
const GATE_MIN_X: i32 = 20;
const GATE_MAX_X: i32 = PAGE_WIDTH - 30;
const OBJECT_MIN_X: i32 = 11;
const OBJECT_MAX_X: i32 = PAGE_WIDTH - 21;
const OBJECT_MIN_Y: i32 = 14;
const OBJECT_MAX_Y: i32 = PAGE_HEIGHT - 21;
const TIMER_X: i32 = 280;
const TIMER_Y: i32 = 10;

/// The status bar background colour.
pub const STATUS_COLOR: u8 = 14;

impl XQuest {
    /// Whether the newest object is clear of the others, the ship's start position, the
    /// attractor, the enemy doors and (if it does not move) the exit gate.
    fn object_is_clear(&self, player: usize) -> bool {
        let (w, h) = (self.gfx.crystal.width(), self.gfx.crystal.height());
        let n = self.objects.len();
        let mut clear = true;

        // The original checks every pair each time, not just the new object.
        for i in 0..n {
            for j in (i + 1)..n {
                let (a, b) = (&self.objects[i], &self.objects[j]);
                if (a.x - b.x).abs() < w + 2 && (a.y - b.y).abs() < h + 2 {
                    clear = false;
                }
            }
        }

        let o = &self.objects[n - 1];
        if o.xbr + 8 >= SHIP_START_X
            && o.ybr + 8 >= SHIP_START_Y
            && o.x <= SHIP_START_X + self.ship.width + 8
            && o.y <= SHIP_START_Y + self.ship.height + 8
        {
            clear = false;
        }

        let p = &self.progress[player];
        let (ax, ay) = p.attractor;
        if p.rank > 0
            && o.xbr + 8 >= ax
            && o.ybr + 8 >= ay
            && o.x <= ax + 13 + 8
            && o.y <= ay + 13 + 8
        {
            clear = false;
        }

        if o.y < PAGE_HEIGHT / 2 + 7
            && o.ybr > PAGE_HEIGHT / 2 - 7
            && (o.x < SHIP_MIN_X + 15 || o.xbr > SHIP_MAX_X - 15)
        {
            clear = false;
        }

        if LEVELS[(p.level - 1) as usize].gate_speed == 0
            && o.y < SHIP_MIN_Y + 18
            && o.xbr > self.gates[0].x
            && o.x < self.gates[1].x
        {
            clear = false;
        }

        clear
    }

    /// Chooses the objects, gate and attractor positions for a player's level.
    pub fn set_up_level(&mut self, player: usize) {
        let level = LEVELS[(self.progress[player].level - 1) as usize];

        let p = &mut self.progress[player];
        let bonus_step = if p.total_level <= LEVELS.len() as i32 {
            level
        } else {
            LEVELS[LEVELS.len() - 1]
        };
        p.next_bonus_life_score = p.last_bonus_life_score + bonus_step.bonus_life_score;
        p.mines = level.mines;
        p.crystals = level.crystals;
        p.smart_bomb_pickups = 0;
        p.time_on_level = 0;
        self.gate_move_count = 0;

        self.gates[0].x = (PAGE_WIDTH - level.gate_width) / 2 - self.gates[0].width;
        self.gates[1].x = (PAGE_WIDTH + level.gate_width) / 2;

        loop {
            let r = self.rng.unit();
            if !(r < level.smart_bomb_prob
                && self.progress[player].smart_bomb_pickups < level.max_smart_bombs)
            {
                break;
            }
            self.progress[player].smart_bomb_pickups += 1;
        }

        let p = self.progress[player];
        let object_count = p.mines + p.crystals + p.smart_bomb_pickups;
        self.no_mines = p.mines == 0;

        loop {
            let ax = self.rng.below(PAGE_WIDTH - 120) + 60;
            let ay = self.rng.below(PAGE_HEIGHT - 120) + 60;
            self.progress[player].attractor = (ax, ay);
            if (ax - SHIP_START_X).abs() + (ay - SHIP_START_Y).abs() > 60 {
                break;
            }
        }

        let (w, h) = (self.gfx.crystal.width(), self.gfx.crystal.height());
        self.objects.clear();

        for i in 0..object_count {
            let kind = if i < p.crystals {
                ObjectKind::Crystal
            } else if i < p.crystals + p.mines {
                ObjectKind::Mine
            } else {
                ObjectKind::SmartBomb
            };
            self.objects.push(Object {
                kind,
                ..Default::default()
            });

            loop {
                let x = self.rng.below(OBJECT_MAX_X - OBJECT_MIN_X) + OBJECT_MIN_X;
                let y = self.rng.below(OBJECT_MAX_Y - OBJECT_MIN_Y) + OBJECT_MIN_Y;
                let o = self.objects.last_mut().unwrap();
                (o.x, o.y, o.xbr, o.ybr) = (x, y, x + w - 1, y + h - 1);
                if self.object_is_clear(player) {
                    break;
                }
            }
        }

        self.game_speed = self.base_game_speed();
    }

    pub(super) fn object_sprite(&self, kind: ObjectKind) -> &crate::core::bitmap::Sprite {
        match kind {
            ObjectKind::Crystal => &self.gfx.crystal,
            ObjectKind::Mine => &self.gfx.mine,
            ObjectKind::SmartBomb => &self.gfx.smart_bomb,
        }
    }

    /// Draws the level and resets everything that moves, at the start of a level or life.
    pub async fn start_level(&mut self) {
        self.enemies.clear();
        self.enemy_missiles.clear();
        self.enemy_mines.clear();
        self.enemy_mine_pending = false;
        self.level_finished = false;
        self.ship_destroyed = false;
        self.entering_left = EnemyEntry::default();
        self.entering_right = EnemyEntry::default();
        self.gate_moving = self.level().gate_speed != 0;

        self.vga.clear_page(Page::Page0);
        self.vga.clear_page(Page::Page1);

        self.scroll_x = (PAGE_WIDTH - VIEW_WIDTH) / 2;
        self.scroll_y = (self.vga.page_height - VIEW_HEIGHT) / 2;
        self.page_flip().await;

        let [gl, gr] = self.gates;
        let page = Page::Page0;

        for (i, &c) in BORDER_COLORS.iter().enumerate() {
            let i = i as i32 + 1;
            self.vga.line(10, 10 - i, gl.x, 10 - i, page, c);
            self.vga
                .line(gr.x + gr.width, 10 - i, PAGE_WIDTH - 10, 10 - i, page, c);
            self.vga.line(
                10,
                PAGE_HEIGHT - 5 - i,
                PAGE_WIDTH - 10,
                PAGE_HEIGHT - 5 - i,
                page,
                c,
            );
            self.vga.line(4 + i, 10, 4 + i, PAGE_HEIGHT - 9, page, c);
            self.vga.line(
                PAGE_WIDTH - 11 + i,
                10,
                PAGE_WIDTH - 11 + i,
                PAGE_HEIGHT - 9,
                page,
                c,
            );
        }

        let g = &self.gfx;
        self.vga.draw_opaque(gl.x, 0, page, &g.gate_left.bitmap);
        self.vga.draw_opaque(gr.x, 0, page, &g.gate_right.bitmap);
        if self.progress[self.player].crystals > 0 {
            self.vga
                .line(gl.x + gl.width, 7, gr.x - 1, 7, page, BARRIER_COLOR);
        }
        self.vga.draw_opaque(0, 0, page, &g.corner_top_left);
        self.vga
            .draw_opaque(PAGE_WIDTH - 10, 0, page, &g.corner_top_right);
        self.vga.draw_opaque(
            PAGE_WIDTH - 10,
            PAGE_HEIGHT - 10,
            page,
            &g.corner_bottom_right,
        );
        self.vga
            .draw_opaque(0, PAGE_HEIGHT - 10, page, &g.corner_bottom_left);
        self.vga
            .draw_opaque(0, PAGE_HEIGHT / 2 - 10, page, &g.enemy_gate_left[0]);
        self.vga.draw_opaque(
            PAGE_WIDTH - 20,
            PAGE_HEIGHT / 2 - 10,
            page,
            &g.enemy_gate_right[0],
        );

        let p = self.progress[self.player];
        if p.rank > 0 {
            self.vga
                .draw_opaque(p.attractor.0 - 6, p.attractor.1 - 6, page, &g.attractor);
        }

        // Background stars: 100 bright and 400 dim.
        for (count, colors, base) in [(100, 20, 10), (400, 15, 5)] {
            for _ in 0..count {
                let x = self.rng.below(SHIP_MAX_X - SHIP_MIN_X - 2) + SHIP_MIN_X + 1;
                let y = self.rng.below(SHIP_MAX_Y - SHIP_MIN_Y - 5) + SHIP_MIN_Y + 5;
                let c = self.rng.below(colors) + base;
                self.vga.set_pixel(x, y, page, c as u8);
            }
        }

        // Objects hit just before the end of the last life are removed now.
        let mut i = self.objects.len();
        while i > 0 {
            i -= 1;
            if self.objects[i].hit {
                self.objects.swap_remove(i);
            }
        }

        let ship = &mut self.ship;
        let b = &mut ship.body;
        (b.x, b.y) = (SHIP_START_X, SHIP_START_Y);
        (b.xbr, b.ybr) = (b.x + ship.width - 1, b.y + ship.height - 1);
        (b.old_x, b.old_y) = (b.x, b.y);
        (b.sx, b.sy) = (b.x << 6, b.y << 6);
        (b.dx, b.dy) = (0, 0);
        ship.direction = 0;
        self.missiles.clear();
        b.background = self.vga.capture(b.x, b.y, ship.width, ship.height, page);
        b.old_background = b.background.clone();

        for o in self.objects.iter_mut() {
            let sprite = match o.kind {
                ObjectKind::Crystal => &self.gfx.crystal,
                ObjectKind::Mine => &self.gfx.mine,
                ObjectKind::SmartBomb => &self.gfx.smart_bomb,
            };
            o.background = self
                .vga
                .capture(o.x, o.y, sprite.width(), sprite.height(), page);
            self.vga.draw_transparent(o.x, o.y, page, &sprite.bitmap);
        }

        self.vga.copy_page(Page::Page0, Page::Page1);

        self.draw_window(0, 0, 319, 24, Page::Split);
        self.add_to_score(0);
        self.show_lives();
        self.show_smart_bombs();
        self.show_crystals();

        let t = &mut self.timer;
        (t.x, t.y) = (TIMER_X + self.scroll_x, TIMER_Y + self.scroll_y);
        (t.old_x, t.old_y) = (t.x, t.y);
        // The original saves 32 pixels here but 36 every frame.
        t.background = self
            .vga
            .capture(t.x, t.y, 32, self.gfx.digits[0].height, page);
        t.old_background = t.background.clone();
        self.show_power_ups();

        self.mouse.take_motion();
        self.mouse.take_presses();
    }

    pub(super) fn timer_position(&self) -> (i32, i32) {
        (
            (self.scroll_x + TIMER_X).min(PAGE_WIDTH - 46),
            self.scroll_y + TIMER_Y,
        )
    }

    // ------------------------------------------------------------------------------------
    // The status bar.
    // ------------------------------------------------------------------------------------

    fn show_counter(&mut self, x: i32, value: i32, icon: fn(&Graphics) -> &Bitmap) {
        self.draw_text(x, 5, true, Page::Split, &format!("  {value:2}"));
        let icon = icon(&self.gfx).clone();
        self.vga.draw_opaque(x, 6, Page::Split, &icon);
    }

    pub fn show_lives(&mut self) {
        let lives = self.progress[self.player].lives - 1;
        self.show_counter(185, lives, |g| &g.status_ship);
    }

    pub fn show_smart_bombs(&mut self) {
        let bombs = self.progress[self.player].smart_bombs;
        self.show_counter(230, bombs, |g| &g.status_smart_bomb);
    }

    pub fn show_crystals(&mut self) {
        let crystals = self.progress[self.player].crystals;
        self.show_counter(275, crystals, |g| &g.status_crystal);
    }

    /// Shows the icons of the active power-ups (as many as fit).
    pub fn show_power_ups(&mut self) {
        let mut x = 85;

        for kind in PowerUpKind::ALL {
            if self.power_ups.active(kind) && x <= 157 {
                self.power_ups[kind].icon_x = x;
                self.vga
                    .draw_opaque(x, 5, Page::Split, &self.gfx.powerups[kind.index()]);
                x += 18;
            } else {
                self.power_ups[kind].icon_x = 0;
            }
        }

        if x <= 157 {
            self.vga
                .fill_rect(x, 5, 157 + 17, 20, Page::Split, STATUS_COLOR);
        }
    }

    fn bonus_life(&mut self) {
        let p = self.progress();
        p.last_bonus_life_score = p.next_bonus_life_score;
        let level = if p.total_level <= LEVELS.len() as i32 {
            LEVELS[(p.level - 1) as usize]
        } else {
            LEVELS[LEVELS.len() - 1]
        };
        p.next_bonus_life_score = p.last_bonus_life_score + level.bonus_life_score;
        p.lives += 1;

        self.show_lives();
        self.play(Sound::WooHoo);
    }

    pub fn add_to_score(&mut self, points: i64) {
        self.progress().score += points;

        while self.progress[self.player].score >= self.progress[self.player].next_bonus_life_score {
            self.bonus_life();
        }

        let score = self.progress[self.player].score;
        self.draw_text(10, 5, true, Page::Split, &format!("{score:8}"));
    }

    // ------------------------------------------------------------------------------------
    // The end of a level.
    // ------------------------------------------------------------------------------------

    /// Shows the level completed window and counts up the bonus for beating the par time.
    pub async fn give_time_bonus(&mut self) {
        self.page_flip().await;
        let b = &self.ship.body;
        let (x, y, background) = (b.old_x, b.old_y, b.background.clone());
        self.vga.draw_opaque(x, y, Page::Visible, &background);

        let p = self.progress[self.player];
        let par = self.level().par_time as i64;
        let time_taken = p.time_on_level / NOMINAL_FRAME_RATE as i64;
        let page = Page::Visible;

        let (sx, sy) = (self.scroll_x, self.scroll_y);
        self.draw_window(50 + sx, 40 + sy, 270 + sx, 160 + sy, page);
        let (wx, wy) = (self.window_x, self.window_y);
        self.draw_text(
            wx + 42,
            wy + 15,
            true,
            page,
            &format!("LEVEL {} COMPLETED", p.total_level),
        );
        self.draw_text(
            wx + 20,
            wy + 40,
            true,
            page,
            &format!("TIME TAKEN: {time_taken:3} SECONDS"),
        );
        self.draw_text(
            wx + 20,
            wy + 65,
            true,
            page,
            &format!("PAR       : {par:3} SECONDS"),
        );

        let bonus = ((par - time_taken) * 500).max(0);
        let steps = bonus / 500;
        self.draw_text(wx + 20, wy + 90, true, page, "BONUS     :  0         ");

        // Count up the bonus, 500 points at a time, unless a key is pressed.
        if !self.wait_or_input(frames(0.5)).await {
            let mut i = 1;
            while i <= steps {
                self.draw_text(wx + 124, wy + 90, false, page, &(i * 500).to_string());
                self.draw_text(
                    wx + 116,
                    wy + 40,
                    false,
                    page,
                    &format!("{:3}", time_taken + i),
                );
                if i % 3 == 1 {
                    self.play(Sound::Countdown);
                }
                if self.wait_or_input(frames(1.0 / 12.0)).await && i != steps {
                    i = steps - 1;
                }
                i += 1;
            }
        } else {
            self.draw_text(wx + 124, wy + 90, false, page, &bonus.to_string());
            self.draw_text(
                wx + 116,
                wy + 40,
                false,
                page,
                &format!("{:3}", time_taken + steps),
            );
            if bonus > 0 {
                self.play(Sound::Countdown);
            }
        }

        // Then count it down into the score.
        if bonus > 0 {
            if !self.wait_or_input(frames(0.5)).await {
                let mut i = steps - 1;
                while i >= 0 {
                    self.draw_text(wx + 124, wy + 90, false, page, &format!("{}    ", i * 500));
                    self.add_to_score(500);
                    self.play(Sound::Countdown);
                    if self.wait_or_input(frames(1.0 / 12.0)).await && i != 0 {
                        self.add_to_score((i - 1) * 500);
                        i = 1;
                    }
                    i -= 1;
                }
            } else {
                self.draw_text(wx + 124, wy + 90, false, page, "0    ");
                self.add_to_score(bonus);
                self.play(Sound::Countdown);
            }
        }

        self.wait_or_input(frames(2.5)).await;
    }

    pub async fn show_rank(&mut self) {
        let (sx, sy) = (self.scroll_x, self.scroll_y);
        let page = Page::Visible;
        self.draw_window(50 + sx, 40 + sy, 270 + sx, 160 + sy, page);

        let wy = self.window_y;
        let rank = RANKS[(self.progress[self.player].rank - 1) as usize];
        for (dy, line) in [
            (15, "CONGRATULATIONS"),
            (40, "YOU HAVE ATTAINED THE"),
            (65, "RANK OF"),
            (90, rank),
        ] {
            self.draw_text_centred(160 + sx, wy + dy, page, line);
        }

        self.wait_for_input(false).await;
    }

    /// Moves the exit gate on the later levels.
    pub(super) fn move_gate(&mut self) {
        let distance = self.gate_move_count / 64;
        self.gate_move_count %= 64;

        if self.rng.unit() < self.level().gate_change_dir_prob {
            let p = self.progress();
            p.gate_moving_right = !p.gate_moving_right;
        }

        if distance == 0 {
            return;
        }

        if self.progress[self.player].gate_moving_right {
            self.gates.iter_mut().for_each(|g| g.x += distance);
            if self.gates[1].x > GATE_MAX_X {
                self.progress().gate_moving_right = false;
            }
        } else {
            self.gates.iter_mut().for_each(|g| g.x -= distance);
            if self.gates[0].x < GATE_MIN_X {
                self.progress().gate_moving_right = true;
            }
        }

        if self.progress[self.player].crystals > 0 {
            let [gl, gr] = self.gates;
            for page in [Page::Page0, Page::Page1] {
                self.vga
                    .line(gl.x + gl.width, 7, gr.x - 1, 7, page, BARRIER_COLOR);
            }
        }
    }

    /// Opens the exit when the last crystal is collected.
    pub(super) fn open_gate(&mut self) {
        let [gl, gr] = self.gates;
        for page in [Page::Visible, Page::Hidden] {
            self.vga.fill_rect(gl.x + gl.width, 0, gr.x, 11, page, 0);
        }
        self.play(Sound::Gate);
    }
}
