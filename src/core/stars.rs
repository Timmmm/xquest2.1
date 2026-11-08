//! The starfield behind the menus.

use crate::core::rng::Rng;
use crate::core::vga::{Page, Vga};

pub const MAX_STARS: usize = 400;
const SPEED: i32 = 128;
const XWIDTH: i32 = 320;
const YWIDTH: i32 = 240;
const XCENTER: i32 = XWIDTH / 2;
const YCENTER: i32 = YWIDTH / 2;

#[derive(Clone, Copy, Default)]
struct Star {
    x: i32,
    y: i32,
    z: i32,
    xz: i32,
    yz: i32,
    c: i32,
}

#[derive(Default)]
pub struct Stars {
    star: Vec<Star>,
    initialised: bool,
}

impl Stars {
    fn init_star(rng: &mut Rng) -> Star {
        let x = -5000 + rng.below(10000);
        let y = -5000 + rng.below(10000);
        let z = rng.below(12000) + 256;
        Star {
            x,
            y,
            z,
            xz: 1,
            yz: 1,
            c: 0,
        }
    }

    fn new_star(rng: &mut Rng) -> Star {
        let x = -8191 + rng.below(16383);
        let y = -8191 + rng.below(16383);
        let z = rng.below(1256) + 14500;
        Star {
            x,
            y,
            z,
            xz: 1,
            yz: 1,
            c: 0,
        }
    }

    pub fn start(&mut self, rng: &mut Rng) {
        if self.initialised {
            return;
        }
        self.star = (0..MAX_STARS).map(|_| Self::init_star(rng)).collect();
        self.initialised = true;
    }

    pub fn stop(&mut self) {
        self.initialised = false;
    }

    /// Moves every star one step towards the viewer.
    pub fn step(&mut self, vga: &mut Vga, rng: &mut Rng) {
        if !self.initialised {
            return;
        }

        for s in self.star.iter_mut() {
            if s.xz < 0 || s.yz < 0 || s.xz > XWIDTH || s.yz > YWIDTH {
                *s = Self::new_star(rng);
                continue;
            }

            if s.c != 0 {
                vga.set_pixel(s.xz, s.yz, Page::Visible, 0);
            }

            s.z = (s.z - SPEED) as i16 as i32;
            if s.z < 257 {
                *s = Self::new_star(rng);
                continue;
            }

            let hz = (s.z as u16 >> 8) as i32;
            s.xz = s.x / hz + XCENTER;
            s.yz = s.y / hz + YCENTER;

            if s.xz < 0 || s.yz < 0 || s.xz > XWIDTH || s.yz > YWIDTH {
                continue;
            }

            s.c = 31 - (s.z as u16 >> 9) as i32;

            // Stars only appear over black, judged on the first page.
            if vga.pixel(s.xz, s.yz, Page::Page0) != 0 {
                s.c = 0;
            } else {
                vga.set_pixel(s.xz, s.yz, Page::Visible, s.c as u8);
            }
        }
    }

    /// Forgets the stars inside a rectangle that is about to be cleared, so they are not
    /// erased over whatever is drawn there.
    pub fn forget(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        for s in self.star.iter_mut() {
            if s.xz > x1 && s.xz < x2 && s.yz > y1 && s.yz < y2 {
                s.c = 0;
            }
        }
    }
}
