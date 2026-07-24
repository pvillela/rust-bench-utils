//! Module defining the key data structure produced by [`crate::bench_run`].

use crate::{
    BenchCfg, FpSeconds, LatencyUnit, SummaryStats,
    dev_support::{
        BootDist, DEFAULT_BOOTSTRAP_SEED, SplitMix64, bc_ci, bc_dist, bca_ci, bca_dist, memoized_fn,
    },
    multi,
};
use basic_stats::{
    core::{AltHyp, Ci, HypTestResult, PositionWrtCi, SampleMoments, sample_mean, sample_stdev},
    normal::{student_1samp_ci, student_1samp_p, student_1samp_t, student_1samp_test},
};
use hdrhistogram::Histogram;
use std::{fmt::Debug, iter, ops::DerefMut, sync::Mutex};

/// Constructs a [`Histogram<u64>`]. The arguments correspond to [Histogram::high] and [Histogram::sigfig].
pub(crate) fn new_hdrhist(hist_high: u64, hist_sigfig: u8) -> Histogram<u64> {
    let mut hist = Histogram::<u64>::new_with_max(hist_high, hist_sigfig)
        .expect("should not happen given histogram construction");
    hist.auto(true);
    hist
}

struct CachedStats {
    rousseeuw_croux_q_ns: Option<FpSeconds>,
    rousseeuw_croux_q_ls: Option<f64>,
    s_hat: Option<f64>,
    /// Cached BC bootstrap distribution for the robust-median CI (alpha-independent).
    median_ci_dist: Option<BootDist>,
    /// Cached BCa bootstrap distribution for the robust-mean CI (alpha-independent).
    mean_ci_dist: Option<BootDist>,
}

impl CachedStats {
    fn rousseeuw_croux_q_ns(&mut self) -> &mut Option<FpSeconds> {
        &mut self.rousseeuw_croux_q_ns
    }

    fn rousseeuw_croux_q_ls(&mut self) -> &mut Option<f64> {
        &mut self.rousseeuw_croux_q_ls
    }

    fn s_hat(&mut self) -> &mut Option<f64> {
        &mut self.s_hat
    }

    fn median_ci_dist(&mut self) -> &mut Option<BootDist> {
        &mut self.median_ci_dist
    }

    fn mean_ci_dist(&mut self) -> &mut Option<BootDist> {
        &mut self.mean_ci_dist
    }
}

impl Default for CachedStats {
    fn default() -> Self {
        Self {
            rousseeuw_croux_q_ns: None,
            rousseeuw_croux_q_ls: None,
            s_hat: None,
            median_ci_dist: None,
            mean_ci_dist: None,
        }
    }
}

/// Severity of the naive-mean-vs-robust-mean disagreement reported by [`BenchOut::mean_check`].
///
/// The bands are keyed on the **standardized gap** `std_gap = rel_gap · √g / σ_Y` (where
/// `rel_gap = (naive_mean − mean_rob)/mean_rob`, `g` is the number of recorded batch means, and
/// `σ_Y = rc_q_ls` is their robust log-scale spread). Standardizing by `σ_Y/√g` — the scale of the
/// gap's clean-data sampling noise — makes a single set of cutoffs control false alarms across batch
/// sizes and sample sizes (empirically calibrated in
/// `analysis/Mean_check_threshold_calibration.md`). A non-positive gap (no right-tail contamination)
/// is always [`MeanVerdict::Insignificant`].
///
/// **High-dispersion caveat:** the standardization assumes `mean_rob` is (near-)unbiased, which
/// holds while the recorded batch means are near-normal. Under **extreme per-execution skew**
/// (very heavy-tailed latency, per-execution `σ ≳ 1.5`) `mean_rob` carries a genuine
/// log-normal-model bias; because that bias does *not* shrink with `√g`, standardizing amplifies it
/// and the verdict can over-flag even on clean data. This regime is not cleanly identifiable from
/// `σ_Y` alone (calibration data: a small `σ_Y` heavy-tail cell can be as unreliable as a large one),
/// so treat a non-`Insignificant` verdict on data you believe is clean as a hint you are in it.
/// [`MeanCheck::sigma_y`] is reported as a partial signal — a large `σ_Y` (typically unbatched /
/// heavy-tailed) means less reliability — and batching (which drives `σ_Y = σ/√k` down and pushes
/// the batch means toward normal) is the fix. See `analysis/Mean_check_threshold_calibration.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeanVerdict {
    /// Standardized gap below [`MEAN_CHECK_MILD_Z`]: naive mean is trustworthy.
    Insignificant,
    /// Standardized gap in `[MEAN_CHECK_MILD_Z, MEAN_CHECK_SIGNIFICANT_Z)`: possible contamination.
    Mild,
    /// Standardized gap at or above [`MEAN_CHECK_SIGNIFICANT_Z`]: the naive mean is likely contaminated.
    Significant,
}

/// Lower bound of the [`MeanVerdict::Mild`] band for the standardized mean-disagreement gap.
///
/// Calibrated so clean data flags `Mild` at most ~5% of the time across the near-normal operating
/// range (`σ_Y ≲ 0.1`, `σ ≤ 1`); see `analysis/Mean_check_threshold_calibration.md`.
pub const MEAN_CHECK_MILD_Z: f64 = 2.5;
/// Lower bound of the [`MeanVerdict::Significant`] band for the standardized mean-disagreement gap.
///
/// Calibrated so clean data flags `Significant` at most ~1% of the time across the near-normal
/// operating range; see `analysis/Mean_check_threshold_calibration.md`.
pub const MEAN_CHECK_SIGNIFICANT_Z: f64 = 4.0;

/// Default number of bootstrap resamples for the BC median CI ([`BenchOut::median_rob_ci`]).
///
/// Separate from [`DEFAULT_MEAN_CI_RESAMPLES`] so the median (BC) and mean (BCa) intervals can be
/// tuned independently; the median CI's BC method has no jackknife, so its cost is `O(resamples)`.
pub const DEFAULT_MEDIAN_CI_RESAMPLES: usize = 1000;

/// Default number of bootstrap resamples for the BCa mean CIs ([`BenchOut::mean_rob_ci`],
/// [`crate::Comp::ratio_means_boot_ci`]).
///
/// Separate from [`DEFAULT_MEDIAN_CI_RESAMPLES`]; the mean CI's BCa method additionally runs a
/// jackknife (one evaluation per reservoir sample) on top of these resamples.
pub const DEFAULT_MEAN_CI_RESAMPLES: usize = 2000;

impl MeanVerdict {
    /// Maps a signed standardized gap `std_gap = rel_gap · √g / σ_Y` to a verdict
    /// (`std_gap < MEAN_CHECK_MILD_Z` ⇒ [`MeanVerdict::Insignificant`], so non-positive gaps are
    /// always insignificant).
    pub fn from_std_gap(std_gap: f64) -> Self {
        if std_gap < MEAN_CHECK_MILD_Z {
            MeanVerdict::Insignificant
        } else if std_gap < MEAN_CHECK_SIGNIFICANT_Z {
            MeanVerdict::Mild
        } else {
            MeanVerdict::Significant
        }
    }
}

/// Result of [`BenchOut::mean_check`]: the naive and robust mean estimates, their relative gap, the
/// standardized gap, the robust log-scale spread, and a [`MeanVerdict`] classifying the disagreement.
#[derive(Debug, Clone, Copy)]
pub struct MeanCheck {
    /// Naive arithmetic mean of the recorded values.
    pub naive_mean: FpSeconds,
    /// Robust (batching-bias-corrected) mean estimate ([`BenchOut::mean_rob`]).
    pub mean_rob: FpSeconds,
    /// Signed relative gap `(naive_mean - mean_rob) / mean_rob`.
    pub rel_gap: f64,
    /// Standardized gap `rel_gap · √g / σ_Y` — the statistic the [`MeanVerdict`] bands key on.
    pub std_gap: f64,
    /// Robust log-scale spread of the recorded batch means, `σ_Y = rc_q_ls`. A partial signal for
    /// the high-dispersion regime where the verdict is less reliable (see [`MeanVerdict`]).
    pub sigma_y: f64,
    /// Verdict classifying the standardized gap.
    pub verdict: MeanVerdict,
}

