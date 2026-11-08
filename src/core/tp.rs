//! Emulation of the Turbo Pascal 7 runtime behaviour that the game logic depends on.
//!
//! The random number generator matches the one in the original executable exactly, which
//! keeps recorded demos in sync.

/// Turbo Pascal's linear congruential random number generator.
#[derive(Clone, Debug)]
pub struct TpRandom {
    pub seed: u32,
}

impl TpRandom {
    pub fn new(seed: u32) -> Self {
        Self { seed }
    }

    fn next(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(0x0808_8405).wrapping_add(1);
        self.seed
    }

    /// `Random(range)`: the high word of the new seed modulo `range`.
    pub fn int(&mut self, range: i32) -> i32 {
        let seed = self.next();
        let range = range as u16;

        if range == 0 {
            return 0;
        }

        ((seed >> 16) as u16 % range) as i32
    }

    /// `Random` without an argument: the new seed scaled to `[0, 1)`.
    pub fn real(&mut self) -> f64 {
        self.next() as f64 / 4_294_967_296.0
    }
}

/// Decode a Turbo Pascal 6-byte `Real`.
pub fn real48(b: &[u8]) -> f64 {
    let exponent = b[0] as i32;

    if exponent == 0 {
        return 0.0;
    }

    let mut mantissa: u64 = 0;
    for i in (1..6).rev() {
        mantissa = (mantissa << 8) | b[i] as u64;
    }

    let negative = mantissa & (1 << 39) != 0;
    mantissa &= (1 << 39) - 1;

    let value = (1.0 + mantissa as f64 / (1u64 << 39) as f64) * 2f64.powi(exponent - 129);

    if negative { -value } else { value }
}

/// Pascal `Round`: rounds half away from zero.
pub fn round(x: f64) -> i32 {
    x.round() as i32
}

/// Truncate to a 16 bit Pascal `integer`.
pub fn w16(x: i32) -> i32 {
    x as i16 as i32
}

/// Pascal `longint shl n` as compiled by Turbo Pascal: shifting by 32 or more gives zero.
pub fn shl32(value: i32, n: i32) -> i32 {
    if (0..32).contains(&n) {
        ((value as u32) << n) as i32
    } else {
        0
    }
}

/// Pascal `Str(i:width, s)`.
pub fn str_w(i: i64, width: usize) -> String {
    format!("{:>width$}", i, width = width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_sequence() {
        let mut r = TpRandom::new(0);
        assert_eq!(r.seed, 0);
        r.int(10);
        assert_eq!(r.seed, 1);
        r.int(10);
        assert_eq!(r.seed, 0x0808_8406);
    }

    #[test]
    fn real48_values() {
        // 1.0 is exponent 0x81 with zero mantissa.
        assert_eq!(real48(&[0x81, 0, 0, 0, 0, 0]), 1.0);
        assert_eq!(real48(&[0x81, 0, 0, 0, 0, 0x80]), -1.0);
        assert_eq!(real48(&[0x82, 0, 0, 0, 0, 0x40]), 3.0);
    }
}
