//! The game's random number generator and 16-bit integer helpers.
//!
//! The generator is the linear congruential generator of the Turbo Pascal runtime that the
//! original was built with. Using the same one (and drawing numbers in the same order) is
//! what keeps recorded demos in sync.

#[derive(Clone, Debug)]
pub struct Rng {
    pub seed: u32,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self { seed }
    }

    fn next(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(0x0808_8405).wrapping_add(1);
        self.seed
    }

    /// A random integer in `0..range`. A range of zero (or one that does not fit in 16 bits
    /// unsigned) behaves as it did in the original.
    pub fn below(&mut self, range: i32) -> i32 {
        let seed = self.next();
        let range = range as u16;

        if range == 0 {
            return 0;
        }

        ((seed >> 16) as u16 % range) as i32
    }

    /// A random number in `0.0..1.0`.
    pub fn unit(&mut self) -> f64 {
        self.next() as f64 / 4_294_967_296.0
    }
}

/// Truncates to 16 bits. The original stores most values in 16-bit integers, and in a few
/// places the arithmetic overflows; this reproduces that.
pub fn wrap16(x: i32) -> i32 {
    x as i16 as i32
}

/// Shifts `value` left by `n` bits; shifting by 32 or more (or a negative amount) gives zero.
pub fn shl32(value: i32, n: i32) -> i32 {
    if (0..32).contains(&n) {
        ((value as u32) << n) as i32
    } else {
        0
    }
}

/// The pixel coordinate of a 10.6 fixed point position held in 16 bits, treated as unsigned
/// as the original's assembler does.
pub fn fixed_to_pixel_unsigned(fixed: i32) -> i32 {
    (fixed as u16 >> 6) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence() {
        let mut r = Rng::new(0);
        r.below(10);
        assert_eq!(r.seed, 1);
        r.below(10);
        assert_eq!(r.seed, 0x0808_8406);
    }
}
