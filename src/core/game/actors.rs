//! Enemies, missiles, the ship and collisions.

use std::f64::consts::PI;

use super::*;
use crate::core::bitmap::masks_collide;
use crate::core::rng::{fixed_to_pixel_unsigned, shl32};

/// How long a power charge stays, in frames: a minimum and a random extra.
const POWER_CHARGE_LIFETIME: (i32, i32) = (5 * NOMINAL_FRAME_RATE, 5 * NOMINAL_FRAME_RATE);
const AIMED_FIRE_SPEED: i32 = 256;

/// The original turns angles round with this approximation of pi, which slightly affects
/// which way the ship points.
#[allow(clippy::approx_constant)]
const ROUGH_PI: f64 = 3.14159;

/// Whether two sprites overlap, given their masks and positions.
fn sprites_collide(a: &[u32], ax: i32, ay: i32, b: &[u32], bx: i32, by: i32) -> bool {
    (ax <= bx && masks_collide(a, ax, ay, b, bx, by))
        || (bx < ax && masks_collide(b, bx, by, a, ax, ay))
}

/// The pixel collision test used by the original: when the boxes start at the same column
/// it is done (redundantly) from both sides.
fn sprites_collide_inclusive(a: &[u32], ax: i32, ay: i32, b: &[u32], bx: i32, by: i32) -> bool {
    (ax <= bx && masks_collide(a, ax, ay, b, bx, by))
        || (bx <= ax && masks_collide(b, bx, by, a, ax, ay))
}

impl XQuest {
    // ------------------------------------------------------------------------------------
    // Enemies.
    // ------------------------------------------------------------------------------------

    pub fn add_enemy(&mut self, x: i32, y: i32, kind: EnemyKind) {
        if self.enemies.len() >= MAX_ENEMIES {
            return;
        }

        let info = kind.info();
        let speed = self.game_speed;
        let (mut dx, mut dy) = (0, 0);
        let mut lifetime = 0;

        if !info.fixed_speed {
            if info.speed > 0 {
                dx = wrap16((self.rng.below(info.speed) - info.speed_offset) * speed / 64);
                dy = wrap16((self.rng.below(info.speed) - info.speed_offset) * speed / 64);
            }
            if kind == EnemyKind::Explosion {
                lifetime = (info.frames as i32 - 1) * (256 / info.frame_speed);
            }
        } else {
            let cos = (self.rng.unit() * 2.0 * PI - PI).cos();
            dx = wrap16((info.speed as f64 * cos).round() as i32 * speed / 64);
            dy = wrap16((info.speed as f64 * (1.0 - cos * cos)).round() as i32 * speed / 64);
        }

        if kind == EnemyKind::PowerCharge {
            lifetime = POWER_CHARGE_LIFETIME.0 + self.rng.below(POWER_CHARGE_LIFETIME.1);
        }

        let curve_sin = wrap16(self.rng.below(info.curve) - info.curve_offset);
        let curve_cos = curve_cos(curve_sin);

        let sprite = &self.gfx.enemy(kind)[0];
        let (w, h) = (sprite.width(), sprite.height());
        let background = self.vga.capture(x, y, w, h, Page::Hidden);

        let body = Body {
            x,
            y,
            xbr: x + w - 1,
            ybr: y + h - 1,
            sx: wrap16(x << 6),
            sy: wrap16(y << 6),
            dx,
            dy,
            old_x: x,
            old_y: y,
            old_background: background.clone(),
            background,
        };

        self.enemies.push(Enemy {
            body,
            kind,
            curve_cos,
            curve_sin,
            hits_left: info.hits,
            frame: 0,
            lifetime,
        });
    }

    /// Creates an enemy missile at the centre of enemy `i`, with the given velocity, or
    /// a velocity worked out from its kind if `None`.
    fn spawn_enemy_missile(
        &mut self,
        i: usize,
        kind: EnemyMissileKind,
        velocity: Option<(i32, i32)>,
    ) {
        let e = &self.enemies[i];
        let width = self.gfx.enemy(e.kind)[0].width();
        // The original uses the width for both coordinates.
        let sx = wrap16(e.body.sx + (width << 5));
        let sy = wrap16(e.body.sy + (width << 5));
        let (x, y) = (fixed_to_pixel_unsigned(sx), fixed_to_pixel_unsigned(sy));

        let sprite = self.gfx.enemy_missile(kind);
        let (w, h) = (sprite.width(), sprite.height());
        let background = self.vga.capture(x, y, 4, h, Page::Hidden);

        let info = kind.info();
        self.play_opt(info.sound);

        let (dx, dy) = velocity.unwrap_or_else(|| {
            let speed = self.game_speed;
            if info.aimed {
                let dx = self.ship.body.x - x;
                let dy = self.ship.body.y - y;
                let distance = if dx.abs() > dy.abs() {
                    dx.abs() + dy.abs() / 2
                } else {
                    dy.abs() + dx.abs() / 2
                };
                let distance = distance.max(1);
                (
                    wrap16(dx * info.speed / distance * speed / 64),
                    wrap16(dy * info.speed / distance * speed / 64),
                )
            } else {
                let dx = wrap16((self.rng.below(info.speed) - (info.speed >> 1)) * speed / 64);
                let dy = wrap16((self.rng.below(info.speed) - (info.speed >> 1)) * speed / 64);
                (dx, dy)
            }
        });

        let body = Body {
            x,
            y,
            xbr: x + w - 1,
            ybr: y + h - 1,
            sx,
            sy,
            dx,
            dy,
            old_x: x,
            old_y: y,
            old_background: background.clone(),
            background,
        };
        self.enemy_missiles.push(EnemyMissile { body, kind });
    }