/// Contains the latency observations resulting from benchmarking a closure.
///
/// It is returned by the core benchmarking functions in this library.
/// Its methods provide access to the raw data sample collected for the benchmarked closure, as well as descriptive and
/// inferential statistics.
///
/// In general, the quality of the statistics depends on batch size and the number of recorded values.
/// Some functions work best with no batching or smaller batch sizes, while others provide better results with higher
/// batch sizes.
/// The higher the number of recorded values, the better.
/// Some statistical functions need at least 5 recorded values to return reasonable values.
///
/// Inferential statistics are organized around the **mean**, the location parameter that batching
/// preserves in expectation: the parametric `student_mean_*` methods provide t-based CIs and tests
/// for `mean(latency(f))` directly on the recorded values (no log-normality assumption), valid under
/// batching because batch means are approximately normal (CLT). [`Self::mean_check`] guards the
/// naive mean against contamination by comparing it to the robust [`Self::mean_rob`]. For the
/// **median**, [`Self::median_rob_ci`] gives a batching-valid BCa bootstrap interval around the
/// batching-bias-corrected [`Self::median_rob`]; robust bootstrap intervals for the mean are
/// available via [`Self::mean_rob_ci`]. The `*_ln_*` accessors ([`Self::mean_ln_r`],
/// [`Self::stdev_ln_r`]) remain as descriptive statistics of `ln(latency(f))`.
pub struct BenchOut {
    pub(crate) recording_unit: LatencyUnit,
    pub(crate) hist: Histogram<u64>,
    pub(crate) sum: f64,
    pub(crate) sum2: f64,
    pub(crate) n_nz: u64,
    pub(crate) sum_ln: f64,
    pub(crate) sum2_ln: f64,
    pub(crate) batch: Option<usize>,
    /// Bounded reservoir of recorded observations (batch means, when batching), in seconds, used as
    /// the resample pool for the bootstrap confidence intervals. Reservoir-sampled once full.
    pub(crate) reservoir: Vec<f64>,
    /// Capacity of `reservoir` (from [`BenchCfg::reservoir_capacity`]).
    reservoir_capacity: usize,
    /// Total number of observations offered to the reservoir (for Algorithm R).
    n_seen: u64,
    /// Seeded RNG driving reservoir replacement, so retention is deterministic given the data.
    reservoir_rng: SplitMix64,
    cached_stats: Mutex<CachedStats>,
    /// Reusable scratch histogram for the Rousseeuw-Croux pairwise-difference computations, pooled
    /// so repeated `median_rob`/`mean_rob` calls (e.g. across bootstrap resamples) reuse one
    /// allocation instead of allocating a fresh histogram each time. Intentionally *not* cleared by
    /// [`Self::reset`], so it persists across the many resets a bootstrap performs.
    rc_scratch: Mutex<Option<Histogram<u64>>>,
}

impl BenchOut {
    /// Creates a new empty instance based on `cfg`.
    pub fn new(cfg: &BenchCfg, batch: Option<usize>) -> Self {
        let high_latency = FpSeconds::from_secs(10);
        let hist_high = cfg.recording_unit().value_from_fpsecs(high_latency);
        let hist = new_hdrhist(hist_high, cfg.sigfig());
        let sum = 0.;
        let sum2 = 0.;
        let n_nz = 0;
        let sum_ln = 0.;
        let sum2_ln = 0.;
        let batch = batch.map(|n| n.max(1));
        let reservoir_capacity = cfg.reservoir_capacity();
        let cached_stats = Mutex::new(CachedStats::default());

        Self {
            recording_unit: cfg.recording_unit(),
            hist,
            sum,
            sum2,
            n_nz,
            sum_ln,
            sum2_ln,
            batch,
            reservoir: Vec::with_capacity(reservoir_capacity),
            reservoir_capacity,
            n_seen: 0,
            reservoir_rng: SplitMix64::new(DEFAULT_BOOTSTRAP_SEED),
            cached_stats,
            rc_scratch: Mutex::new(None),
        }
    }

    /// Updates `self` from a **finite** iterator of [`FpSeconds`] values.
    ///
    /// Each item from the iterator is recorded as a single latency value.
    ///
    /// ## May hang
    /// Hangs if the iterator is not finite.
    pub fn record_from_iter(&mut self, src: impl Iterator<Item = FpSeconds>) {
        self.record_from_iter_with_counts(src.map(|item| (item, 1)))
    }

    /// Updates `self` from a **finite** iterator of ([`FpSeconds`], [`usize`]) pairs.
    ///
    /// Each item from the iterator is recorded as `count` latency observations, where `count` is the
    /// second component of the pair.
    ///
    /// ## May hang
    /// Hangs if the iterator is not finite.
    pub fn record_from_iter_with_counts(&mut self, src: impl Iterator<Item = (FpSeconds, usize)>) {
        for item in src {
            self.capture_data_with_counts(item);
        }
    }

    /// Resets `self` to an empty instance.
    pub(crate) fn reset(&mut self) {
        self.hist.reset();
        self.sum = 0.;
        self.sum2 = 0.;
        self.n_nz = 0;
        self.sum_ln = 0.;
        self.sum2_ln = 0.;
        self.reservoir.clear();
        self.n_seen = 0;
        self.reservoir_rng = SplitMix64::new(DEFAULT_BOOTSTRAP_SEED);
        self.cached_stats = Mutex::new(CachedStats::default());
    }

    /// Offers one observation (in seconds) to the bounded reservoir (Algorithm R).
    #[inline(always)]
    fn reservoir_offer(&mut self, value: f64) {
        if self.reservoir_capacity == 0 {
            return;
        }
        self.n_seen += 1;
        if self.reservoir.len() < self.reservoir_capacity {
            self.reservoir.push(value);
        } else {
            let j = self.reservoir_rng.next_index(self.n_seen as usize);
            if j < self.reservoir_capacity {
                self.reservoir[j] = value;
            }
        }
    }

    #[inline(always)]
    /// Updates `self` with an elapsed time observation for the target function.
    pub(crate) fn capture_data_with_counts(&mut self, latency_with_count: (FpSeconds, usize)) {
        let (mean_latency, count) = latency_with_count;
        let mean_elapsed_u64 = self.recording_unit.value_from_fpsecs(mean_latency);
        self.hist
            .record_n(mean_elapsed_u64, count as u64)
            .expect("can't happen: histogram is auto-resizable");

        // The pair represents `count` observations of value `mean_latency` (matching the
        // histogram's `record_n` above), so each sum accumulates `count` per-observation terms.
        let count_f64 = count as f64;
        let mean_elapsed_f64 = mean_latency.as_f64();
        self.sum += mean_elapsed_f64 * count_f64;
        self.sum2 += mean_elapsed_f64.powi(2) * count_f64;

        if mean_latency > FpSeconds::ZERO && count > 0 {
            let ln = mean_elapsed_f64.ln();
            self.n_nz += count as u64;
            self.sum_ln += ln * count_f64;
            self.sum2_ln += ln.powi(2) * count_f64;
        }

        // Offer each of the `count` observations to the bounded reservoir used for bootstrap CIs.
        // This runs after the timed region (the caller measures inside `src.next()`), so it cannot
        // perturb the sample it is deciding whether to keep.
        for _ in 0..count {
            self.reservoir_offer(mean_elapsed_f64);
        }
    }

    #[inline(always)]
    /// Updates `self` with an elapsed time observation for the target function.
    pub(crate) fn capture_data(&mut self, mean_latency: FpSeconds) {
        self.capture_data_with_counts((mean_latency, 1));
    }

    /// Returns all the latency data collected as an iterator of value-count pairs, where each value is a latency
    /// measurement and each count is the number of occurences of the latency measurment.
    ///
    /// The iterator yields values in strictly increasing order and all counts are positive.
    pub fn iter_with_counts(&self) -> impl Iterator<Item = (FpSeconds, usize)> {
        self.hist.iter_recorded().map(|x| {
            let value = self.recording_unit.fpsecs_from_value(x.value_iterated_to());
            let count = x.count_at_value();
            (value, count as usize)
        })
    }

    /// Returns all the latency data collected as an iterator of durations.
    ///
    /// The iterator yields values in monotonically non-decreasing order.
    pub fn iter(&self) -> impl Iterator<Item = FpSeconds> {
        self.iter_with_counts()
            .map(|(value, count)| iter::repeat_n(value, count))
            .flatten()
    }

    /// Latency unit used in data collection.
    pub fn recording_unit(&self) -> LatencyUnit {
        self.recording_unit
    }

    /// Batching used in data collection.
    ///
    /// - `None` means no batching;
    /// - `Some(b)` means batches of size `b`.
    #[inline(always)]
    pub fn batch(&self) -> Option<usize> {
        self.batch
    }

    /// Batch size used in data collection. Returns `1` for `batch` values of `None`, `Some(0)`, and `Some(1)`.
    #[inline(always)]
    pub fn bsz(&self) -> usize {
        self.batch.unwrap_or(1).max(1)
    }

    /// Number of recorded values. In case of batching, each group (batch) contributes one recorded value.
    #[inline(always)]
    pub fn n_r(&self) -> u64 {
        self.hist.len()
    }

    /// Total number of function executions accounting for batching (`= self.groups() * self.bsz()`).
    #[inline(always)]
    pub fn n(&self) -> u64 {
        self.n_r() * self.bsz() as u64
    }

