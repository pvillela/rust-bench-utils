//! Seeded nonparametric bootstrap for confidence intervals of arbitrary statistics.
//!
//! Provides [`percentile_ci`] and [`bca_ci`] (bias-corrected and accelerated), both operating on a
//! `&[f64]` sample and an arbitrary `statistic: Fn(&[f64]) -> f64`. Resampling is driven by a small,
//! deterministic [`SplitMix64`] generator seeded from an explicit `seed`, so a fixed seed yields a
//! fixed interval (reproducible CIs / stable tests).
//!
//! In this crate the sample is the reservoir of recorded batch means and the statistic is a robust
//! estimator (e.g. `median_rob` / `mean_rob`) recomputed on each resample. Because the batch means
//! are approximately i.i.d. (interleaved execution), resampling them is the statistically correct
//! nonparametric bootstrap for functionals of the batch-mean distribution.
//!
//! This is a `#[doc(hidden)]` support module, re-exported from [`crate::dev_support`].

use basic_stats::{core::AltHyp, normal::z_alpha, normal::z_to_p};

/// Default number of bootstrap resamples for bootstrap intervals.
pub const DEFAULT_BOOTSTRAP_RESAMPLES: usize = 1000;

/// Default seed for the reproducible bootstrap RNG.
pub const DEFAULT_BOOTSTRAP_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// Minimal, fast, deterministic PRNG (SplitMix64) used to drive bootstrap resampling.
///
/// SplitMix64 is a well-distributed 64-bit generator; it is more than adequate for selecting
/// resample indices and keeps the crate dependency-free of a heavier RNG.
#[derive(Debug, Clone)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// Creates a generator with the given seed.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Returns the next pseudo-random `u64`.
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Returns a pseudo-random index in `0..n` (Lemire-style, low bias).
    ///
    /// # Panics
    /// Panics if `n == 0`.
    #[inline]
    pub fn next_index(&mut self, n: usize) -> usize {
        assert!(n > 0, "next_index requires n > 0");
        // Multiply-high mapping onto `0..n` avoids the modulo bias of `% n` for our sample sizes.
        let m = (self.next_u64() as u128) * (n as u128);
        (m >> 64) as usize
    }
}

/// Standard normal CDF, `Phi(x)`.
#[inline]
fn phi(x: f64) -> f64 {
    z_to_p(x, AltHyp::Lt)
}

/// Standard normal inverse CDF, `Phi^{-1}(p)`, with `p` clamped to a safe open interval.
#[inline]
fn phi_inv(p: f64) -> f64 {
    const EPS: f64 = 1e-12;
    let p = p.clamp(EPS, 1.0 - EPS);
    // `z_alpha(alpha) = inverse_cdf(1 - alpha)`, so `Phi^{-1}(p) = z_alpha(1 - p)`.
    z_alpha(1.0 - p).expect("1 - p is in (0, 1) after clamping")
}

/// Extracts the `q`-quantile (`q` in `[0, 1]`) from an ascending-sorted slice by linear interpolation.
fn sorted_quantile(sorted: &[f64], q: f64) -> f64 {
    let n = sorted.len();
    assert!(n > 0, "sorted_quantile requires a non-empty slice");
    if n == 1 {
        return sorted[0];
    }
    let q = q.clamp(0.0, 1.0);
    let pos = q * (n as f64 - 1.0);
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let frac = pos - lo as f64;
        sorted[lo] * (1.0 - frac) + sorted[hi] * frac
    }
}

/// Generates `n_resamples` bootstrap replicates of `statistic` over `sample`, returned ascending-sorted.
fn bootstrap_replicates(
    sample: &[f64],
    statistic: &mut impl FnMut(&[f64]) -> f64,
    n_resamples: usize,
    seed: u64,
) -> Vec<f64> {
    let n = sample.len();
    assert!(n > 0, "bootstrap requires a non-empty sample");
    let mut rng = SplitMix64::new(seed);
    let mut scratch = vec![0.0_f64; n];
    let mut replicates = Vec::with_capacity(n_resamples);
    for _ in 0..n_resamples {
        for slot in scratch.iter_mut() {
            *slot = sample[rng.next_index(n)];
        }
        replicates.push(statistic(&scratch));
    }
    replicates.sort_by(f64::total_cmp);
    replicates
}