    fn fire_enemy_missile(&mut self, i: usize) {
        if self.enemy_missiles.len() >= MAX_ENEMY_MISSILES {
            return;
        }
        if let Some(kind) = self.enemies[i].kind.info().missile {
            self.spawn_enemy_missile(i, kind, None);
        }
    }

    /// Bursts enemy `i` into up to 15 missiles flying outwards.
    fn explode(&mut self, i: usize) {
        let Some(kind) = self.enemies[i].kind.info().missile else {
            return;
        };
        let count = (MAX_ENEMY_MISSILES - self.enemy_missiles.len()).min(15);

        let directions = self.fragment_directions;
        for &(cos, sin) in &directions[..count] {
            let speed = kind.info().speed;
            let dx = wrap16(cos * speed / 32768 * self.game_speed / 64);
            let dy = wrap16(sin * speed / 32768 * self.game_speed / 64);
            self.spawn_enemy_missile(i, kind, Some((dx, dy)));
        }
    }

    /// Removes enemy `i`, erasing it from the screen. The last enemy takes its place.
    pub fn delete_enemy(&mut self, i: usize) {
        let e = self.enemies.swap_remove(i);
        self.vga.draw_opaque(
            e.body.old_x,
            e.body.old_y,
            Page::Visible,
            &e.body.old_background,
        );
    }

    pub(super) fn clear_enemies(&mut self) {
        while let Some(e) = self.enemies.pop() {
            self.vga.draw_opaque(
                e.body.old_x,
                e.body.old_y,
                Page::Visible,
                &e.body.old_background,
            );
        }
    }

    /// Enemy `i` has been hit, by something moving at `(dx, dy)`. If `explosion` is set an
    /// explosion is left behind.
    fn enemy_hit(&mut self, i: usize, dx: i32, dy: i32, mut explosion: bool) {
        let heavy = self.power_ups.active(PowerUpKind::HeavyFire);

        if heavy {
            self.enemies[i].hits_left = 0;
        } else {
            self.enemies[i].hits_left -= 1;
        }

        if self.enemies[i].hits_left > 0 {
            self.play(Sound::Doh);
            return;
        }

        let kind = self.enemies[i].kind;
        let info = kind.info();
        let knocked_back = info.rebounds && !heavy;

        if info.shoots_back {
            self.fire_enemy_missile(i);
        }
        if knocked_back {
            self.play(Sound::Ow);
            explosion = false;
        }
        if info.explodes {
            self.explode(i);
        }
        self.add_to_score(info.score as i64);

        if explosion {
            if info.death_sound == Some(Sound::Explosion) {
                let variant = self.rng.below(3);
                self.play(Sound::explosion(variant));
            } else {
                self.play_opt(info.death_sound);
            }

            let (x, y) = (self.enemies[i].body.x, self.enemies[i].body.y);
            if info.tribbles {
                for _ in 0..5 {
                    self.add_enemy(x, y, kind.offspring());
                }
            }

            self.delete_enemy(i);
            self.add_enemy(x, y, EnemyKind::Explosion);
        } else if knocked_back {
            let b = &mut self.enemies[i].body;
            b.dx = wrap16(3 * b.dx / 4 + dx / 4);
            b.dy = wrap16(3 * b.dy / 4 + dy / 4);
        } else {
            self.delete_enemy(i);
        }
    }

    fn fire_smart_bomb(&mut self) {
        if self.progress[self.player].smart_bombs > 0 {
            for i in (0..self.enemies.len()).rev() {
                self.enemies[i].hits_left = 1;
                let explosion = !matches!(
                    self.enemies[i].kind,
                    EnemyKind::PowerCharge | EnemyKind::Explosion
                );
                self.enemy_hit(i, 0, 0, explosion);
            }

            for m in self.enemy_missiles.drain(..).rev() {
                self.vga.draw_opaque(
                    m.body.old_x,
                    m.body.old_y,
                    Page::Visible,
                    &m.body.old_background,
                );
            }

            self.progress().smart_bombs -= 1;
            self.vga.set_color(0, SMART_BOMB_FLASH[9]);
            self.smart_bomb_flash = 11;
            self.play(Sound::Explosion);
        }

        self.show_smart_bombs();
    }

    fn add_enemy_mine(&mut self, i: usize) {
        if self.enemy_mine_pending || self.enemy_mines.len() >= MAX_ENEMY_MINES {
            return;
        }

        let e = &self.enemies[i];
        let half_width = self.gfx.enemy(e.kind)[0].width() >> 1;
        // The original uses the width for both coordinates.
        let (x, y) = (e.body.x + half_width, e.body.y + half_width);
        let mine = &self.gfx.enemy_mine;
        self.enemy_mines.push(Rect {
            x,
            y,
            xbr: x + mine.width() - 1,
            ybr: y + mine.height() - 1,
        });

        self.vga.draw_transparent(x, y, Page::Hidden, &mine.bitmap);
        self.enemy_mine_pending = true;
        self.play(Sound::Squelch);
    }

    fn draw_enemy_door(&mut self, side: Side, frame: usize) {
        let (x, picture) = match side {
            Side::Left => (0, &self.gfx.enemy_gate_left[frame]),
            Side::Right => (PAGE_WIDTH - 20, &self.gfx.enemy_gate_right[frame]),
        };
        for page in [Page::Page0, Page::Page1] {
            self.vga.draw_opaque(x, PAGE_HEIGHT / 2 - 10, page, picture);
        }
    }

