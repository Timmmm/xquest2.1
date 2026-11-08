//! Drawing the moving sprites.
//!
//! Each frame is drawn on the hidden page. Before a sprite is drawn the screen under it is
//! saved, and on the next frame drawn to the same page it is put back. Because the pages
//! alternate, each sprite keeps two saved backgrounds, swapping them every frame.

use super::*;

impl XQuest {
    /// Restores the backgrounds saved when the hidden page was last drawn.
    pub(super) fn erase_sprites(&mut self) {
        let page = Page::Hidden;
        let erase =
            |vga: &mut Vga, b: &Body| vga.draw_opaque(b.old_x, b.old_y, page, &b.background);

        erase(&mut self.vga, &self.ship.body);
        self.missiles
            .iter()
            .rev()
            .for_each(|m| erase(&mut self.vga, &m.body));
        self.enemies
            .iter()
            .rev()
            .for_each(|e| erase(&mut self.vga, &e.body));
        self.enemy_missiles
            .iter()
            .rev()
            .for_each(|m| erase(&mut self.vga, &m.body));

        if self.gate_moving {
            for g in self.gates {
                self.vga.fill_rect(g.x, 0, g.x + g.width, g.height, page, 0);
            }
        }

        if self.show_timer {
            let t = &self.timer;
            self.vga.draw_opaque(t.old_x, t.old_y, page, &t.background);
        }

        // Remove the objects hit last frame.
        for i in (0..self.objects.len()).rev() {
            if self.objects[i].hit {
                let o = self.objects.swap_remove(i);
                self.vga.draw_opaque(o.x, o.y, page, &o.background);
                self.redraw_enemy_mines(o.x, o.y, o.xbr, o.ybr, page);
            }
        }

        // A mine laid last frame was only drawn on the other page.
        if self.enemy_mine_pending {
            let m = *self.enemy_mines.last().unwrap();
            self.vga
                .draw_transparent(m.x, m.y, page, &self.gfx.enemy_mine.bitmap);
            self.enemy_mine_pending = false;
        }
    }

    /// Saves the backgrounds of the sprites and draws them on the hidden page.
    pub(super) fn draw_sprites(&mut self) {
        let page = Page::Hidden;

        fn save(vga: &Vga, x: i32, y: i32, w: i32, h: i32, current: &mut Bitmap, old: &mut Bitmap) {
            *current = vga.capture(x, y, w, h, Page::Hidden);
            std::mem::swap(current, old);
        }

        if self.show_timer {
            let (x, y) = self.timer_position();
            let t = &mut self.timer;
            (t.old_x, t.old_y) = (t.x, t.y);
            (t.x, t.y) = (x, y);
            save(
                &self.vga,
                x,
                y,
                36,
                self.gfx.digits[0].height,
                &mut t.background,
                &mut t.old_background,
            );
        }

        // Nothing is saved or drawn once the ship has left through the top.
        let ship_visible = self.ship.body.y >= 0;
        if ship_visible {
            let (w, h) = (self.ship.width, self.ship.height);
            let b = &mut self.ship.body;
            save(
                &self.vga,
                b.x,
                b.y,
                w,
                h,
                &mut b.background,
                &mut b.old_background,
            );
        }

        let missile_height = self.gfx.ship_missile.height();
        for m in self.missiles.iter_mut().rev() {
            let b = &mut m.body;
            save(
                &self.vga,
                b.x,
                b.y,
                4,
                missile_height,
                &mut b.background,
                &mut b.old_background,
            );
        }

        for e in self.enemies.iter_mut().rev() {
            let sprite = &self.gfx.enemies[e.kind.index()][0];
            let b = &mut e.body;
            save(
                &self.vga,
                b.x,
                b.y,
                sprite.width(),
                sprite.height(),
                &mut b.background,
                &mut b.old_background,
            );
        }

        for m in self.enemy_missiles.iter_mut().rev() {
            let h = self.gfx.enemy_missiles[m.kind.index()].height();
            let b = &mut m.body;
            save(
                &self.vga,
                b.x,
                b.y,
                4,
                h,
                &mut b.background,
                &mut b.old_background,
            );
        }

        if self.gate_moving {
            for (g, sprite) in self
                .gates
                .iter()
                .zip([&self.gfx.gate_left, &self.gfx.gate_right])
            {
                self.vga.draw_opaque(g.x, 0, page, &sprite.bitmap);
            }
        }

        // The ship disappears a frame after it is destroyed.
        if ship_visible && self.ship_destroyed_count > SHIP_DESTROYED_FRAMES - 2 {
            let b = &self.ship.body;
            self.vga
                .draw_transparent(b.x, b.y, page, &self.gfx.ship[self.ship.direction].bitmap);
        }

        for m in self.missiles.iter().rev() {
            self.vga
                .draw_transparent(m.body.x, m.body.y, page, &self.gfx.ship_missile.bitmap);
        }

        for e in self.enemies.iter().rev() {
            let sprite = &self.gfx.enemy(e.kind)[e.frame_index()];
            self.vga
                .draw_transparent(e.body.x, e.body.y, page, &sprite.bitmap);
        }

        for m in self.enemy_missiles.iter().rev() {
            self.vga.draw_transparent(
                m.body.x,
                m.body.y,
                page,
                &self.gfx.enemy_missile(m.kind).bitmap,
            );
        }

        self.draw_timer();
    }

    /// Draws the time left to beat the par time, in seconds and hundredths.
    fn draw_timer(&mut self) {
        if !self.show_timer {
            return;
        }

        let rate = NOMINAL_FRAME_RATE as i64;
        let left = self.level().par_time as i64 * rate - self.progress[self.player].time_on_level;
        if left < 0 {
            return;
        }

        let hundredths = (left % rate) * 100 / rate;
        let seconds = left / rate % 60;
        let minutes = left / rate / 60;

        let text = match (minutes, seconds) {
            (0, 0) => format!("{hundredths:02}"),
            (0, s) => format!("{s}:{hundredths:02}"),
            (m, s) => format!("{m}:{s:02}:{hundredths:02}"),
        };

        // The colons are left as gaps.
        let (x, y) = (self.timer.x, self.timer.y);
        for (i, c) in format!("{text:>7}").chars().enumerate() {
            if let Some(d) = c.to_digit(10) {
                self.vga.draw_transparent(
                    x + i as i32 * 5,
                    y,
                    Page::Hidden,
                    &self.gfx.digits[d as usize],
                );
            }
        }
    }
}