    /// Summary descriptive statistics.
    ///
    /// # Panics
    /// Panics if the number of recorded values is zero.
    pub fn summary(&self) -> SummaryStats {
        SummaryStats::new(self)
    }

    /// Sample mean. Doesn't depend on batching.
    ///
    /// # Panics
    /// Panics if the number of recorded values is zero.
    pub fn mean(&self) -> FpSeconds {
        let mean = sample_mean(self.n_r(), self.sum).expect("number of recorded values is zero");
        mean.into()
    }

    /// Sample standard deviation of recorded values.
    ///
    /// # Panics
    /// Panics if the number of recorded values is zero.
    pub fn stdev_r(&self) -> FpSeconds {
        let stdev_r = sample_stdev(self.n_r(), self.sum, self.sum2)
            .expect("number of recorded values is zero");
        stdev_r.into()
    }

    /// Sample standard deviation accounting for batching.
    ///
    /// # Panics
    /// Panics if the number of recorded values is zero.
    pub fn stdev(&self) -> FpSeconds {
        self.stdev_r() * (self.bsz() as f64).sqrt()
    }

    /// Sample median of recorded values.
    ///
    /// Computes only the 0.5 quantile directly (a single histogram scan), rather than materializing
    /// a full [`SummaryStats`]; this matters because the robust estimators call `median_r` several
    /// times per evaluation (and many times over during a bootstrap).
    ///
    /// # Panics
    /// Panics if the number of recorded values is zero.
    pub fn median_r(&self) -> FpSeconds {
        assert!(!self.hist.is_empty(), "number of recorded values is zero");
        self.recording_unit
            .fpsecs_from_value(self.hist.value_at_quantile(0.50))
    }

    //=== Helper functions for estimators ===

    fn memoized<T: Clone>(
        &self,
        extract: impl FnOnce(&mut CachedStats) -> &mut Option<T>,
        f: impl FnOnce() -> T,
    ) -> T {
        let mut lock = self
            .cached_stats
            .lock()
            .expect("mutex shouldn't be poisoned");
        let cached = lock.deref_mut();
        memoized_fn(cached, extract, f)
    }

    #[allow(unused)]
    fn cv(&self) -> f64 {
        todo!()
    }

    #[allow(unused)]
    fn cv_rob(&self) -> f64 {
        todo!()
    }

    /// Consistency constant `d` normalises `Q` so that `Q/d` is a consistent estimator of sigma$ for a Gaussian population.
    fn rousseeuw_croux_d(&self) -> f64 {
        const C_Q: f64 = 2.2219;
        let g = self.n_r() as f64;

        C_Q * g
            / if self.n_r().is_multiple_of(2) {
                g + 3.8
            } else {
                g + 1.4
            }
    }

    fn rousseeuw_croux_q_general(
        &self,
        transf_in: impl Fn(u64) -> u64,
        transf_out: impl Fn(u64) -> f64,
    ) -> f64 {
        let mut rc_hist = self.take_rc_scratch();

        // Materialize the recorded entries once. Re-deriving `iter_recorded()` inside the
        // pairwise loop below (i.e. `self.hist.iter_recorded().skip(i + 1)`) would rebuild the
        // whole histogram iterator from scratch on every outer step; since that iterator walks
        // the underlying bucket array -- not just the recorded entries -- up to the position of
        // the highest recorded value, this turned an intended O(m^2) pairwise comparison (m =
        // number of distinct recorded values) into something closer to O(m * bucket_depth),
        // which dominates runtime for large samples.
        let entries: Vec<(u64, u64, u64)> = self
            .hist
            .iter_recorded()
            .filter_map(|iv| {
                let v = iv.value_iterated_to();
                if v == 0 {
                    return None;
                }
                let mid = transf_in(self.hist.median_equivalent(v));
                Some((mid, iv.count_at_value(), v))
            })
            .collect();

        for (i, &(mid_i, count_i, v_i)) in entries.iter().enumerate() {
            if count_i > 1 {
                // Within-bin pairs: C(count_i, 2) pairs, not count_i - 1. Their true
                // (pre-quantization) difference is unknown but bounded by the bucket's
                // width -- treating it as exactly 0 (a literal reading of the Rousseeuw-Croux
                // histogram recipe) is only safe when bucket width is negligible relative to
                // genuine dispersion. For real latency data with a tight mode, a single bucket
                // can hold enough of the sample that this zero-mass alone reaches the target
                // rank, collapsing Q_n to a misleading exact zero. Approximate instead by
                // E[|U1-U2|] for two points i.i.d. uniform on the bucket's width, i.e. width/3.
                // The width is computed in the transformed space (via `transf_in` on both
                // bucket edges) so this is correct for the nonlinear log-space transform too.
                let lo = transf_in(self.hist.lowest_equivalent(v_i));
                let hi = transf_in(self.hist.next_non_equivalent(v_i));
                let within_bucket_diff = hi.saturating_sub(lo) / 3;
                let n_within_bucket_pairs = count_i * (count_i - 1) / 2;
                rc_hist
                    .record_n(within_bucket_diff, n_within_bucket_pairs)
                    .expect("shouldn't happen as histogram is sized properly");
            }
            for &(mid_j, count_j, _) in &entries[i + 1..] {
                let abs_diff = mid_i.abs_diff(mid_j);
                let count = count_i * count_j;
                rc_hist
                    .record_n(abs_diff, count)
                    .expect("shouldn't happen as histogram is sized properly");
            }
        }
        // Q_n is the r-th smallest of the C(g, 2) pairwise absolute differences, with
        // h = floor(g/2) + 1 and r = C(h, 2) (Rousseeuw-Croux); this is the rank that
        // `rousseeuw_croux_d` is calibrated against, so look it up by exact rank rather
        // than by an arbitrary quantile.
        let g = self.n_r();
        let h = g / 2 + 1;
        let r = h * (h - 1) / 2;
        let total_pairs = g * (g - 1) / 2;
        let rank_quantile = r as f64 / total_pairs as f64;
        // `value_at_quantile` returns the *top* of the bucket holding the target rank
        // (`highest_equivalent`); read out the bucket midpoint instead, consistent with the
        // `median_equivalent` convention used on the inputs above. (No-op for values in the
        // histogram's unit-resolution range, where buckets are exact.)
        let rth_smallest = rc_hist.median_equivalent(rc_hist.value_at_quantile(rank_quantile));
        self.put_rc_scratch(rc_hist);
        transf_out(rth_smallest) * self.rousseeuw_croux_d()
    }

    #[allow(unused)]
    /// Returns the Rousseeuw-Croux Q statistic computed in natural space.
    fn pure_rousseeuw_croux_q_ns(&self) -> FpSeconds {
        let transf_in = |value: u64| -> u64 { value };
        let transf_out =
            |value: u64| -> f64 { value as f64 * self.recording_unit.factor_to_secs() };

        self.rousseeuw_croux_q_general(transf_in, transf_out).into()
    }

    #[allow(unused)]
    /// Returns the Rousseeuw-Croux Q statistic computed in log space.
    fn pure_rousseeuw_croux_q_ls(&self) -> f64 {
        let max = self.hist.max() as f64;
        let multiplyer = max / max.ln();

        // Unlike `pure_rousseeuw_croux_q_ns`, there is o need to adjust for `self.recording_unit()` because
        // the difference of the logs of scaled values is the same as the difference of the logs of the
        // non-scaled values.
        let transf_in = |value: u64| -> u64 { ((value as f64).ln() * multiplyer).round() as u64 };
        let transf_out = |value: u64| -> f64 { (value as f64) / multiplyer };

        self.rousseeuw_croux_q_general(transf_in, transf_out)
    }

    #[doc(hidden)]
    /// Returns the Rousseeuw-Croux Q statistic in natural space.
    pub fn rousseeuw_croux_q_ns(&self) -> FpSeconds {
        self.memoized(CachedStats::rousseeuw_croux_q_ns, || {
            self.pure_rousseeuw_croux_q_ns()
        })
    }

    #[doc(hidden)]
    /// Returns the Rousseeuw-Croux Q statistic in log space.
    pub fn rousseeuw_croux_q_ls(&self) -> f64 {
        self.memoized(CachedStats::rousseeuw_croux_q_ls, || {
            self.pure_rousseeuw_croux_q_ls()
        })
    }

    //=== Estimators of population mean ===