/// Percentile bootstrap confidence interval for `statistic`, at confidence level `1 - alpha`.
///
/// Returns `(low, high)`.
///
/// # Panics
/// Panics if `sample` is empty or `alpha` is not in `(0, 1)`.
pub fn percentile_ci(
    sample: &[f64],
    mut statistic: impl FnMut(&[f64]) -> f64,
    n_resamples: usize,
    alpha: f64,
    seed: u64,
) -> (f64, f64) {
    assert!(alpha > 0.0 && alpha < 1.0, "alpha must be in (0, 1)");
    let replicates = bootstrap_replicates(sample, &mut statistic, n_resamples, seed);
    let low = sorted_quantile(&replicates, alpha / 2.0);
    let high = sorted_quantile(&replicates, 1.0 - alpha / 2.0);
    (low, high)
}

/// A computed bootstrap distribution: the sorted replicates plus the bias-correction (`z0`) and
/// acceleration (`a`) terms. Everything here is **independent of the confidence level**, so it can
/// be computed once (the expensive step) and cached, then turned into an interval at any `alpha`
/// via [`BootDist::interval`].
///
/// `a == 0.0` corresponds to a BC (bias-corrected) distribution; a jackknife-estimated `a`
/// corresponds to a BCa distribution.
#[derive(Debug, Clone)]
pub struct BootDist {
    /// Bootstrap replicates of the statistic, ascending-sorted.
    pub replicates: Vec<f64>,
    /// Bias-correction term `z0`.
    pub z0: f64,
    /// Acceleration term `a` (`0.0` for BC).
    pub a: f64,
}

impl BootDist {
    /// Extracts the `(low, high)` interval endpoints at confidence level `1 - alpha`.
    ///
    /// Cheap: applies the BCa endpoint adjustment (which collapses to the BC formula when `a == 0`)
    /// and reads two quantiles from the cached replicates. Falls back to plain percentile endpoints
    /// when `z0`/`a` is degenerate.
    ///
    /// # Panics
    /// Panics if `alpha` is not in `(0, 1)`.
    pub fn interval(&self, alpha: f64) -> (f64, f64) {
        assert!(alpha > 0.0 && alpha < 1.0, "alpha must be in (0, 1)");
        let (z0, a) = (self.z0, self.a);
        if !z0.is_finite() || !a.is_finite() {
            let low = sorted_quantile(&self.replicates, alpha / 2.0);
            let high = sorted_quantile(&self.replicates, 1.0 - alpha / 2.0);
            return (low, high);
        }
        let adjust = |p: f64| -> f64 {
            let z = phi_inv(p);
            let denom = 1.0 - a * (z0 + z);
            phi(z0 + (z0 + z) / denom)
        };
        let alpha1 = adjust(alpha / 2.0);
        let alpha2 = adjust(1.0 - alpha / 2.0);
        let low = sorted_quantile(&self.replicates, alpha1);
        let high = sorted_quantile(&self.replicates, alpha2);
        (low, high)
    }
}

/// Sorted bootstrap replicates plus the bias-correction `z0` from the fraction below `theta_hat`.
fn replicates_and_z0(
    sample: &[f64],
    statistic: &mut impl FnMut(&[f64]) -> f64,
    n_resamples: usize,
    seed: u64,
) -> (Vec<f64>, f64) {
    let theta_hat = statistic(sample);
    let replicates = bootstrap_replicates(sample, statistic, n_resamples, seed);
    let n_below = replicates.iter().filter(|&&r| r < theta_hat).count();
    let prop_below = n_below as f64 / replicates.len() as f64;
    let z0 = phi_inv(prop_below);
    (replicates, z0)
}

/// Computes the BC (bias-corrected, **no** acceleration) bootstrap distribution for `statistic`.
///
/// Needs **no jackknife** — cost is `O(n_resamples)`, independent of sample size. It keeps the bias
/// correction (`z0`) but drops the acceleration, which for non-smooth statistics like the **median**
/// is estimated poorly by the jackknife anyway (leave-one-out barely moves a median), so BC ≈ BCa
/// there at a fraction of the cost.
///
/// # Panics
/// Panics if `sample` is empty.
pub fn bc_dist(
    sample: &[f64],
    mut statistic: impl FnMut(&[f64]) -> f64,
    n_resamples: usize,
    seed: u64,
) -> BootDist {
    let (replicates, z0) = replicates_and_z0(sample, &mut statistic, n_resamples, seed);
    BootDist {
        replicates,
        z0,
        a: 0.0,
    }
}

