//! Deterministic random numbers.
//!
//! The whole game is *reproducible*: give it the same seed and the same choices and you get the
//! exact same league, injuries, draft classes and championships. To guarantee that on every
//! computer (and in every future version of Rust) we ship our own tiny generator instead of
//! depending on an outside crate whose algorithm could change.
//!
//! The algorithm is xoshiro256** (public domain), seeded through SplitMix64.

use serde::{Deserialize, Serialize};

/// A fast, seedable, serializable random number generator.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rng {
    s: [u64; 4],
}

/// SplitMix64: turns one 64-bit number into a stream of well-mixed numbers. Used for seeding.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Hash a text label into a number (FNV-1a). Lets us create independent random "streams"
/// such as `rng.fork("draft-2031")` that don't disturb each other.
pub fn hash_label(label: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in label.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut sm = seed;
        let s = [
            splitmix64(&mut sm),
            splitmix64(&mut sm),
            splitmix64(&mut sm),
            splitmix64(&mut sm),
        ];
        Rng { s }
    }

    /// Build a generator from a seed plus a label (e.g. the year and a purpose).
    pub fn from_label(seed: u64, label: &str) -> Self {
        Rng::new(seed ^ hash_label(label).rotate_left(17))
    }

    /// Create an independent child generator. The parent advances by one step.
    pub fn fork(&mut self, label: &str) -> Rng {
        let base = self.next_u64();
        Rng::from_label(base, label)
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// Uniform float in [0, 1).
    #[inline]
    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// True with probability `p` (clamped to 0..=1).
    #[inline]
    pub fn chance(&mut self, p: f64) -> bool {
        self.f64() < p
    }

    /// Uniform integer in `lo..=hi` (inclusive).
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }

    pub fn range_usize(&mut self, n: usize) -> usize {
        if n <= 1 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    pub fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.f64()
    }

    /// Normal (bell curve) random number via the Box-Muller transform.
    pub fn gauss(&mut self, mean: f64, sd: f64) -> f64 {
        let u1 = self.f64().max(1e-12);
        let u2 = self.f64();
        let z = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
        mean + sd * z
    }

    /// Pick one element of a slice.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.range_usize(items.len())]
    }

    /// Choose an index with probability proportional to `weights` (negative values count as 0).
    pub fn weighted(&mut self, weights: &[f64]) -> usize {
        let total: f64 = weights.iter().map(|w| w.max(0.0)).sum();
        if total <= 0.0 {
            return self.range_usize(weights.len());
        }
        let mut x = self.f64() * total;
        for (i, w) in weights.iter().enumerate() {
            let w = w.max(0.0);
            if x < w {
                return i;
            }
            x -= w;
        }
        weights.len() - 1
    }

    /// Fisher-Yates shuffle.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.range_usize(i + 1);
            items.swap(i, j);
        }
    }

    /// Log-normal-ish heavy tail helper used for "rare but huge" outcomes.
    pub fn exp(&mut self, mean: f64) -> f64 {
        -mean * (1.0 - self.f64()).max(1e-12).ln()
    }
}

/// Convert a seed text typed by a human ("my-first-league") into a number.
pub fn seed_from_text(text: &str) -> u64 {
    if let Ok(n) = text.trim().parse::<u64>() {
        n
    } else {
        hash_label(text.trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_numbers() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn gauss_is_roughly_centered() {
        let mut r = Rng::new(7);
        let n = 20000;
        let mean: f64 = (0..n).map(|_| r.gauss(10.0, 2.0)).sum::<f64>() / n as f64;
        assert!((mean - 10.0).abs() < 0.1, "mean was {mean}");
    }

    #[test]
    fn weighted_respects_zero() {
        let mut r = Rng::new(1);
        for _ in 0..1000 {
            assert_ne!(r.weighted(&[0.0, 1.0, 0.0]), 0);
        }
    }
}