    #[doc(hidden)]
    /// Log-space, batching-bias-corrected estimator of the population mean, `exp(M + sigma_y^2 / 2)`,
    /// where `M = median(ln Y)` and `sigma_y` is the Rousseeuw-Croux Q statistic in log space.
    ///
    /// Unlike the median estimators, this needs no explicit `k` term: the Fenton-Wilkinson
    /// correction folded into `sigma_y` already accounts for batching, so the formula is valid,
    /// unmodified, for every batch size.
    pub fn mean_log_space_estimator(&self) -> FpSeconds {
        let m_y_ln = self.median_r().ln();
        let sigma_y = self.rousseeuw_croux_q_ls();
        (m_y_ln + sigma_y.powi(2) / 2.0).exp().into()
    }

    /// Batching-bias-corrected robust estimator of the population mean.
    ///
    /// This is [`Self::mean_log_space_estimator`], which is the robust primary estimator of the
    /// mean for every batch size and every degree of skew, under the assumption -- the default
    /// posture for latency benchmarking -- that the data may be contaminated by outliers (e.g.
    /// GC pauses, OS interrupts). Unlike [`Self::median_rob`], this estimator does not switch
    /// between alternatives based on the dispersion of the recorded values.
    pub fn mean_rob(&self) -> FpSeconds {
        self.mean_log_space_estimator()
    }

    //=== Estimators of population median ===

    #[doc(hidden)]
    pub fn median_log_space_estimator(&self) -> FpSeconds {
        let k = self.bsz() as f64;
        let m_y_ln = self.median_r().ln();
        let sigma_y = self.rousseeuw_croux_q_ls();
        let sigma2_x = (1.0 + k * (sigma_y.powi(2).exp() - 1.0)).ln();
        (m_y_ln + (sigma_y.powi(2) - sigma2_x) / 2.0).exp().into()
    }

    #[doc(hidden)]
    pub fn median_rmom_estimator(&self) -> FpSeconds {
        let k = self.bsz() as f64;
        let nu_y = self.median_r();
        let tau_y = self.rousseeuw_croux_q_ns();
        (nu_y / (1.0 + k * tau_y.powi(2) / nu_y.powi(2)).sqrt()).into()
    }

    fn pure_s_hat(&self) -> f64 {
        const INV_PHI_0_75: f64 = 0.6745; // Inverse normal CDF at 0.75.
        // `m` is `M = median(ln Y)`, i.e. the log of the median expressed in recording units --
        // the same log space as `mid.ln()` below (log of a raw recording-unit integer). Using
        // the natural-scale median in seconds here (un-logged) would put `m` many orders of
        // magnitude away from `mid.ln()`, making every `abs_diff` below dominated by `m` alone.
        let m = (self.recording_unit.value_from_fpsecs(self.median_r()) as f64).ln();
        let max = self.hist.max() as f64;
        if max.ln() == m {
            return 0.0;
        }
        let multiplyer = max / (max.ln() - m);

        let mut rc_hist = self.take_rc_scratch();
        for iv in self.hist.iter_recorded() {
            let v = iv.value_iterated_to();
            if v == 0 {
                continue;
            }
            let mid = self.hist.median_equivalent(v) as f64;
            let abs_diff = (mid.ln() - m).abs();
            let abs_diff_x = (abs_diff * multiplyer).round() as u64;
            let count: u64 = iv.count_at_value();
            rc_hist
                .record_n(abs_diff_x, count)
                .expect("shouldn't happen as histogram is sized properly");
        }
        let median_abs_diff_x = rc_hist.value_at_quantile(0.5);
        self.put_rc_scratch(rc_hist);
        let median_abs_diff = median_abs_diff_x as f64 / multiplyer;
        median_abs_diff / INV_PHI_0_75
    }

    fn s_hat(&self) -> f64 {
        self.memoized(CachedStats::s_hat, || self.pure_s_hat())
    }

    pub fn median_rob(&self) -> FpSeconds {
        let s_hat = self.s_hat();
        match self.bsz() {
            1 => self.median_r(),
            _ if s_hat <= 0.3 => self.median_rmom_estimator(),
            _ if 0.3 < s_hat && s_hat <= 0.6 => self.median_log_space_estimator(),
            _ if s_hat > 0.6 => self.median_rmom_estimator(),
            _ => self.median_r(),
        }
    }

    //=== Bootstrap confidence intervals ===

    #[doc(hidden)]
    /// Bounded reservoir of recorded observations (in seconds) used as the bootstrap resample pool.
    pub fn reservoir(&self) -> &[f64] {
        &self.reservoir
    }

    /// Builds a single reusable scratch [`BenchOut`] for bootstrap resampling.
    ///
    /// It shares this instance's `sigfig` and batch size, but its histogram is sized only to the
    /// reservoir's observed maximum (not the full 10 s range), so it — and the Rousseeuw-Croux
    /// scratch histograms derived from it inside `median_rob`/`mean_rob` — stay small. The scratch
    /// is reset and refilled per resample via [`Self::record_reset`], so the whole bootstrap
    /// allocates its histograms once rather than once per resample.
    fn bootstrap_scratch(&self) -> BenchOut {
        let max_secs = self.reservoir.iter().copied().fold(0.0_f64, f64::max);
        let high = self
            .recording_unit
            .value_from_fpsecs(FpSeconds(max_secs))
            .max(1);
        BenchOut {
            recording_unit: self.recording_unit,
            hist: new_hdrhist(high, self.hist.sigfig()),
            sum: 0.,
            sum2: 0.,
            n_nz: 0,
            sum_ln: 0.,
            sum2_ln: 0.,
            batch: self.batch,
            reservoir: Vec::new(),
            reservoir_capacity: 0,
            n_seen: 0,
            reservoir_rng: SplitMix64::new(DEFAULT_BOOTSTRAP_SEED),
            cached_stats: Mutex::new(CachedStats::default()),
            rc_scratch: Mutex::new(None),
        }
    }

    /// Borrows the pooled Rousseeuw-Croux scratch histogram (creating it on first use, from this
    /// instance's histogram geometry), reset to empty and ready to record into.
    fn take_rc_scratch(&self) -> Histogram<u64> {
        let mut lock = self.rc_scratch.lock().expect("mutex shouldn't be poisoned");
        let mut h = lock.take().unwrap_or_else(|| Histogram::new_from(&self.hist));
        h.reset();
        h
    }

    /// Returns the pooled scratch histogram after use.
    fn put_rc_scratch(&self, h: Histogram<u64>) {
        *self.rc_scratch.lock().expect("mutex shouldn't be poisoned") = Some(h);
    }

    /// Clears this instance and records `samples` (in seconds), reusing the existing histogram
    /// allocation. Used to refill a bootstrap scratch between resamples.
    fn record_reset(&mut self, samples: &[f64]) {
        self.reset();
        for &v in samples {
            self.capture_data(FpSeconds::from(v));
        }
    }

    /// BC (bias-corrected) bootstrap confidence interval for the batching-bias-corrected robust
    /// median ([`Self::median_rob`]), at confidence level `1 - alpha`.
    ///
    /// The interval is computed by resampling the recorded observations (batch means, when batching)
    /// held in the bounded reservoir and recomputing `median_rob` on each resample. Unlike the
    /// parametric mean interval, this makes no log-normality assumption and is valid under batching.
    ///
    /// Uses **BC** rather than BCa: for a median the jackknife acceleration is weak and unstable
    /// (leave-one-out barely moves a median), so BC gives essentially the same coverage as BCa
    /// while its cost is `O(n_resamples)`, independent of reservoir size. (The robust-*mean*
    /// interval [`Self::mean_rob_ci`], a smoother statistic, keeps full BCa.)
    ///
    /// The interval is centered on `median_rob` computed on the reservoir sample, which may differ
    /// negligibly from [`Self::median_rob`] (computed on the full histogram) when the reservoir is a
    /// strict subsample.
    ///
    /// # Panics
    /// Panics if the reservoir is empty (no recorded observations) or `alpha` is not in `(0, 1)`.
    pub fn median_rob_ci(&self, alpha: f64) -> (FpSeconds, FpSeconds) {
        let (low, high) = self.median_ci_dist().interval(alpha);
        (low.into(), high.into())
    }

    /// Memoized (alpha-independent) BC bootstrap distribution backing [`Self::median_rob_ci`].
    ///
    /// Computed once at the default resample count/seed and cached in `cached_stats`, so repeated
    /// `median_rob_ci` calls (e.g. at different confidence levels) only re-extract endpoints.
    fn median_ci_dist(&self) -> BootDist {
        self.memoized(CachedStats::median_ci_dist, || {
            let mut scratch = self.bootstrap_scratch();
            let stat = |samples: &[f64]| {
                scratch.record_reset(samples);
                scratch.median_rob().as_f64()
            };
            bc_dist(
                &self.reservoir,
                stat,
                DEFAULT_MEDIAN_CI_RESAMPLES,
                DEFAULT_BOOTSTRAP_SEED,
            )
        })
    }