    /// Counts down an enemy coming through a door, animating the door and adding the enemy
    /// when it gets to zero.
    fn update_enemy_entry(&mut self, side: Side) {
        let entry = match side {
            Side::Left => &mut self.entering_left,
            Side::Right => &mut self.entering_right,
        };
        if entry.countdown <= 0 {
            return;
        }

        entry.countdown -= 1;
        let (countdown, kind) = (entry.countdown, entry.kind.unwrap());

        if countdown & 7 == 0 {
            self.draw_enemy_door(side, ((countdown >> 3) % 5 + 1) as usize);
        }

        if countdown == 0 {
            let x = match side {
                Side::Left => 15,
                Side::Right => PAGE_WIDTH - self.gfx.enemy(kind)[0].width() - 16,
            };
            self.add_enemy(x, ENEMY_START_Y, kind);
            self.draw_enemy_door(side, 0);
            self.play(Sound::EnemyEnter);
        }
    }

    pub(super) fn move_enemies(&mut self) {
        let level = *self.level();
        let frequency = self.player_settings().difficulty.enemy_frequency();

        // Release new enemies through the doors.
        for kind in EnemyKind::ALL {
            if (self.enemies.len() as i32) < level.max_enemies
                && self.rng.unit() < level.enemy_release_prob * frequency
                && self.rng.below(100) < level.enemy_odds(kind)
            {
                let side = if self.rng.below(2) == 1 && self.entering_left.countdown <= 0 {
                    Some(Side::Left)
                } else if self.entering_right.countdown <= 0 {
                    Some(Side::Right)
                } else {
                    None
                };

                if let Some(side) = side {
                    let entry = EnemyEntry {
                        countdown: 80,
                        kind: Some(kind),
                    };
                    match side {
                        Side::Left => self.entering_left = entry,
                        Side::Right => self.entering_right = entry,
                    }
                    self.draw_enemy_door(side, 1);
                }
            }
        }

        self.update_enemy_entry(Side::Left);
        self.update_enemy_entry(Side::Right);

        for i in 0..self.enemies.len() {
            self.move_enemy(i);
        }

        // Power charges and explosions disappear after a while.
        for i in (0..self.enemies.len()).rev() {
            if self.enemies[i].lifetime > 0 {
                self.enemies[i].lifetime -= 1;
                if self.enemies[i].lifetime <= 0 {
                    self.enemy_hit(i, 0, 0, false);
                }
            }
        }
    }

    fn move_enemy(&mut self, i: usize) {
        let kind = self.enemies[i].kind;
        let info = kind.info();
        let (w, h) = (
            self.gfx.enemy(kind)[0].width(),
            self.gfx.enemy(kind)[0].height(),
        );

        if info.fires && self.rng.unit() < info.fire_prob {
            self.fire_enemy_missile(i);
        }
        if info.lays_mines && self.rng.unit() < info.fire_prob {
            self.add_enemy_mine(i);
        }

        let speed = self.game_speed;
        let (ship_x, ship_y) = (self.ship.body.x, self.ship.body.y);
        let ship_speed = wrap16(self.ship.body.dx.abs() + self.ship.body.dy.abs());

        let e = &mut self.enemies[i];
        e.frame += info.frame_speed;
        if e.frame >= (info.frames as i32) << 8 {
            e.frame = 0;
        }

        if info.follows && self.rng.unit() < info.follow_prob && !self.ship_destroyed {
            let e = &mut self.enemies[i];
            let dx = ship_x - e.body.x;
            let dy = ship_y - e.body.y;
            let distance = wrap16(dx.abs() + dy.abs()).max(1);
            e.body.dx = wrap16(dx * info.speed / distance * speed / 64);
            e.body.dy = wrap16(dy * info.speed / distance * speed / 64);
        } else if self.rng.unit() < info.change_dir_prob {
            if info.zooms && ship_speed < 60 {
                self.play(Sound::Bark);
                let e = &mut self.enemies[i];
                let dx = wrap16(ship_x - e.body.x);
                let dy = wrap16(ship_y - e.body.y);
                let distance = wrap16(dx.abs() + dy.abs()).max(1);
                // 16-bit arithmetic throughout, which overflows when the ship is far away.
                e.body.dx = wrap16(wrap16(wrap16(dx * info.speed) / distance) * speed) / 48;
                e.body.dy = wrap16(wrap16(wrap16(dy * info.speed) / distance) * speed) / 48;
            } else if !info.fixed_speed {
                let rx = self.rng.below(info.speed);
                let ry = self.rng.below(info.speed);
                let e = &mut self.enemies[i];
                e.body.dx = wrap16((rx - info.speed_offset) * speed / 64);
                e.body.dy = wrap16((ry - info.speed_offset) * speed / 64);
            } else {
                let cos = (self.rng.unit() * 2.0 * PI - PI).cos();
                let e = &mut self.enemies[i];
                e.body.dx = wrap16((info.speed as f64 * cos).round() as i32 * speed / 64);
                e.body.dy =
                    wrap16((info.speed as f64 * (1.0 - cos * cos)).round() as i32 * speed / 64);
            }
        }

        if info.curves {
            if self.rng.unit() < info.change_curve_prob {
                let sin = wrap16(self.rng.below(info.curve) - info.curve_offset);
                let e = &mut self.enemies[i];
                e.curve_sin = sin;
                e.curve_cos = curve_cos(sin);
            }

            let e = &mut self.enemies[i];
            let (dx, dy) = (e.body.dx as i64, e.body.dy as i64);
            let (cos, sin) = (e.curve_cos as i64, e.curve_sin as i64);
            e.body.dx = rotate_component(dx * cos - dy * sin);
            e.body.dy = rotate_component(dy * cos + dx * sin);
        }

        let b = &mut self.enemies[i].body;
        b.sx = wrap16(b.sx + b.dx);
        b.sy = wrap16(b.sy + b.dy);
        (b.old_x, b.old_y) = (b.x, b.y);
        (b.x, b.y) = (b.sx / 64, b.sy / 64);
        (b.xbr, b.ybr) = (b.x + w - 1, b.y + h - 1);

        // Bounce off the walls. The corner positions are not updated after clamping, as in
        // the original.
        if b.x < ENEMY_MIN_X {
            b.x = ENEMY_MIN_X;
            b.dx = b.dx.abs();
        }
        if b.xbr > ENEMY_MAX_X {
            b.x = ENEMY_MAX_X - w;
            b.dx = -b.dx.abs();
        }
        if b.y < ENEMY_MIN_Y {
            b.y = ENEMY_MIN_Y;
            b.dy = b.dy.abs();
        }
        if b.ybr > ENEMY_MAX_Y {
            b.y = ENEMY_MAX_Y - h;
            b.dy = -b.dy.abs();
        }

        // And off the doors at the sides.
        if b.y < PAGE_HEIGHT / 2 + 7 && b.ybr > PAGE_HEIGHT / 2 - 7 {
            if b.x < ENEMY_MIN_X + 7 {
                b.dx = b.dx.abs();
            }
            if b.xbr > ENEMY_MAX_X - 7 {
                b.dx = -b.dx.abs();
            }
        }
    }

