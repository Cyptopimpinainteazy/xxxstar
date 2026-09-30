//! Deterministic randomness for the search layer.
//!
//! The engine's whole claim is that a reported failure can be reproduced from
//! a seed. That only holds if every draw in a run comes from one stream seeded
//! by that run's seed, so nothing here may touch `OsRng`, `SystemTime` or
//! `thread_rng`.
//!
//! Run `i` of a campaign uses [`derive_seed`] rather than a shared generator,
//! so run 900,000 can be replayed without replaying the 899,999 before it and
//! adding a worker does not renumber anything.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// SplitMix64. Used only to turn `(campaign_seed, run_index)` into a run seed;
/// the run itself is drawn from the ChaCha stream below.
fn split_mix_64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The seed for run `index` of a campaign.
///
/// Adjacent indices produce unrelated streams: a campaign that walked a single
/// generator would make `--runs 1000` and `--runs 10000` agree for the first
/// 1000 runs, and would make two workers overlap on the same scenarios.
pub fn derive_seed(campaign_seed: u64, index: u64) -> u64 {
    let mut state = campaign_seed ^ 0xD1B5_4A32_D192_ED03;
    let _ = split_mix_64(&mut state);
    state ^= index;
    split_mix_64(&mut state)
}

/// A seeded, counted random stream.
#[derive(Debug)]
pub struct McRng {
    inner: ChaCha8Rng,
    draws: u64,
}

impl McRng {
    pub fn from_seed(seed: u64) -> Self {
        Self {
            inner: ChaCha8Rng::seed_from_u64(seed),
            draws: 0,
        }
    }

    /// Values drawn so far. A cheap fingerprint of the sampling order.
    pub fn draws(&self) -> u64 {
        self.draws
    }

    pub fn next_u64(&mut self) -> u64 {
        self.draws += 1;
        self.inner.gen()
    }

    /// Uniform in `[0, 1)`. Built from 53 bits so every value is exactly
    /// representable and the mapping is reproducible.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in `[0, n)`. `n == 0` yields `0`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    /// Uniform in `[low, high)`. A reversed range is treated as degenerate
    /// rather than panicking mid-campaign.
    pub fn range(&mut self, low: f64, high: f64) -> f64 {
        if !(high > low) {
            return low;
        }
        low + self.unit() * (high - low)
    }

    /// One standard normal draw, Box-Muller. Deterministic and dependency-free.
    pub fn standard_normal(&mut self) -> f64 {
        // `1 - unit()` keeps the argument away from zero, where ln is infinite.
        let u1 = 1.0 - self.unit();
        let u2 = self.unit();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_stream() {
        let mut a = McRng::from_seed(99);
        let mut b = McRng::from_seed(99);
        let left: Vec<u64> = (0..16).map(|_| a.next_u64()).collect();
        let right: Vec<u64> = (0..16).map(|_| b.next_u64()).collect();
        assert_eq!(left, right);
    }

    #[test]
    fn different_run_indices_give_unrelated_seeds() {
        let seeds: Vec<u64> = (0..8).map(|i| derive_seed(1, i)).collect();
        let mut sorted = seeds.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), seeds.len(), "run seeds must not collide");
        // Adjacent indices must not produce adjacent seeds.
        assert!(derive_seed(1, 5).abs_diff(derive_seed(1, 6)) > 1_000_000);
    }

    #[test]
    fn a_run_seed_does_not_depend_on_how_many_runs_came_before_it() {
        assert_eq!(derive_seed(1234, 7), derive_seed(1234, 7));
        assert_ne!(derive_seed(1234, 7), derive_seed(1235, 7));
    }

    #[test]
    fn unit_stays_in_the_unit_interval() {
        let mut rng = McRng::from_seed(5);
        for _ in 0..10_000 {
            let value = rng.unit();
            assert!((0.0..1.0).contains(&value), "{value} out of range");
        }
    }

    #[test]
    fn a_reversed_range_is_degenerate_rather_than_a_panic() {
        let mut rng = McRng::from_seed(5);
        assert_eq!(rng.range(5.0, 1.0), 5.0);
        assert_eq!(rng.range(2.0, 2.0), 2.0);
    }

    #[test]
    fn the_normal_draw_has_a_sane_shape() {
        let mut rng = McRng::from_seed(11);
        let draws: Vec<f64> = (0..20_000).map(|_| rng.standard_normal()).collect();
        let mean = draws.iter().sum::<f64>() / draws.len() as f64;
        let variance = draws.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / draws.len() as f64;
        assert!(mean.abs() < 0.05, "mean {mean} is not near zero");
        assert!((variance - 1.0).abs() < 0.05, "variance {variance} is not near one");
    }
}