    #[doc(hidden)]
    /// [`Self::median_rob_ci`] with explicit resample count and seed (unmemoized; used by tests).
    pub fn median_rob_ci_with(
        &self,
        alpha: f64,
        n_resamples: usize,
        seed: u64,
    ) -> (FpSeconds, FpSeconds) {
        let mut scratch = self.bootstrap_scratch();
        let stat = |samples: &[f64]| {
            scratch.record_reset(samples);
            scratch.median_rob().as_f64()
        };
        let (low, high) = bc_ci(&self.reservoir, stat, n_resamples, alpha, seed);
        (low.into(), high.into())
    }

    /// BCa bootstrap confidence interval for the batching-bias-corrected robust mean
    /// ([`Self::mean_rob`]), at confidence level `1 - alpha`.
    ///
    /// This is the robust, distribution-free companion to the parametric mean interval
    /// ([`Self::student_mean_ci`]); prefer it when [`Self::mean_check`] flags contamination.
    ///
    /// # Panics
    /// Panics if the reservoir is empty (no recorded observations) or `alpha` is not in `(0, 1)`.
    pub fn mean_rob_ci(&self, alpha: f64) -> (FpSeconds, FpSeconds) {
        let (low, high) = self.mean_ci_dist().interval(alpha);
        (low.into(), high.into())
    }

    /// Memoized (alpha-independent) BCa bootstrap distribution backing [`Self::mean_rob_ci`].
    ///
    /// Computed once at the default resample count/seed and cached in `cached_stats`, so repeated
    /// `mean_rob_ci` calls only re-extract endpoints (the BCa jackknife runs at most once).
    fn mean_ci_dist(&self) -> BootDist {
        self.memoized(CachedStats::mean_ci_dist, || {
            let mut scratch = self.bootstrap_scratch();
            let stat = |samples: &[f64]| {
                scratch.record_reset(samples);
                scratch.mean_rob().as_f64()
            };
            bca_dist(
                &self.reservoir,
                stat,
                DEFAULT_MEAN_CI_RESAMPLES,
                DEFAULT_BOOTSTRAP_SEED,
            )
        })
    }

    #[doc(hidden)]
    /// [`Self::mean_rob_ci`] with explicit resample count and seed (unmemoized; used by tests).
    pub fn mean_rob_ci_with(
        &self,
        alpha: f64,
        n_resamples: usize,
        seed: u64,
    ) -> (FpSeconds, FpSeconds) {
        let mut scratch = self.bootstrap_scratch();
        let stat = |samples: &[f64]| {
            scratch.record_reset(samples);
            scratch.mean_rob().as_f64()
        };
        let (low, high) = bca_ci(&self.reservoir, stat, n_resamples, alpha, seed);
        (low.into(), high.into())
    }

    /// Sample mean of the natural logarithms of recorded [`FpSeconds`] values.
    ///
    /// # Panics
    /// Panics if the number of non-zero observations is zero.
    pub fn mean_ln_r(&self) -> f64 {
        sample_mean(self.n_nz, self.sum_ln).expect("number of non-zero observations is zero")
    }

    /// Sample standard deviation of the natural logarithms of recorded [`FpSeconds`] values.
    ///
    /// # Panics
    /// Panics if the number of non-zero recorded values is zero.
    pub fn stdev_ln_r(&self) -> f64 {
        sample_stdev(self.n_nz, self.sum_ln, self.sum2_ln)
            .expect("number of non-zero observations is zero")
    }

    /// Sample moments of the raw recorded values (in seconds), for the parametric mean inference.
    fn moments_mean(&self) -> SampleMoments {
        SampleMoments::new(self.n_r(), self.sum, self.sum2)
    }

    /// Student's one-sample t statistic for the equality of `mean(latency(f))` and `mu0`.
    ///
    /// The mean is the location parameter that batching preserves in expectation (the grand mean of
    /// batch means is unbiased for the true per-execution mean at every batch size). Under batching
    /// the recorded batch means are approximately normal (CLT), so the Student's t machinery applies
    /// directly to the raw recorded values; no log-normality assumption is used.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `number of recorded values <= 1`.
    /// - `self.stdev_r() == 0`.
    pub fn student_mean_t(&self, mu0: FpSeconds) -> f64 {
        student_1samp_t(&self.moments_mean(), mu0.into())
            .expect("`number of recorded values <= 1` or `self.stdev_r() == 0`")
    }

    /// Degrees of freedom for the one-sample Student's t statistic for `mean(latency(f))`.
    pub fn student_mean_df(&self) -> f64 {
        self.n_r() as f64 - 1.
    }

    /// p-value of Student's one-sample t-test for the equality of `mean(latency(f))` and `mu0`.
    ///
    /// See [`Self::student_mean_t`] for the batching rationale.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `number of recorded values <= 1`.
    /// - `self.stdev_r() == 0`.
    pub fn student_mean_p(&self, mu0: FpSeconds, alt_hyp: AltHyp) -> f64 {
        student_1samp_p(&self.moments_mean(), mu0.into(), alt_hyp)
            .expect("`number of recorded values <= 1` or `self.stdev_r() == 0`")
    }

    /// Student's one-sample confidence interval for `mean(latency(f))`, expressed as a pair of
    /// [`FpSeconds`] (low, high), with confidence level `(1 - alpha)`.
    ///
    /// See [`Self::student_mean_t`] for the batching rationale. When [`Self::mean_check`] flags
    /// contamination, prefer the robust bootstrap interval [`Self::mean_rob_ci`].
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `number of recorded values <= 1`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn student_mean_ci(&self, alpha: f64) -> (FpSeconds, FpSeconds) {
        let Ci(low, high) = student_1samp_ci(&self.moments_mean(), alpha).expect(
            "`number of recorded values <= 1` or `alpha` not in open interval `(0, 1)`",
        );
        (low.into(), high.into())
    }

    /// Position of `value` with respect to Student's one-sample confidence interval for
    /// `mean(latency(f))`, with confidence level `(1 - alpha)`.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `number of recorded values <= 1`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn student_value_position_wrt_mean_ci(
        &self,
        value: FpSeconds,
        alpha: f64,
    ) -> PositionWrtCi {
        let (low, high) = self.student_mean_ci(alpha);
        if value < low {
            PositionWrtCi::Below
        } else if value > high {
            PositionWrtCi::Above
        } else {
            PositionWrtCi::In
        }
    }

    /// Student's one-sample test of the hypothesis that `mean(latency(f)) == mu0`.
    ///
    /// See [`Self::student_mean_t`] for the batching rationale.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `number of recorded values <= 1`.
    /// - `self.stdev_r() == 0`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn student_mean_test(
        &self,
        mu0: FpSeconds,
        alt_hyp: AltHyp,
        alpha: f64,
    ) -> HypTestResult {
        student_1samp_test(&self.moments_mean(), mu0.into(), alt_hyp, alpha).expect(
            "`number of recorded values <= 1` or `self.stdev_r() == 0` or `alpha` not in open interval `(0, 1)`",
        )
    }

    /// Contamination diagnostic comparing the naive arithmetic mean against the robust mean.
    ///
    /// The parametric mean interval ([`Self::student_mean_ci`]) is efficient on clean data but
    /// fragile under right-tail contamination (GC pauses, scheduler preemption), which inflates the
    /// arithmetic mean while the robust [`Self::mean_rob`] resists it. `mean_check` reports both
    /// estimates, their relative gap `(naive_mean - mean_rob) / mean_rob`, the standardized gap
    /// `rel_gap · √g / σ_Y`, the robust log-scale spread `σ_Y`, and a [`MeanVerdict`]; a positive
    /// gap is the fingerprint of contamination. The verdict keys on the *standardized* gap so its
    /// false-alarm rate is stable across batch/sample sizes (see [`MeanVerdict`], including the
    /// high-dispersion caveat). When the verdict is not [`MeanVerdict::Insignificant`], prefer the
    /// robust estimate / [`Self::mean_rob_ci`].
    ///
    /// # Panics
    /// Panics if the number of recorded values is zero.
    pub fn mean_check(&self) -> MeanCheck {
        let naive_mean = self.mean();
        let mean_rob = self.mean_rob();
        let rel_gap = (naive_mean.as_f64() - mean_rob.as_f64()) / mean_rob.as_f64();
        let sigma_y = self.rousseeuw_croux_q_ls();
        let g = self.n_r();
        // Standardize by the gap's clean-data sampling scale σ_Y/√g. Degenerate σ_Y (all batch
        // means equal) ⇒ no dispersion to standardize against ⇒ treat as insignificant.
        let std_gap = if sigma_y > 0.0 && g > 0 {
            rel_gap * (g as f64).sqrt() / sigma_y
        } else {
            0.0
        };
        let verdict = MeanVerdict::from_std_gap(std_gap);
        MeanCheck {
            naive_mean,
            mean_rob,
            rel_gap,
            std_gap,
            sigma_y,
            verdict,
        }
    }

    #[cfg(feature = "_test_support")]
    #[inline(always)]
    /// Reference to the raw HDR histogram. Gated by feature **"_test_support"**.
    pub fn hist(&self) -> &Histogram<u64> {
        &self.hist
    }

    #[cfg(feature = "_test_support")]
    #[inline(always)]
    /// Raw sum of recorded latencies. Gated by feature **"_test_support"**.
    pub fn sum(&self) -> f64 {
        self.sum
    }

    #[cfg(feature = "_test_support")]
    #[inline(always)]
    /// Raw sum of squares of recorded latencies. Gated by feature **"_test_support"**.
    pub fn sum2(&self) -> f64 {
        self.sum2
    }

    #[inline(always)]
    /// Number of non-zero observations.
    pub fn n_nz(&self) -> u64 {
        self.n_nz
    }

    #[cfg(feature = "_test_support")]
    #[inline(always)]
    /// Raw sum of natural logarithms of latencies. Gated by feature **"_test_support"**.
    pub fn sum_ln(&self) -> f64 {
        self.sum_ln
    }

    #[cfg(feature = "_test_support")]
    #[inline(always)]
    /// Raw sum of squares of natural logarithms of latencies. Gated by feature **"_test_support"**.
    pub fn sum2_ln(&self) -> f64 {
        self.sum2_ln
    }
}