    // ------------------------------------------------------------------------------------
    // Missiles.
    // ------------------------------------------------------------------------------------

    pub(super) fn move_missiles(&mut self) {
        let bounce = self.power_ups.active(PowerUpKind::Bounce);
        let (w, h) = (
            self.gfx.ship_missile.width(),
            self.gfx.ship_missile.height(),
        );

        for m in self.missiles.iter_mut() {
            move_fixed_point(&mut m.body, w, h);
            m.age += 1;
        }

        for i in (0..self.missiles.len()).rev() {
            let m = &mut self.missiles[i];
            let b = &mut m.body;

            let off_screen = if b.x < 10 {
                b.x = 11;
                Some(true)
            } else if b.xbr > PAGE_WIDTH - 10 {
                b.x = PAGE_WIDTH - 11;
                Some(true)
            } else if b.y < 10 {
                b.y = 11;
                Some(false)
            } else if b.ybr > PAGE_HEIGHT - 10 {
                b.y = PAGE_HEIGHT - 11;
                Some(false)
            } else {
                None
            };

            let remove = match off_screen {
                Some(_) if !bounce => true,
                Some(horizontal) => {
                    if horizontal {
                        b.dx = wrap16(-b.dx);
                    } else {
                        b.dy = wrap16(-b.dy);
                    }
                    false
                }
                None => m.age > MISSILE_LIFE,
            };

            if remove {
                let m = self.missiles.swap_remove(i);
                self.vga.draw_opaque(
                    m.body.old_x,
                    m.body.old_y,
                    Page::Visible,
                    &m.body.old_background,
                );
            }
        }

        for m in self.enemy_missiles.iter_mut() {
            let sprite = &self.gfx.enemy_missiles[m.kind.index()];
            move_fixed_point(&mut m.body, sprite.width(), sprite.height());
        }

        for i in (0..self.enemy_missiles.len()).rev() {
            let m = &mut self.enemy_missiles[i];
            let b = &mut m.body;
            let off_x = b.x < 10 || b.xbr > PAGE_WIDTH - 10;
            let off_y = b.y < 10 || b.ybr > PAGE_HEIGHT - 10;

            if !(off_x || off_y) {
                continue;
            }

            if m.kind.info().rebounds {
                if off_x {
                    b.dx = wrap16(-b.dx);
                } else {
                    b.dy = wrap16(-b.dy);
                }
                self.play(Sound::Boing);
            } else {
                let m = self.enemy_missiles.swap_remove(i);
                self.vga.draw_opaque(
                    m.body.old_x,
                    m.body.old_y,
                    Page::Visible,
                    &m.body.old_background,
                );
            }
        }
    }

    fn fire_missile(&mut self, dx: i32, dy: i32) {
        if self.missiles.len() >= MAX_MISSILES {
            return;
        }

        let (w, h) = (
            self.gfx.ship_missile.width(),
            self.gfx.ship_missile.height(),
        );
        let s = &self.ship;
        // Centred on the ship.
        let sx = wrap16(s.body.sx + (s.width << 5) - (w << 4));
        let sy = wrap16(s.body.sy + (s.height << 5) - (h << 4));
        let (x, y) = (fixed_to_pixel_unsigned(sx), fixed_to_pixel_unsigned(sy));
        let background = self.vga.capture(x, y, 4, h, Page::Hidden);

        let body = Body {
            x,
            y,
            xbr: x + w - 1,
            ybr: y + h - 1,
            sx,
            sy,
            dx,
            dy,
            old_x: x,
            old_y: y,
            old_background: background.clone(),
            background,
        };
        self.missiles.push(Missile { body, age: 0 });
    }

    /// Fires the ship's gun with the velocity `(dx, dy)`, plus whatever the power-ups add.
    fn shoot(&mut self, mut dx: i32, mut dy: i32) {
        if self.missiles.len() < MAX_MISSILES {
            self.play(Sound::Fire);
        }

        if self.power_ups.active(PowerUpKind::AimedFire) && !self.enemies.is_empty() {
            // Aim at where the nearest enemy will be.
            let ship = (self.ship.body.x, self.ship.body.y);
            let (nearest, distance_squared) = self
                .enemies
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let (ex, ey) = ((ship.0 - e.body.x) as i64, (ship.1 - e.body.y) as i64);
                    (i, ex * ex + ey * ey)
                })
                .fold(
                    (0, i32::MAX as i64),
                    |best, c| if c.1 < best.1 { c } else { best },
                );

