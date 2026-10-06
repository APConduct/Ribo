type State = [u64; 4];
/// xohiro256** state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RNG {
    s: State,
}

#[inline]
fn splitmix64(x: &mut u64) -> u64 {
    *x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl RNG {
    pub fn from_seed(seed: u64) -> Self {
        let mut x = seed;
        let s = [
            splitmix64(&mut x),
            splitmix64(&mut x),
            splitmix64(&mut x),
            splitmix64(&mut x),
        ];
        if s == [0, 0, 0, 0] {
            return RNG { s: [1, 2, 3, 4] };
        }
        RNG { s }
    }

    pub fn for_lineage(root_seed: u64, lineage: u64) -> Self {
        let mut x = root_seed ^ lineage.wrapping_mul(0xD185_4A32_D192_Ed03);
        RNG::from_seed(splitmix64(&mut x))
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] ^= self.s[3].rotate_right(45);
        result
    }

    /// Uniform in [0.0, 1.0). 24 bits of mantissa, no float bias games.
    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        ((self.next_u64() >> 40) as f32) * (1.0 / 16_777_216.0)
    }
    /// Uniform in [0, n). Returns 0 when n == 0.
    #[inline]
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as usize
    }
    /// Uniform in [lo, hi). Returns lo when the range is empty.
    #[inline]
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            return lo;
        }
        lo + self.below(hi - lo)
    }
    #[inline]
    pub fn chance(&mut self, p: f32) -> bool {
        self.next_f32() < p
    }
    /// Roughly normal, via the sum-of-uniforms trick. Good enough for drift.
    pub fn gaussian(&mut self) -> f32 {
        let a = self.next_f32();
        let b = self.next_f32();
        let c = self.next_f32();
        (a + b + c) * 2.0 - 3.0
    }
    /// Fork a child stream. Advances self.
    pub fn split(&mut self, tag: u64) -> Self {
        let n = self.next_u64();
        RNG::for_lineage(n, tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn same_seed_same_stream() {
        let mut a = RNG::from_seed(0xABCD);
        let mut b = RNG::from_seed(0xABCD);
        for _ in 0..64 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }
    #[test]
    fn lineage_streams_are_order_independent() {
        let a = RNG::for_lineage(99, 7);
        let b = RNG::for_lineage(99, 7);
        let c = RNG::for_lineage(99, 8);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
    #[test]
    fn floats_stay_in_unit_range() {
        let mut r = RNG::from_seed(5);
        for _ in 0..4096 {
            let f = r.next_f32();
            assert!((0.0..1.0).contains(&f));
        }
    }
}