/// Computes the BCa (bias-corrected and accelerated) bootstrap distribution for `statistic`.
///
/// The acceleration `a` is estimated by jackknife (leave-one-out), so this additionally evaluates
/// `statistic` once per sample element.
///
/// # Panics
/// Panics if `sample` is empty.
pub fn bca_dist(
    sample: &[f64],
    mut statistic: impl FnMut(&[f64]) -> f64,
    n_resamples: usize,
    seed: u64,
) -> BootDist {
    let n = sample.len();
    let (replicates, z0) = replicates_and_z0(sample, &mut statistic, n_resamples, seed);

    // Acceleration `a` via the jackknife (leave-one-out) over the sample.
    let mut jack = Vec::with_capacity(n);
    let mut loo = Vec::with_capacity(n.saturating_sub(1));
    for i in 0..n {
        loo.clear();
        loo.extend(sample.iter().enumerate().filter(|&(j, _)| j != i).map(|(_, &v)| v));
        jack.push(statistic(&loo));
    }
    let jack_mean = jack.iter().sum::<f64>() / n as f64;
    let mut num = 0.0;
    let mut den = 0.0;
    for &j in &jack {
        let d = jack_mean - j;
        num += d * d * d;
        den += d * d;
    }
    let a = if den > 0.0 {
        num / (6.0 * den.powf(1.5))
    } else {
        0.0
    };

    BootDist { replicates, z0, a }
}

/// BCa (bias-corrected and accelerated) bootstrap confidence interval for `statistic`,
/// at confidence level `1 - alpha`. Convenience wrapper over [`bca_dist`] + [`BootDist::interval`].
///
/// # Panics
/// Panics if `sample` is empty or `alpha` is not in `(0, 1)`.
pub fn bca_ci(
    sample: &[f64],
    statistic: impl FnMut(&[f64]) -> f64,
    n_resamples: usize,
    alpha: f64,
    seed: u64,
) -> (f64, f64) {
    bca_dist(sample, statistic, n_resamples, seed).interval(alpha)
}

/// BC (bias-corrected, **no** acceleration) bootstrap confidence interval for `statistic`, at
/// confidence level `1 - alpha`. Convenience wrapper over [`bc_dist`] + [`BootDist::interval`].
///
/// # Panics
/// Panics if `sample` is empty or `alpha` is not in `(0, 1)`.
pub fn bc_ci(
    sample: &[f64],
    statistic: impl FnMut(&[f64]) -> f64,
    n_resamples: usize,
    alpha: f64,
    seed: u64,
) -> (f64, f64) {
    bc_dist(sample, statistic, n_resamples, seed).interval(alpha)
}

/// Percentile bootstrap CI for the ratio of means of two independent samples, `mean(a)/mean(b)`,
/// at confidence level `1 - alpha`.
///
/// Each replicate independently resamples `a` and `b` (with replacement) and forms the ratio of the
/// resample means. Returns a [`basic_stats::core::Ci`].
///
/// # Panics
/// Panics if either sample is empty or `alpha` is not in `(0, 1)`.
pub fn two_sample_ratio_means_ci(
    a: &[f64],
    b: &[f64],
    n_resamples: usize,
    alpha: f64,
    seed: u64,
) -> basic_stats::core::Ci {
    assert!(!a.is_empty() && !b.is_empty(), "both samples must be non-empty");
    assert!(alpha > 0.0 && alpha < 1.0, "alpha must be in (0, 1)");
    let mut rng = SplitMix64::new(seed);
    let na = a.len();
    let nb = b.len();
    let mut replicates = Vec::with_capacity(n_resamples);
    for _ in 0..n_resamples {
        let mut sum_a = 0.0;
        for _ in 0..na {
            sum_a += a[rng.next_index(na)];
        }
        let mut sum_b = 0.0;
        for _ in 0..nb {
            sum_b += b[rng.next_index(nb)];
        }
        let mean_a = sum_a / na as f64;
        let mean_b = sum_b / nb as f64;
        replicates.push(mean_a / mean_b);
    }
    replicates.sort_by(f64::total_cmp);
    let low = sorted_quantile(&replicates, alpha / 2.0);
    let high = sorted_quantile(&replicates, 1.0 - alpha / 2.0);
    basic_stats::core::Ci(low, high)
}