            let distance = (distance_squared as f64).sqrt().round() as i32;
            let frames_to_reach = wrap16(distance / (AIMED_FIRE_SPEED / 64));
            let e = &self.enemies[nearest].body;
            let target_x = wrap16(e.x + wrap16(frames_to_reach * e.dx) / 64);
            let target_y = wrap16(e.y + wrap16(frames_to_reach * e.dy) / 64);
            let distance = distance.max(1);
            dx = wrap16(AIMED_FIRE_SPEED * (target_x - ship.0) / distance);
            dy = wrap16(AIMED_FIRE_SPEED * (target_y - ship.1) / distance);
        }

        let rear = self.power_ups.active(PowerUpKind::RearFire);

        self.fire_missile(dx, dy);
        if rear {
            self.fire_missile(wrap16(-dx), wrap16(-dy));
        }

        if self.power_ups.active(PowerUpKind::MultiFire) {
            // Two more shots 10 degrees either side.
            let spread = PI / 180.0 * 10.0;
            let r = ((dx as f64).powi(2) + (dy as f64).powi(2)).sqrt();
            let mut theta = if dx != 0 {
                (dy as f64 / dx as f64).atan()
            } else if dy > 0 {
                PI / 2.0
            } else {
                -PI / 2.0
            };
            if dx < 0 {
                theta += ROUGH_PI;
            }

            let shot = |angle: f64| {
                (
                    wrap16((r * angle.cos()).round() as i32),
                    wrap16((r * angle.sin()).round() as i32),
                )
            };
            let (a, b) = (shot(theta + spread), shot(theta - spread));
            self.fire_missile(a.0, a.1);
            self.fire_missile(b.0, b.1);

            if rear {
                self.fire_missile(wrap16(-a.0), wrap16(-a.1));
                self.fire_missile(wrap16(-b.0), wrap16(-b.1));
            }
        }
    }

    // ------------------------------------------------------------------------------------
    // The ship.
    // ------------------------------------------------------------------------------------

    /// Counts down the power-ups, flashing their icons when they are about to run out.
    fn update_power_ups(&mut self) {
        for kind in PowerUpKind::ALL {
            if !self.power_ups.active(kind) {
                continue;
            }

            let p = &mut self.power_ups[kind];
            p.remaining -= 1;
            let PowerUp { remaining, icon_x } = *p;

            if remaining < 198 && icon_x != 0 {
                if remaining % 22 == 0 {
                    self.vga.fill_rect(
                        icon_x,
                        5,
                        icon_x + 17,
                        20,
                        Page::Split,
                        level::STATUS_COLOR,
                    );
                } else if remaining % 22 == 11 {
                    self.vga
                        .draw_opaque(icon_x, 5, Page::Split, &self.gfx.powerups[kind.index()]);
                }
            }

            if remaining == 0 {
                self.show_power_ups();
            }
        }
    }

    fn walls_kill(&self) -> bool {
        !self.power_ups.active(PowerUpKind::Shield)
            && !self.player_settings().difficulty.walls_are_safe()
    }

    fn check_wall_collisions(&mut self) {
        let walls_kill = self.walls_kill();
        let shielded = self.power_ups.active(PowerUpKind::Shield);
        let mask = &self.gfx.ship[self.ship.direction].mask;
        let (w, h) = (self.ship.width, self.ship.height);
        let b = &mut self.ship.body;

        // Rows of the mask that hit a wall, given as a column mask or a range of rows.
        let hits_columns = |columns: i32| {
            mask.iter()
                .take(h as usize)
                .any(|&m| m & columns as u32 != 0)
        };
        let hits_rows = |rows: std::ops::RangeInclusive<i32>| {
            rows.into_iter()
                .any(|r| mask.get(r as usize).copied().unwrap_or(0) != 0)
        };

        if b.xbr >= SHIP_MAX_X - 10
            && hits_columns(shl32(1, 31 - (SHIP_MAX_X - b.x)).wrapping_sub(1))
        {
            b.x = SHIP_MAX_X - w;
            if walls_kill {
                self.ship_destroyed = true;
            } else {
                b.dx = -b.dx.abs();
            }
        }

        if b.x < SHIP_MIN_X && hits_columns(!shl32(1, 31 - (SHIP_MIN_X - b.x - 1)).wrapping_sub(1))
        {
            b.x = SHIP_MIN_X;
            if walls_kill {
                self.ship_destroyed = true;
            } else {
                b.dx = b.dx.abs();
            }
        }

        // The doors the enemies come through.
        if b.y < PAGE_HEIGHT / 2 + 7 && b.ybr > PAGE_HEIGHT / 2 - 7 {
            if b.x < SHIP_MIN_X + 6 {
                b.x = SHIP_MIN_X + 7;
                if walls_kill {
                    self.ship_destroyed = true;
                } else {
                    b.dx = b.dx.abs();
                }
            }
            if b.xbr > SHIP_MAX_X - 6 {
                b.x = SHIP_MAX_X - 7 - w;
                if walls_kill {
                    self.ship_destroyed = true;
                } else {
                    b.dx = -b.dx.abs();
                }
            }
        }

        // Scroll horizontally.
        let excess = b.x - (self.scroll_x + VIEW_WIDTH - SCROLL_BORDER_X);
        if excess > 0 && self.scroll_x < MAX_SCROLL_X {
            self.scroll_x = (self.scroll_x + excess / 20 + 1).min(MAX_SCROLL_X);
        }
        let excess = (self.scroll_x + SCROLL_BORDER_X) - b.x;
        if excess > 0 && self.scroll_x > 0 {
            self.scroll_x = (self.scroll_x - (excess / 20 + 1)).max(0);
        }

        if b.ybr > SHIP_MAX_Y && hits_rows(SHIP_MAX_Y - b.y + 1..=h - 1) {
            b.y = SHIP_MAX_Y - h;
            if walls_kill {
                self.ship_destroyed = true;
            } else {
                b.dy = -b.dy.abs();
            }
        }

        // The exit gate posts kill even on the easy levels.
        for (gate, sprite) in self
            .gates
            .iter()
            .zip([&self.gfx.gate_left, &self.gfx.gate_right])
        {
            if gate.x + gate.width > b.x
                && gate.x <= b.xbr
                && gate.height > b.y
                && sprites_collide_inclusive(mask, b.x, b.y, &sprite.mask, gate.x, 0)
            {
                b.y = gate.height;
                if shielded {
                    b.dy = b.dy.abs();
                } else {
                    self.ship_destroyed = true;
                }
            }
        }

        if b.y < SHIP_MIN_Y {
            if b.x <= self.gates[0].x || b.xbr >= self.gates[1].x + self.gates[1].width {
                if hits_rows(0..=SHIP_MIN_Y - b.y - 1) {
                    b.y = SHIP_MIN_Y;
                    if walls_kill {
                        self.ship_destroyed = true;
                    } else {
                        b.dy = b.dy.abs();
                    }
                }
            } else {
                // In the gateway: blocked by the barrier until the crystals are collected.
                if self.progress[self.player].crystals > 0 && hits_rows(0..=8 - b.y - 1) {
                    b.y = SHIP_MIN_Y;
                    if shielded {
                        b.dy = b.dy.abs();
                    } else {
                        self.ship_destroyed = true;
                    }
                }
                if b.y < 5 && !self.ship_destroyed {
                    self.level_finished = true;
                }
            }
        }

        // Scroll vertically.
        let excess = b.y - (self.scroll_y + VIEW_HEIGHT - SCROLL_BORDER_Y);
        if excess > 0 && self.scroll_y < MAX_SCROLL_Y {
            self.scroll_y = (self.scroll_y + excess / 20 + 1).min(MAX_SCROLL_Y);
        }
        let excess = (self.scroll_y + SCROLL_BORDER_Y) - b.y;
        if excess > 0 && self.scroll_y > 0 {
            self.scroll_y = (self.scroll_y - (excess / 20 + 1)).max(0);
        }
    }

    /// Pushes the ship away from a point at `(dx, dy)` from it (or pulls it if `strength` is
    /// negative), with a force falling off with the cube of the distance.
    fn push_ship(&mut self, dx: i64, dy: i64, strength: i64, sound: bool) {
        if dx == 0 && dy == 0 {
            return;
        }

        let longest = dx.abs().max(dy.abs());
        let shortest = dx.abs().min(dy.abs());
        // 32-bit arithmetic, as in the original.
        let modulus = ((dx * dx + dy * dy) * (longest + shortest / 2)) as i32 as i64;
        if modulus <= 0 {
            return;
        }

        if sound && modulus < 500_000 && self.frame_count & 31 == 0 {
            self.play(Sound::Repulse);
        }

        let b = &mut self.ship.body;
        b.dx = wrap16((b.dx as i64 + strength * ((8192 * dx) as i32 as i64 / modulus)) as i32);
        b.dy = wrap16((b.dy as i64 + strength * ((8192 * dy) as i32 as i64 / modulus)) as i32);
    }

    pub(super) fn move_ship(&mut self) {
        if self.demo_playing {
            self.demo_position += 1;
            if self.demo_position > self.demo.frames.len() {
                self.game_over = true;
                return;
            }
        } else if self.demo_recording {
            self.demo_position += 1;
        }

        let (mut dx, mut dy) = (self.ship.body.dx, self.ship.body.dy);
        self.input_movement(&mut dx, &mut dy, false, true);
        (self.ship.body.dx, self.ship.body.dy) = (dx, dy);

        if self.demo_recording {
            self.demo.frames.push(DemoFrame::new(dx, dy));
        } else if self.demo_playing {
            let f = self.demo_frame();
            (self.ship.body.dx, self.ship.body.dy) = (f.dx(), f.dy());
        }

        let b = &mut self.ship.body;
        let speed = wrap16(b.dx.abs() + b.dy.abs());
        if speed > MAX_SHIP_SPEED {
            b.dx = (b.dx as f64 / speed as f64 * MAX_SHIP_SPEED as f64).round() as i32;
            b.dy = (b.dy as f64 / speed as f64 * MAX_SHIP_SPEED as f64).round() as i32;
        }

        for i in 0..self.enemies.len() {
            if self.enemies[i].kind.info().repulses {
                let (ex, ey) = (self.enemies[i].body.x, self.enemies[i].body.y);
                let (dx, dy) = (
                    (self.ship.body.x - ex) as i64,
                    (self.ship.body.y - ey) as i64,
                );
                self.push_ship(dx, dy, 1, true);
            }
        }

        let p = self.progress[self.player];
        if p.rank > 0 {
            let (dx, dy) = (
                (self.ship.body.x - p.attractor.0) as i64,
                (self.ship.body.y - p.attractor.1) as i64,
            );
            self.push_ship(dx, dy, -1, false);
        }

        let (w, h) = (self.ship.width, self.ship.height);
        let b = &mut self.ship.body;
        b.sx = wrap16(b.sx + b.dx);
        b.sy = wrap16(b.sy + b.dy);
        (b.old_x, b.old_y) = (b.x, b.y);
        (b.x, b.y) = (b.sx / 64, b.sy / 64);
        (b.xbr, b.ybr) = (b.x + w - 1, b.y + h - 1);

        // Point the ship the way it is going.
        let mut theta = if b.dy != 0 {
            (b.dx as f64 / b.dy as f64).atan()
        } else if b.dx > 0 {
            PI / 2.0
        } else {
            -PI / 2.0
        };
        if b.dy < 0 {
            theta += ROUGH_PI;
        }
        if theta < 0.0 {
            theta += 2.0 * PI;
        }
        let pictures = self.gfx.ship.len();
        self.ship.direction = ((theta / (2.0 * PI) * pictures as f64).round() as usize) % pictures;

        self.check_wall_collisions();
        self.update_power_ups();
        self.handle_ship_buttons();
    }

    fn handle_ship_buttons(&mut self) {
        let pressed = self.buttons_pressed();
        let held = self.buttons_down();
        let p = *self.player_settings();

        let mut fire = pressed.contains(p.fire_button);
        let mut smart_bomb = pressed.contains(p.smart_bomb_button);

        if self.demo_playing {
            // Any click ends a demo.
            if fire || smart_bomb {
                self.game_over = true;
                fire = false;
                smart_bomb = false;
            }
        } else if self.keyboard.take_press(p.keys.smart_bomb) {
            smart_bomb = true;
        }
        // The original also lets the fire key fire during a demo.
        if self.keyboard.take_press(p.keys.fire) {
            fire = true;
        }

        let demo = if self.demo_playing {
            self.demo_frame()
        } else {
            DemoFrame::default()
        };
        let (dx, dy) = (
            wrap16(self.ship.body.dx << 1),
            wrap16(self.ship.body.dy << 1),
        );

        if fire || demo.fire_pressed() {
            if self.demo_recording {
                self.demo.frames.last_mut().unwrap().set_fire_pressed();
            }
            self.shoot(dx, dy);
        } else if self.power_ups.active(PowerUpKind::RapidFire)
            && self.frame_count & 3 == 0
            && (held.contains(p.fire_button) || demo.fire_held())
        {
            if self.demo_recording {
                self.demo.frames.last_mut().unwrap().set_fire_held();
            }
            self.shoot(dx, dy);
        }

        if smart_bomb || demo.smart_bomb_pressed() {
            if self.demo_recording {
                self.demo
                    .frames
                    .last_mut()
                    .unwrap()
                    .set_smart_bomb_pressed();
            }
            self.fire_smart_bomb();
        }
    }

    // ------------------------------------------------------------------------------------
    // Collisions.
    // ------------------------------------------------------------------------------------

    /// The ship has touched object `i`.
    fn object_hit(&mut self, i: usize) {
        let o = &self.objects[i];
        let (x, y, xbr, ybr, kind) = (o.x, o.y, o.xbr, o.ybr, o.kind);

        match kind {
            ObjectKind::Crystal => {
                self.add_to_score(200);
                self.play(Sound::GetCrystal);
                self.progress().crystals -= 1;
                self.show_crystals();

                if self.progress[self.player].crystals == 0 {
                    self.open_gate();
                }
            }
            ObjectKind::Mine => {
                self.ship_destroyed = true;
                self.add_enemy(x, y, EnemyKind::Explosion);
                self.play(Sound::Explosion);
            }
            ObjectKind::SmartBomb => {
                self.progress().smart_bombs += 1;
                self.show_smart_bombs();
                self.play(Sound::AllRight);
                self.progress().smart_bomb_pickups -= 1;
            }
        }

        let background = self.objects[i].background.clone();
        self.vga.draw_opaque(x, y, Page::Hidden, &background);
        self.redraw_enemy_mines(x, y, xbr, ybr, Page::Hidden);
        self.objects[i].hit = true;
    }

    /// Redraws the enemy mines overlapping a rectangle that has just been restored.
    pub(super) fn redraw_enemy_mines(&mut self, x: i32, y: i32, xbr: i32, ybr: i32, page: Page) {
        for m in self.enemy_mines.iter().rev() {
            if m.xbr >= x && m.ybr >= y && m.x <= xbr && m.y <= ybr {
                self.vga
                    .draw_transparent(m.x, m.y, page, &self.gfx.enemy_mine.bitmap);
            }
        }
    }

    fn give_power_up(&mut self, kind: PowerUpKind) -> bool {
        if self.power_ups.active(kind) && kind != PowerUpKind::Shield {
            return false;
        }

        let (min, random) = kind.duration();
        let extra = self.rng.below(random);
        self.power_ups[kind].remaining += extra + min;
        self.show_power_ups();
        true
    }

    /// Picks a random special for collecting a power charge.
    fn collect_power_charge(&mut self) {
        loop {
            let given = match self.rng.below(22) {
                0..=2 => self.give_power_up(PowerUpKind::RapidFire),
                3..=5 => self.give_power_up(PowerUpKind::MultiFire),
                6..=8 => self.give_power_up(PowerUpKind::HeavyFire),
                9..=11 => self.give_power_up(PowerUpKind::RearFire),
                12..=13 => self.give_power_up(PowerUpKind::AimedFire),
                14..=15 => self.give_power_up(PowerUpKind::Bounce),
                16..=17 if !self.no_mines => {
                    // Destroy all the mines. A shield stops the ship being destroyed too.
                    if !self.power_ups.active(PowerUpKind::Shield) {
                        self.power_ups[PowerUpKind::Shield].remaining = 1;
                    }
                    for j in 0..self.objects.len() {
                        if self.objects[j].kind == ObjectKind::Mine {
                            self.object_hit(j);
                        }
                    }
                    self.no_mines = true;
                    self.smart_bomb_flash = 11;
                    true
                }
                18 => self.give_power_up(PowerUpKind::Shield),
                19.. if self.gate_moving => {
                    // Stop the exit gate moving.
                    self.gate_moving = false;
                    for (gate, sprite) in self
                        .gates
                        .iter()
                        .zip([&self.gfx.gate_left, &self.gfx.gate_right])
                    {
                        for page in [Page::Visible, Page::Hidden] {
                            self.vga.draw_opaque(gate.x, 0, page, &sprite.bitmap);
                        }
                    }
                    true
                }
                _ => false,
            };

            if given {
                break;
            }
        }
    }

    pub(super) fn check_collisions(&mut self) {
        let ship_mask = self.gfx.ship[self.ship.direction].mask.clone();
        let ship = self.ship.body.clone();
        let overlaps = |r: (i32, i32, i32, i32)| {
            r.2 >= ship.x && r.3 >= ship.y && r.0 <= ship.xbr && r.1 <= ship.ybr
        };

        if !self.ship_destroyed {
            for i in (0..self.enemies.len()).rev() {
                let Some(e) = self.enemies.get(i) else {
                    continue;
                };
                if e.kind == EnemyKind::Explosion {
                    continue;
                }

                let b = &e.body;
                let mask = &self.gfx.enemy(e.kind)[e.frame_index()].mask;
                if !(overlaps((b.x, b.y, b.xbr, b.ybr))
                    && sprites_collide(&ship_mask, ship.x, ship.y, mask, b.x, b.y))
                {
                    continue;
                }

                if e.kind == EnemyKind::PowerCharge {
                    self.collect_power_charge();
                    self.enemy_hit(i, 0, 0, false);
                    self.play(Sound::OhYeah);
                } else if !self.ship_destroyed {
                    self.ship_destroyed = true;
                    self.enemy_hit(i, ship.dx, ship.dy, true);
                }
            }

            let mine = &self.gfx.enemy_mine.mask;
            if self.enemy_mines.iter().any(|m| {
                overlaps((m.x, m.y, m.xbr, m.ybr))
                    && sprites_collide(&ship_mask, ship.x, ship.y, mine, m.x, m.y)
            }) {
                self.ship_destroyed = true;
            }

            for i in (0..self.objects.len()).rev() {
                let o = &self.objects[i];
                let mask = &self.object_sprite(o.kind).mask;
                if overlaps((o.x, o.y, o.xbr, o.ybr))
                    && sprites_collide(&ship_mask, ship.x, ship.y, mask, o.x, o.y)
                {
                    self.object_hit(i);
                }
            }
        }

        // The ship's missiles against enemies. A missile carries on after a hit, and can
        // destroy more than one enemy.
        let missile_mask = self.gfx.ship_missile.mask.clone();
        for i in 0..self.missiles.len() {
            for j in (0..self.enemies.len()).rev() {
                let Some(e) = self.enemies.get(j) else {
                    continue;
                };
                if e.kind == EnemyKind::Explosion {
                    continue;
                }

                let m = &self.missiles[i].body;
                let b = &e.body;
                let mask = &self.gfx.enemy(e.kind)[e.frame_index()].mask;
                if m.xbr > b.x
                    && m.x < b.xbr
                    && m.ybr > b.y
                    && m.y < b.ybr
                    && sprites_collide(&missile_mask, m.x, m.y, mask, b.x, b.y)
                {
                    let (dx, dy) = (m.dx, m.dy);
                    self.enemy_hit(j, dx, dy, true);
                    self.missiles[i].age = 10000;
                }
            }
        }

        // Enemy missiles against the ship.
        for i in (0..self.enemy_missiles.len()).rev() {
            let m = &self.enemy_missiles[i];
            let b = &m.body;
            let mask = &self.gfx.enemy_missile(m.kind).mask;
            if overlaps((b.x, b.y, b.xbr, b.ybr))
                && sprites_collide(&ship_mask, ship.x, ship.y, mask, b.x, b.y)
            {
                self.ship_destroyed = true;
                let m = self.enemy_missiles.swap_remove(i);
                self.vga.draw_opaque(
                    m.body.old_x,
                    m.body.old_y,
                    Page::Visible,
                    &m.body.old_background,
                );
            }
        }
    }
}

/// The cosine for a curve sine, both times 32767.
fn curve_cos(sin: i32) -> i32 {
    ((1.0 - (sin as f64 / 32767.0).powi(2)).sqrt() * 32767.0).round() as i32
}

/// Scales a rotated velocity component back down by 32767, rounding away from zero.
fn rotate_component(v: i64) -> i32 {
    // 32-bit arithmetic, as in the original.
    let v = (v as i32) as i64;
    wrap16(if v > 0 {
        (v + 16384) / 32767
    } else {
        (v - 16384) / 32767
    } as i32)
}

/// Moves a missile by its velocity, with the position treated as unsigned 16-bit.
fn move_fixed_point(b: &mut Body, w: i32, h: i32) {
    (b.old_x, b.old_y) = (b.x, b.y);
    b.sx = wrap16(b.sx + b.dx);
    b.x = fixed_to_pixel_unsigned(b.sx);
    b.xbr = b.x + w;
    b.sy = wrap16(b.sy + b.dy);
    b.y = fixed_to_pixel_unsigned(b.sy);
    b.ybr = b.y + h;
}
