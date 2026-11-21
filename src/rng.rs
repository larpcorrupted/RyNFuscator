//! Deterministic pseudo-random generator.
//!
//! A fixed seed produces byte-identical output, which makes builds
//! reproducible and makes the test suite meaningful.

#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        // xor with a constant so a seed of 0 still behaves.
        Rng {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    /// xorshift64*
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as usize
    }

    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as i64
    }

    pub fn chance(&mut self, percent: u32) -> bool {
        (self.next_u64() % 100) < percent as u64
    }

    #[allow(dead_code)]
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// Seed derived from the source text, so the same input always protects the
/// same way unless an explicit seed is given.
pub fn seed_from_source(src: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in src.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h ^ (src.len() as u64).wrapping_mul(0x9E37_79B9)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let a: Vec<u64> = (0..8).map(|_| Rng::new(42).next_u64()).collect();
        let b: Vec<u64> = (0..8).map(|_| Rng::new(42).next_u64()).collect();
        assert_eq!(a, b);
    }

    #[test]
    fn different_seeds_differ() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn below_in_range() {
        let mut r = Rng::new(7);
        for _ in 0..1000 {
            assert!(r.below(10) < 10);
        }
    }

    #[test]
    fn range_inclusive() {
        let mut r = Rng::new(7);
        for _ in 0..1000 {
            let v = r.range(2, 5);
            assert!((2..=5).contains(&v));
        }
    }

    #[test]
    fn seed_from_source_is_stable() {
        assert_eq!(seed_from_source("print('hi')"), seed_from_source("print('hi')"));
        assert_ne!(seed_from_source("print('hi')"), seed_from_source("print('ho')"));
    }
}