#[cfg(test)]
#[cfg(feature = "_test")]
mod test {
    use super::*;

    fn mean(s: &[f64]) -> f64 {
        s.iter().sum::<f64>() / s.len() as f64
    }

    #[test]
    fn test_splitmix_deterministic() {
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn test_next_index_in_range() {
        let mut rng = SplitMix64::new(1);
        for _ in 0..10_000 {
            let idx = rng.next_index(7);
            assert!(idx < 7);
        }
    }

    #[test]
    fn test_percentile_ci_reproducible_and_brackets_mean() {
        // Symmetric data: mean ~ 10; CI should bracket it and be reproducible for a fixed seed.
        let sample: Vec<f64> = (0..200).map(|i| 10.0 + ((i % 21) as f64 - 10.0) * 0.1).collect();
        let ci1 = percentile_ci(&sample, mean, 500, 0.05, 12345);
        let ci2 = percentile_ci(&sample, mean, 500, 0.05, 12345);
        assert_eq!(ci1, ci2, "fixed seed must give identical CI");
        assert!(ci1.0 < 10.0 && ci1.1 > 10.0, "CI should bracket the mean: {ci1:?}");
    }

    #[test]
    fn test_bca_ci_reproducible_and_brackets_mean() {
        let sample: Vec<f64> = (0..200).map(|i| 10.0 + ((i % 21) as f64 - 10.0) * 0.1).collect();
        let ci1 = bca_ci(&sample, mean, 500, 0.05, 12345);
        let ci2 = bca_ci(&sample, mean, 500, 0.05, 12345);
        assert_eq!(ci1, ci2, "fixed seed must give identical CI");
        assert!(ci1.0 < 10.0 && ci1.1 > 10.0, "CI should bracket the mean: {ci1:?}");
        assert!(ci1.0 < ci1.1);
    }

    #[test]
    fn test_bc_ci_reproducible_and_brackets_mean() {
        let sample: Vec<f64> = (0..200).map(|i| 10.0 + ((i % 21) as f64 - 10.0) * 0.1).collect();
        let ci1 = bc_ci(&sample, mean, 500, 0.05, 12345);
        let ci2 = bc_ci(&sample, mean, 500, 0.05, 12345);
        assert_eq!(ci1, ci2, "fixed seed must give identical CI");
        assert!(ci1.0 < 10.0 && ci1.1 > 10.0, "CI should bracket the mean: {ci1:?}");
        assert!(ci1.0 < ci1.1);
    }

    #[test]
    fn test_bootdist_interval() {
        let reps: Vec<f64> = (0..=1000).map(|i| i as f64 / 1000.0).collect();
        // z0 = 0, a = 0 ⇒ adjust(p) = p, so the interval is the plain percentile interval.
        let d = BootDist {
            replicates: reps.clone(),
            z0: 0.0,
            a: 0.0,
        };
        let (lo, hi) = d.interval(0.05);
        assert!((lo - 0.025).abs() < 1e-6, "lo={lo}");
        assert!((hi - 0.975).abs() < 1e-6, "hi={hi}");
        // Degenerate z0 ⇒ percentile fallback (same endpoints here).
        let deg = BootDist {
            replicates: reps,
            z0: f64::NAN,
            a: 0.0,
        };
        let (lo2, hi2) = deg.interval(0.05);
        assert!((lo2 - 0.025).abs() < 1e-6 && (hi2 - 0.975).abs() < 1e-6);
    }

    #[test]
    fn test_sorted_quantile() {
        let s = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(sorted_quantile(&s, 0.0), 1.0);
        assert_eq!(sorted_quantile(&s, 1.0), 5.0);
        assert_eq!(sorted_quantile(&s, 0.5), 3.0);
    }
}