impl Debug for BenchOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("BenchOut {{ recording_unit={:?}, sigfig={}, n={}, sum={}, sum2={}, n_nz={}, sum_ln={}, sum2_ln={}, summary={:?} }}",
            self.recording_unit,
            self.hist.sigfig(),
            self.n_r(),
            self.sum,
            self.sum2,
            self.n_nz,
            self.sum_ln,
            self.sum2_ln,
            self.summary()))
    }
}

impl From<multi::BenchOut<1>> for BenchOut {
    fn from(value: multi::BenchOut<1>) -> Self {
        let [b] = value.arr;
        b
    }
}

#[cfg(test)]
#[cfg(feature = "_test")]
// cargo test --package bench_utils --lib --all-features -- bench_out::test --nocapture
mod test {
    use super::*;
    use crate::rel_approx_eq_fpsecs;
    use crate::{
        BenchCfg,
        test_support::{LO_STDEV_LN, lognormal_samp},
    };
    use basic_stats::{
        approx_eq,
        core::{AcceptedHyp, PositionWrtCi},
        normal::{student_1samp_df, student_1samp_p},
        rel_approx_eq,
    };
    use statrs::distribution::{ContinuousCDF, Normal};

    const ALPHA: f64 = 0.05;

    #[test]
    fn test_from_iter_to_iter() {
        const EPSILON: f64 = 0.001;
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(
            [
                FpSeconds::from_millis(1),
                FpSeconds::from_millis(1),
                FpSeconds::from_millis(2),
            ]
            .into_iter(),
        );
        let items: Vec<_> = out.iter().collect();
        assert_eq!(items.len(), 3);
        // HDR histogram has slight quantization; compare approximately
        rel_approx_eq_fpsecs!(items[0], FpSeconds::from_millis(1), EPSILON);
        rel_approx_eq_fpsecs!(items[1], FpSeconds::from_millis(1), EPSILON);
        rel_approx_eq_fpsecs!(items[2], FpSeconds::from_millis(2), EPSILON);
    }

    #[test]
    fn test_from_iter_to_iter_with_counts() {
        const EPSILON: f64 = 0.001;
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(
            [
                FpSeconds::from_millis(1),
                FpSeconds::from_millis(1),
                FpSeconds::from_millis(2),
            ]
            .into_iter(),
        );
        let pairs: Vec<_> = out.iter_with_counts().collect();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].1, 2);
        assert_eq!(pairs[1].1, 1);
        // HDR histogram has slight quantization; compare approximately
        rel_approx_eq_fpsecs!(pairs[0].0, FpSeconds::from_millis(1), EPSILON);
        rel_approx_eq_fpsecs!(pairs[1].0, FpSeconds::from_millis(2), EPSILON);
    }

    #[test]
    fn test_from_iter_with_counts_to_iter() {
        const EPSILON: f64 = 0.001;
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter_with_counts(
            [
                (FpSeconds::from_millis(1), 2),
                (FpSeconds::from_millis(2), 1),
            ]
            .into_iter(),
        );
        let items: Vec<_> = out.iter().collect();
        assert_eq!(items.len(), 3);
        // HDR histogram has slight quantization; compare approximately
        rel_approx_eq_fpsecs!(items[0], FpSeconds::from_millis(1), EPSILON);
        rel_approx_eq_fpsecs!(items[1], FpSeconds::from_millis(1), EPSILON);
        rel_approx_eq_fpsecs!(items[2], FpSeconds::from_millis(2), EPSILON);
    }

    #[test]
    fn test_from_iter_with_counts_to_iter_with_counts() {
        const EPSILON: f64 = 0.001;
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter_with_counts(
            [
                (FpSeconds::from_millis(1), 2),
                (FpSeconds::from_millis(2), 1),
            ]
            .into_iter(),
        );
        let pairs: Vec<_> = out.iter_with_counts().collect();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].1, 2);
        assert_eq!(pairs[1].1, 1);
        // HDR histogram has slight quantization; compare approximately
        rel_approx_eq_fpsecs!(pairs[0].0, FpSeconds::from_millis(1), EPSILON);
        rel_approx_eq_fpsecs!(pairs[1].0, FpSeconds::from_millis(2), EPSILON);
    }

    #[test]
    fn test_record_with_counts_moments_match_expanded() {
        const EPSILON: f64 = 1e-12;
        let cfg = BenchCfg::default();
        let pairs = [
            (FpSeconds::from_millis(1), 3),
            (FpSeconds::from_millis(2), 1),
            (FpSeconds::from_millis(4), 5),
            (FpSeconds::from_millis(8), 2),
        ];

        let mut out_counts = BenchOut::new(&cfg, None);
        out_counts.record_from_iter_with_counts(pairs.into_iter());

        let mut out_expanded = BenchOut::new(&cfg, None);
        out_expanded.record_from_iter(
            pairs
                .into_iter()
                .flat_map(|(v, count)| iter::repeat_n(v, count)),
        );

        assert_eq!(out_counts.n_r(), out_expanded.n_r());
        assert_eq!(out_counts.n_nz(), out_expanded.n_nz());
        rel_approx_eq!(out_counts.mean().0, out_expanded.mean().0, EPSILON);
        rel_approx_eq!(out_counts.stdev_r().0, out_expanded.stdev_r().0, EPSILON);
        rel_approx_eq!(out_counts.mean_ln_r(), out_expanded.mean_ln_r(), EPSILON);
        rel_approx_eq!(out_counts.stdev_ln_r(), out_expanded.stdev_ln_r(), EPSILON);
    }

    #[test]
    fn test_descriptive_stats() {
        // Descriptive stats are computed from a genuinely random sample (see `normal_rand_samp`
        // in `basic_stats`), so tail percentiles in particular carry real sampling noise around
        // their theoretical values; 0.001 was tight enough only for the previous low-discrepancy
        // (non-random) generator.
        const EPSILON: f64 = 0.005;

        // in ln of microseconds
        let mu_micro = 8.;
        // in ln of seconds: ln(exp(mu_micro)*1e-6) = mu_micro - ln(1e6)
        let mu = mu_micro - 1e6_f64.ln();
        let sigma = *LO_STDEV_LN;
        let samp_size = 20_000;

        let cfg = BenchCfg::default();

        let lognormal_samp = lognormal_samp(mu, sigma, samp_size);
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(lognormal_samp);

        assert_eq!(out.n_r() as usize, samp_size);

        let normal = Normal::new(mu, sigma).unwrap();

        let exp_mean_ln = mu;
        let exp_stdev_ln = sigma;
        let exp_mean = (mu + 0.5 * sigma.powi(2)).exp();
        let exp_stdev = exp_mean * ((sigma.powi(2).exp() - 1.).sqrt());
        let exp_p1 = normal.inverse_cdf(0.01).exp();
        let exp_p5 = normal.inverse_cdf(0.05).exp();
        let exp_p10 = normal.inverse_cdf(0.10).exp();
        let exp_p25 = normal.inverse_cdf(0.25).exp();
        let exp_median = normal.inverse_cdf(0.5).exp();
        let exp_p75 = normal.inverse_cdf(0.75).exp();
        let exp_p90 = normal.inverse_cdf(0.90).exp();
        let exp_p95 = normal.inverse_cdf(0.95).exp();
        let exp_p99 = normal.inverse_cdf(0.99).exp();

        let summary = out.summary();

        println!("exp_mean={:?}, out.mean={:?}", exp_mean, out.mean());
        println!("exp_stdev={:?}, out.stdev={:?}", exp_stdev, out.stdev());
        println!("exp_p1={:?}, summary.p1={:?}", exp_p1, summary.p1);
        println!("exp_p5={:?}, summary.p5={:?}", exp_p5, summary.p5);
        println!("exp_p10={:?}, summary.p10={:?}", exp_p10, summary.p10);
        println!("exp_p25={:?}, summary.p25={:?}", exp_p25, summary.p25);
        println!(
            "exp_median={:?}, summary.median={:?}",
            exp_median, summary.p50
        );
        println!("exp_p75={:?}, summary.p75={:?}", exp_p75, summary.p75);
        println!("exp_p90={:?}, summary.p90={:?}", exp_p90, summary.p90);
        println!("exp_p95={:?}, summary.p95={:?}", exp_p95, summary.p95);
        println!("exp_p99={:?}, summary.p99={:?}", exp_p99, summary.p99);

        rel_approx_eq!(exp_mean, out.mean().0, EPSILON);
        rel_approx_eq!(exp_stdev, out.stdev().0, EPSILON);
        rel_approx_eq!(exp_median, out.median_r().as_f64(), EPSILON);
        approx_eq!(exp_mean_ln, out.mean_ln_r(), EPSILON);
        approx_eq!(exp_stdev_ln, out.stdev_ln_r(), EPSILON);

        rel_approx_eq!(exp_mean, summary.mean.0, EPSILON);
        rel_approx_eq!(exp_stdev, summary.stdev.0, EPSILON);
        rel_approx_eq!(exp_p1, summary.p1.as_f64(), EPSILON);
        rel_approx_eq!(exp_p5, summary.p5.as_f64(), EPSILON);
        rel_approx_eq!(exp_p10, summary.p10.as_f64(), EPSILON);
        rel_approx_eq!(exp_p25, summary.p25.as_f64(), EPSILON);
        rel_approx_eq!(exp_median, summary.p50.as_f64(), EPSILON);
        rel_approx_eq!(exp_p75, summary.p75.as_f64(), EPSILON);
        rel_approx_eq!(exp_p90, summary.p90.as_f64(), EPSILON);
        rel_approx_eq!(exp_p95, summary.p95.as_f64(), EPSILON);
        rel_approx_eq!(exp_p99, summary.p99.as_f64(), EPSILON);
    }

    #[test]
    fn test_student() {
        const EPSILON: f64 = 0.001;

        // in ln of microseconds
        let mu_micro = 8.;
        // in ln of seconds: ln(exp(mu_micro)*1e-6) = mu_micro - ln(1e6)
        let mu = mu_micro - 1e6_f64.ln();
        let sigma = *LO_STDEV_LN;
        let samp_size = 20_000;

        let cfg = BenchCfg::default();

        // Materialize the sample so the crate's exact `sum`/`sum2` accumulators and the reference
        // moments are computed over the very same raw second-values (the mean methods use the
        // accumulators, not the histogram, so this is an exact-equality check).
        let samp: Vec<FpSeconds> = lognormal_samp(mu, sigma, samp_size).collect();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(samp.iter().copied());

        let moments_ns = SampleMoments::from_iterator(samp.iter().map(|x| x.as_f64()));

        assert_eq!(out.n_r() as usize, samp_size);

        // The true (population) mean should lie inside the mean CI.
        let true_mean = FpSeconds((mu + sigma * sigma / 2.0).exp());
        let position = out.student_value_position_wrt_mean_ci(true_mean, ALPHA);
        assert_eq!(position, PositionWrtCi::In);

        {
            // mu0 at the sample mean → t ≈ 0 → accept the null.
            let mu0 = out.mean();
            let alt_hyp = AltHyp::Ne;
            let exp_accepted_hyp = AcceptedHyp::Null;

            let exp_t = student_1samp_t(&moments_ns, mu0.as_f64()).unwrap();
            let exp_df = student_1samp_df(&moments_ns).unwrap();
            let exp_p = student_1samp_p(&moments_ns, mu0.as_f64(), alt_hyp).unwrap();
            let exp_ci = student_1samp_ci(&moments_ns, ALPHA).unwrap();

            approx_eq!(exp_t, out.student_mean_t(mu0), EPSILON);
            approx_eq!(exp_df, out.student_mean_df(), EPSILON);
            rel_approx_eq!(exp_p, out.student_mean_p(mu0, alt_hyp), EPSILON);
            rel_approx_eq_fpsecs!(FpSeconds(exp_ci.0), out.student_mean_ci(ALPHA).0, EPSILON);
            rel_approx_eq_fpsecs!(FpSeconds(exp_ci.1), out.student_mean_ci(ALPHA).1, EPSILON);
            let student_test = out.student_mean_test(mu0, alt_hyp, ALPHA);
            println!("out.student_mean_test={student_test:?}");
            assert_eq!(exp_accepted_hyp, student_test.accepted());
        }

        {
            // mu0 1% below the sample mean, one-sided Gt → reject the null (huge n, tiny SE).
            let mu0 = out.mean() * 0.99;
            let alt_hyp = AltHyp::Gt;
            let exp_accepted_hyp = AcceptedHyp::Alt;

            let exp_t = student_1samp_t(&moments_ns, mu0.as_f64()).unwrap();
            let exp_df = student_1samp_df(&moments_ns).unwrap();
            let exp_p = student_1samp_p(&moments_ns, mu0.as_f64(), alt_hyp).unwrap();

            rel_approx_eq!(exp_t, out.student_mean_t(mu0), EPSILON);
            approx_eq!(exp_df, out.student_mean_df(), EPSILON);
            approx_eq!(exp_p, out.student_mean_p(mu0, alt_hyp), EPSILON);
            let student_test = out.student_mean_test(mu0, alt_hyp, ALPHA);
            println!("out.student_mean_test={student_test:?}");
            assert_eq!(exp_accepted_hyp, student_test.accepted());
        }
    }

    #[test]
    #[should_panic(expected = "number of recorded values is zero")]
    fn test_mean_panics_on_empty() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(std::iter::empty());
        out.mean();
    }

    #[test]
    #[should_panic(expected = "number of recorded values is zero")]
    fn test_stdev_panics_on_empty() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(std::iter::empty());
        out.stdev();
    }

    #[test]
    #[should_panic(expected = "number of recorded values is zero")]
    fn test_median_panics_on_empty() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(std::iter::empty());
        out.median_r();
    }

    #[test]
    #[should_panic(expected = "number of non-zero observations is zero")]
    fn test_mean_ln_panics_on_empty() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(std::iter::empty());
        out.mean_ln_r();
    }

    #[test]
    #[should_panic(expected = "number of non-zero observations is zero")]
    fn test_stdev_ln_panics_on_empty() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(std::iter::empty());
        out.stdev_ln_r();
    }

    #[test]
    fn test_student_mean_t_panics_on_single() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter([FpSeconds::from_millis(1)].into_iter());
        let result =
            std::panic::catch_unwind(|| out.student_mean_t(FpSeconds::from_millis(1)));
        assert!(result.is_err());
    }

    #[test]
    fn test_mean_check_verdict_bands() {
        // Bands key on the standardized gap; non-positive gaps are always insignificant.
        assert_eq!(MeanVerdict::from_std_gap(-5.0), MeanVerdict::Insignificant);
        assert_eq!(MeanVerdict::from_std_gap(0.0), MeanVerdict::Insignificant);
        assert_eq!(
            MeanVerdict::from_std_gap(MEAN_CHECK_MILD_Z - 0.01),
            MeanVerdict::Insignificant
        );
        assert_eq!(
            MeanVerdict::from_std_gap(MEAN_CHECK_MILD_Z),
            MeanVerdict::Mild
        );
        assert_eq!(
            MeanVerdict::from_std_gap(MEAN_CHECK_SIGNIFICANT_Z - 0.01),
            MeanVerdict::Mild
        );
        assert_eq!(
            MeanVerdict::from_std_gap(MEAN_CHECK_SIGNIFICANT_Z),
            MeanVerdict::Significant
        );
        assert_eq!(
            MeanVerdict::from_std_gap(MEAN_CHECK_SIGNIFICANT_Z + 10.0),
            MeanVerdict::Significant
        );
    }

    #[test]
    fn test_median_rob_ci_reproducible_and_brackets() {
        // Batched lognormal data: the BCa median CI should be reproducible for a fixed seed and
        // bracket the point estimate. Kept small on purpose: BCa cost is O(n_resamples + n_batches)
        // robust-median recomputations, each an O(D^2) Rousseeuw-Croux pass, so this uses a modest
        // batch count and resample count to stay fast.
        let cfg = BenchCfg::default();
        let mu = -6.0;
        let sigma = *LO_STDEV_LN;
        let n_batches = 80;
        let samp: Vec<FpSeconds> = lognormal_samp(mu, sigma, n_batches * 8).collect();
        let mut out = BenchOut::new(&cfg, Some(8));
        // Record batch means: average groups of 8.
        let batched = samp.chunks(8).filter(|c| c.len() == 8).map(|c| {
            let s: f64 = c.iter().map(|x| x.as_f64()).sum();
            FpSeconds(s / 8.0)
        });
        out.record_from_iter(batched);

        let ci1 = out.median_rob_ci_with(ALPHA, 60, 7);
        let ci2 = out.median_rob_ci_with(ALPHA, 60, 7);
        assert_eq!(ci1, ci2, "fixed seed must give identical CI");
        let center = out.median_rob();
        assert!(
            ci1.0 <= center && center <= ci1.1,
            "CI {ci1:?} should bracket median_rob {center:?}"
        );
    }

    #[test]
    fn test_reset() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter([FpSeconds::from_millis(1), FpSeconds::from_millis(2)].into_iter());
        assert_eq!(out.n_r(), 2);
        out.reset();
        assert_eq!(out.n_r(), 0);
    }

    #[test]
    fn test_rousseeuw_croux_q_not_zero_for_tight_mode_with_tail() {
        // Regression test for `rousseeuw_croux_q_general` collapsing to a literal, misleading 0:
        // a "tight mode + long tail" sample -- common for real latency data -- can put enough of
        // the sample into a single HDR bucket that its own C(count_i,2) same-bucket pairs alone
        // exceed the target rank, so `value_at_quantile` used to return exactly 0 for both natural
        // and log space before this bucket collapsed into the smallest recorded (nonzero) value.
        //
        // 480 identical values (a dominant mode) + 20 larger ones (a tail) reproduces this: with
        // g=500, target rank r=C(251,2)=31375, and C(480,2)=114960 >> r, so the old code returned
        // exactly 0 for both `rousseeuw_croux_q_ns` and `rousseeuw_croux_q_ls`.
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        let mode = std::iter::repeat_n(FpSeconds::from_millis(1), 480);
        let tail = std::iter::repeat_n(FpSeconds::from_millis(2), 20);
        out.record_from_iter(mode.chain(tail));

        assert_eq!(out.n_r(), 500);
        assert!(
            out.rousseeuw_croux_q_ns() > FpSeconds::ZERO,
            "rousseeuw_croux_q_ns should not collapse to exactly zero for a tight-mode+tail sample"
        );
        assert!(
            out.rousseeuw_croux_q_ls() > 0.0,
            "rousseeuw_croux_q_ls should not collapse to exactly zero for a tight-mode+tail sample"
        );
    }

    #[test]
    fn test_rousseeuw_croux_q_ls_tracks_stdev_ln_r_for_clean_sample() {
        // Sanity/non-degenerate-reading guard for `rousseeuw_croux_q_general`: on a clean
        // (uncontaminated) lognormal sample, `ln Y` is exactly normal, so the non-robust
        // `stdev_ln_r` and the robust `rousseeuw_croux_q_ls` should closely agree -- unlike on
        // contaminated data, there's no reason for the robust estimator to read differently here.
        // `sigma` is deliberately tight relative to the HDR histogram's bucket width so that a
        // coarse resolution (e.g. the pre-fix `DEFAULT_SIGFIG = 3`, or worse) pushes many samples
        // into the same few buckets; a badly under-resolved histogram would then either pin `Q_n`
        // toward the bucket-width fallback used for tied pairs (reading too small) or, if the
        // fallback itself dwarfs the true dispersion, read too large -- either way, far from
        // `stdev_ln_r`. At the current `DEFAULT_SIGFIG` this reads within ~10% of `stdev_ln_r`;
        // this test only asserts the much looser bound needed to catch a gross regression.
        const G: usize = 300;
        let sigma = 0.0002_f64;
        let samp: Vec<FpSeconds> = lognormal_samp(0.0, sigma, G).collect();

        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(samp.into_iter());

        let stdev_ln_r = out.stdev_ln_r();
        let rc_q_ls = out.rousseeuw_croux_q_ls();
        let ratio = rc_q_ls / stdev_ln_r;
        assert!(
            (0.3..3.0).contains(&ratio),
            "rousseeuw_croux_q_ls ({rc_q_ls}) should be within a loose factor of stdev_ln_r \
             ({stdev_ln_r}) for a clean, uncontaminated sample; ratio={ratio}"
        );
    }

    /// Batches a raw lognormal sample of individual latencies into `n / k` group means, following
    /// the reference's `Y_i = mean(X_{(i-1)k+1}, ..., X_{ik})` construction.
    fn batch_lognormal_samp(mu: f64, sigma: f64, k: usize, g: usize) -> Vec<FpSeconds> {
        let raw: Vec<FpSeconds> = lognormal_samp(mu, sigma, k * g).collect();
        raw.chunks(k)
            .map(|chunk| chunk.iter().copied().sum::<FpSeconds>() / k as f64)
            .collect()
    }

    #[test]
    fn test_s_hat_well_scaled() {
        // Regression test for the `pure_s_hat` center bug: `m` must be `ln(median in recording
        // units)`, in the same log space as `mid.ln()`, not the un-logged natural-scale median in
        // seconds. Before the fix, mixing those scales produced `s_hat` values in the tens
        // (dominated by the un-logged `m` term) instead of the well-scaled dispersion estimate
        // (order 0.1-1) that `median_rob`'s `Ŝ` bands (§1.5 of the reference) are calibrated to.
        const K: usize = 16;
        const G: usize = 500;
        let sigma = 0.5_f64;
        let batched = batch_lognormal_samp(0.0, sigma, K, G);

        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, Some(K));
        out.record_from_iter(batched.into_iter());

        let s_hat = out.s_hat();
        assert!(
            (0.0..1.0).contains(&s_hat),
            "s_hat should be a small, well-scaled dispersion estimate; got {s_hat}"
        );
    }

    #[test]
    fn test_median_rob_selects_matching_band() {
        // Regression test for the `pure_s_hat` center bug (see `test_s_hat_well_scaled`): before
        // the fix, the inflated `s_hat` (tens instead of tenths) meant `median_rob` fell through
        // to `median_rmom_estimator` for every k > 1, regardless of the sample's actual
        // dispersion. Pick (sigma, k) that lands `s_hat` in the mildly-skewed band
        // (0.3 < Ŝ <= 0.6; §1.5 of the reference) and confirm `median_rob` now actually selects
        // `median_log_space_estimator` there, instead of always defaulting to RMoM.
        const K: usize = 24;
        const G: usize = 2_000;
        let sigma = 1.5_f64;
        let batched = batch_lognormal_samp(0.0, sigma, K, G);

        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, Some(K));
        out.record_from_iter(batched.into_iter());

        let s_hat = out.s_hat();
        assert!(
            (0.3..=0.6).contains(&s_hat),
            "expected this (sigma, k) combination to land in the mildly-skewed Ŝ band; got s_hat={s_hat}"
        );

        let median_rob = out.median_rob();
        let median_log_space = out.median_log_space_estimator();
        assert_eq!(
            median_rob, median_log_space,
            "median_rob should select median_log_space_estimator when 0.3 < s_hat <= 0.6"
        );
    }

    #[test]
    // cargo test --package bench_utils --lib --all-features -- bench_out::test::test_mean_rob_and_median_rob_batched --exact --nocapture --include-ignored
    fn test_mean_rob_and_median_rob_batched() {
        const EPSILON: f64 = 0.01;
        const K: usize = 16;
        const G: usize = 2_000;
        let mu = 0.0_f64;
        let sigma = 0.5_f64;
        let batched = batch_lognormal_samp(mu, sigma, K, G);

        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, Some(K));
        out.record_from_iter(batched.into_iter());

        let true_mean = (mu + sigma.powi(2) / 2.0).exp();
        let true_median = mu.exp();

        let mean_log_space = out.mean_log_space_estimator().as_f64();
        let mean_rob = out.mean_rob().as_f64();
        let median_log_space = out.median_log_space_estimator().as_f64();
        let median_rmom = out.median_rmom_estimator().as_f64();
        let median_rob = out.median_rob().as_f64();

        assert!(mean_log_space.is_finite());
        assert!(median_log_space.is_finite());
        assert!(median_rmom.is_finite());

        // mean_rob is defined as mean_log_space_estimator for every k.
        approx_eq!(mean_log_space, mean_rob, 1e-9);

        rel_approx_eq!(true_mean, mean_rob, EPSILON);
        rel_approx_eq!(true_median, median_rob, EPSILON);
    }
}
