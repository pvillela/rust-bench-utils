//! Module defining the key data structure produced by [`crate::bench_run`].

use crate::{BenchCfg, FpSeconds, LatencyUnit, SummaryStats, dev_support::memoized_value, multi};
use basic_stats::{
    core::{AltHyp, Ci, HypTestResult, PositionWrtCi, SampleMoments, sample_mean, sample_stdev},
    normal::{student_1samp_ci, student_1samp_p, student_1samp_t, student_1samp_test},
};
use hdrhistogram::Histogram;
use std::{fmt::Debug, iter, sync::Mutex};

/// Constructs a [`Histogram<u64>`]. The arguments correspond to [Histogram::high] and [Histogram::sigfig].
pub(crate) fn new_hdrhist(hist_high: u64, hist_sigfig: u8) -> Histogram<u64> {
    let mut hist = Histogram::<u64>::new_with_max(hist_high, hist_sigfig)
        .expect("should not happen given histogram construction");
    hist.auto(true);
    hist
}

struct CachedStats {
    rousseeuw_croux_q: Option<f64>,
}

impl Default for CachedStats {
    fn default() -> Self {
        Self {
            rousseeuw_croux_q: None,
        }
    }
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
/// The `*_ln_*` methods provide statistics for `mean(ln(latency(f)))`, where `ln` is the natural logarithm.
/// Under the assumption that `latency(f)` is approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
/// This assumption is widely supported by performance analysis theory and empirical data.
/// Thus, the `*_ln_*` methods are useful for the analysis of median latencies.
/// However, batching changes the statistical distribution of the recorded values -- the higher the batch size, the
/// more the recorded values deviate from log-normal and approach a normal distribution.
pub struct BenchOut {
    pub(crate) recording_unit: LatencyUnit,
    pub(crate) hist: Histogram<u64>,
    pub(crate) sum: f64,
    pub(crate) sum2: f64,
    pub(crate) n_nz: u64,
    pub(crate) sum_ln: f64,
    pub(crate) sum2_ln: f64,
    pub(crate) batch: Option<usize>,
    cached_stats: Mutex<CachedStats>,
}

impl BenchOut {
    #[doc(hidden)]
    /// Creates a new empty instance based on `cfg`.
    pub fn new(cfg: &BenchCfg, batch: Option<usize>) -> Self {
        let hist = new_hdrhist(20 * 1000 * 1000, cfg.sigfig());
        let sum = 0.;
        let sum2 = 0.;
        let n_nz = 0;
        let sum_ln = 0.;
        let sum2_ln = 0.;
        let batch = batch.map(|n| n.max(1));
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
            cached_stats,
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
        self.cached_stats = Mutex::new(CachedStats::default());
    }

    #[inline(always)]
    /// Updates `self` with an elapsed time observation for the target function.
    pub(crate) fn capture_data_with_counts(&mut self, latency_with_count: (FpSeconds, usize)) {
        let (mean_latency, count) = latency_with_count;
        let mean_elapsed_u64 = self.recording_unit.value_from_fpsecs(mean_latency);
        self.hist
            .record_n(mean_elapsed_u64, count as u64)
            .expect("can't happen: histogram is auto-resizable");

        let total_elapsed_f64 = (mean_latency * count).as_f64();
        self.sum += total_elapsed_f64;
        self.sum2 += total_elapsed_f64.powi(2);

        if latency_with_count.0 > FpSeconds::ZERO {
            let ln = total_elapsed_f64.ln();
            self.n_nz += 1;
            self.sum_ln += ln;
            self.sum2_ln += ln.powi(2);
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
    /// # Panics
    /// Panics if the number of recorded values is zero.
    pub fn median_r(&self) -> FpSeconds {
        self.summary().p50
    }

    //=== Helper functions for estimators ===

    #[allow(unused)]
    fn cv(&self) -> f64 {
        todo!()
    }

    #[allow(unused)]
    fn cv_rob(&self) -> f64 {
        todo!()
    }

    fn rousseeuw_croux_q_general(
        &self,
        inflate: impl Fn(u64) -> u64,
        deflate: impl Fn(u64) -> f64,
    ) -> f64 {
        const C_Q: f64 = 2.2219;

        let mut rc_hist = Histogram::new_from(&self.hist);
        for (i, iv_i) in self.hist.iter_recorded().enumerate() {
            let v = iv_i.value_iterated_to();
            if v == 0 {
                continue;
            }
            let mid_i = inflate(self.hist.median_equivalent(v));
            let count_i: u64 = iv_i.count_at_value();
            if count_i > 1 {
                rc_hist
                    .record_n(0, count_i - 1)
                    .expect("shouldn't happen as histogram is sized properly");
            }
            for iv_j in self.hist.iter_recorded().skip(i + 1) {
                let v = iv_j.value_iterated_to();
                let mid_j = inflate(self.hist.median_equivalent(v));
                let count_j = iv_j.count_at_value();
                let abs_diff = mid_i.abs_diff(mid_j);
                let count = count_i * count_j;
                rc_hist
                    .record_n(abs_diff, count)
                    .expect("shouldn't happen as histogram is sized properly");
            }
        }
        let quartile = rc_hist.value_at_quantile(0.25);
        deflate(quartile) * C_Q
    }

    #[allow(unused)]
    /// Returns the Rousseeuw-Croux Q statistic computed in natural scale.
    fn rousseeuw_croux_q_ns(&self) -> f64 {
        let inflate = |value: u64| -> u64 { value };
        let deflate = |value: u64| -> f64 { value as f64 };

        self.rousseeuw_croux_q_general(inflate, deflate)
    }

    #[allow(unused)]
    /// Returns the Rousseeuw-Croux Q statistic computed in log scale.
    fn rousseeuw_croux_q_ls(&self) -> f64 {
        let max = self.hist.max() as f64;
        let multiplyer = max / max.ln();
        let inflate = |value: u64| -> u64 { ((value as f64).ln() * multiplyer).round() as u64 };
        let deflate = |value: u64| -> f64 { ((value as f64) / multiplyer).exp() };

        self.rousseeuw_croux_q_general(inflate, deflate)
    }

    /// Returns the Rousseeuw-Croux Q statistic.
    pub fn rousseeuw_croux_q(&self) -> f64 {
        let mut rcq = self
            .cached_stats
            .lock()
            .expect("mutex shouldn't be poisoned")
            .rousseeuw_croux_q;
        memoized_value(&mut rcq, || self.rousseeuw_croux_q_ls())
    }

    #[allow(unused)]
    fn sigma2_rousseeuw_croux(&self) -> f64 {
        let sigma_y = self.rousseeuw_croux_q();
        (1.0 + self.bsz() as f64 * sigma_y.powi(2) / self.mean().powi(2)).ln()
    }

    //=== Estimators of population mean ===

    /// Robust estimator of mean latency for moderate to large batch sizes (Y near-symmetric).
    fn mean_a_rousseeuw_croux(&self) -> FpSeconds {
        self.mean() * (-self.sigma2_rousseeuw_croux() / 2.0).exp()
    }

    /// Robust estimator of mean latency for small batch size (Y still skewed, n_r large).
    fn mean_b_rousseeuw_croux(&self) -> FpSeconds {
        self.mean() * (-self.sigma2_rousseeuw_croux() / 2.0).exp()
    }

    /// Robust estimator of mean latency across batch sizes.
    pub fn mean_rob(&self) -> FpSeconds {
        if self.cv_rob() >= 0.2 {
            self.mean_b_rousseeuw_croux()
        } else {
            self.mean_a_rousseeuw_croux()
        }
    }

    //=== Estimators of population median ===

    /// Robust estimator of median latency for moderate to large batch sizes (Y near-symmetric).
    fn median_a_rousseeuw_croux(&self) -> FpSeconds {
        self.median_r() * (-self.sigma2_rousseeuw_croux() / 2.0).exp()
    }

    /// Robust estimator of median latency for small batch size (Y still skewed, n_r large).
    fn median_b_rousseeuw_croux(&self) -> FpSeconds {
        self.mean() * (-self.sigma2_rousseeuw_croux() / 2.0).exp()
    }

    /// Robust estimator of median latency across batch sizes.
    pub fn median_rob(&self) -> FpSeconds {
        if self.cv_rob() >= 0.2 {
            self.median_b_rousseeuw_croux()
        } else {
            self.median_a_rousseeuw_croux()
        }
    }

    fn simple_median(&self) -> FpSeconds {
        const HI_BSZ: usize = 100;
        let median_1 = self.mean_ln_r().exp();
        let median_hi_bsz =
            self.median_r() / (1.0 + self.stdev().powi(2) / self.median_r().powi(2)).sqrt();
        let clipped_bsz = self.bsz().min(HI_BSZ);
        let median_interp = ((median_1.ln() * (HI_BSZ - clipped_bsz) as f64
            + median_hi_bsz.ln() * (clipped_bsz - 1) as f64)
            / (HI_BSZ - 1) as f64)
            .exp();
        median_interp.into()
    }

    pub fn median(&self) -> FpSeconds {
        self.simple_median()
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

    #[allow(unused)]
    /// Student's one-sample t statistic for
    /// the equality of `mean(ln(latency(f)))` and `ln_mu0` (where `ln` is the natural logarithm in [`FpSeconds`]),
    /// or equivalently, the equality of `median(latency(f))` and `exp(ln_mu0)`.
    ///
    /// Without batching, it can be assumed that
    /// `latency(f)` is approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// Arguments:
    /// - `ln_mu0`: hypothesized `mean(ln(latency(f)))`, or equivalently, `ln(median(latency(f)))`,
    ///   where the latency is expressed in [`FpSeconds`].
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `number of non-zero recorded values <= 1`.
    /// - `self.stdev_ln() == 0`.
    fn student_ln_t(&self, ln_mu0: f64) -> f64 {
        let moments = SampleMoments::new(self.n_nz, self.sum_ln, self.sum2_ln);
        student_1samp_t(&moments, ln_mu0)
            .expect("`number of non-zero recorded values <= 1` or `self.stdev_ln() == 0`")
    }

    #[allow(unused)]
    /// Student's one-sample t statistic for
    /// the equality of `mean(latency(f))` and `mu0` in [`FpSeconds`]),
    ///
    /// For sufficiently high batch sizes, the recorded values can be assumed to be approximately normal.
    ///
    /// Arguments:
    /// - `mu0`: hypothesized `mean(latency(f))`.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `number of recorded values <= 1`.
    /// - `self.stdev() == 0`.
    fn student_t(&self, mu0: FpSeconds) -> f64 {
        let moments = SampleMoments::new(self.n_r(), self.sum, self.sum2);
        student_1samp_t(&moments, mu0.into())
            .expect("`number of recorded values <= 1` or `self.stdev_ln() == 0`")
    }

    #[allow(unused)]
    /// Degrees of freedom for Student's t statistic for `mean(ln(latency(f)))` (where `ln` is the natural logarithm,
    /// in [`FpSeconds`]).
    ///
    /// Without batching, it can be assumed that
    /// `latency(f)` is approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// Under the assumption that `latency(f)` is approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
    /// Thus, this statistics equivalently pertains to `ln(median(latency(f)))`.
    fn student_ln_df(&self) -> f64 {
        self.n_nz as f64 - 1.
    }

    #[allow(unused)]
    /// Degrees of freedom for Student's t statistic for `mean(latency(f))` with latency expressed in [`FpSeconds`]).
    ///
    /// For sufficiently high batch sizes, the recorded values can be assumed to be approximately normal.
    fn student_df(&self) -> f64 {
        self.n_nz as f64 - 1.
    }

    /// p-value of Student's one-sample t-test for
    /// the equality of `mean(ln(latency(f)))` and `ln_mu0` (where `ln` is the natural logarithm, in [`FpSeconds`]),
    /// or equivalently, the equality of `median(latency(f))` and `exp(ln_mu0)`.
    ///
    /// Under the assumption that `latency(f)` is approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// Arguments:
    /// - `ln_mu0`: hypothesized `mean(ln(latency(f)))`, or equivalently, `ln(median(latency(f)))`,
    ///   where the latency is expressed in the recording unit.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - Number of non-zero observations <= 1.
    /// - `self.stdev_ln()` == 0.
    pub fn student_ln_p(&self, ln_mu0: f64, alt_hyp: AltHyp) -> f64 {
        let moments = SampleMoments::new(self.n_nz, self.sum_ln, self.sum2_ln);
        student_1samp_p(&moments, ln_mu0, alt_hyp)
            .expect("`number of non-zero observations <= 1` or `self.stdev_ln() == 0`")
    }

    /// Student's one-sample confidence interval for
    /// `mean(ln(latency(f)))` (where `ln` is the natural logarithm, in [`FpSeconds`]),
    /// with confidence level `(1 - alpha)`.
    ///
    /// Assumes that `latency(f)` is approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `Number of non-zero observations <= 1`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn student_ln_ci(&self, alpha: f64) -> Ci {
        let moments = SampleMoments::new(self.n_nz, self.sum_ln, self.sum2_ln);
        student_1samp_ci(&moments, alpha).expect(
            "`number of non-zero observations <= 1` or `alpha` not in open interval `(0, 1)`",
        )
    }

    /// Student's one-sample confidence interval for
    /// `median(latency(f))`,
    /// with confidence level `(1 - alpha)`.
    ///
    /// The confidence interval is expressed as a pair of [`FpSeconds`] representing the
    /// low and high ends of the interval.
    ///
    /// Assumes that `latency(f)` is approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `Sample size <= 1`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn student_median_ci(&self, alpha: f64) -> (FpSeconds, FpSeconds) {
        let Ci(log_low, log_high) = self.student_ln_ci(alpha);
        let low = log_low.exp().into();
        let high = log_high.exp().into();
        (low, high)
    }

    /// Position of `value` with respect to
    /// Student's one-sample confidence interval for
    /// `median(latency(f))`,
    /// with confidence level `(1 - alpha)`.
    ///
    /// Assumes that `latency(f)` is approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `Sample size <= 1`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn student_value_position_wrt_median_ci(
        &self,
        value: FpSeconds,
        alpha: f64,
    ) -> PositionWrtCi {
        let (low, high) = self.student_median_ci(alpha);
        if value < low {
            PositionWrtCi::Below
        } else if value > high {
            PositionWrtCi::Above
        } else {
            PositionWrtCi::In
        }
    }

    /// Student's one-sample test of the hypothesis that
    /// `mean(ln(latency(f))) == ln_mu0` (where `ln` is the natural logarithm, in [`FpSeconds`]), or equivalently,
    /// `median(latency(f)) == exp(ln_mu0)`.
    ///
    /// Under the assumption that `latency(f)` is approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// Arguments:
    /// - `ln_mu0`: hypothesized `mean(ln(latency(f)))`, or equivalently, `ln(median(latency(f)))`,
    ///   where the latency is expressed in the recording unit.
    /// - `alt_hyp`: alternative hypothesis.
    /// - `alpha`: confidence level is `1 - alpha`.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `number of non-zero observations <= 1`.
    /// - `self.stdev_ln()` == 0.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn student_ln_test(&self, ln_mu0: f64, alt_hyp: AltHyp, alpha: f64) -> HypTestResult {
        let moments = SampleMoments::new(self.n_nz, self.sum_ln, self.sum2_ln);
        student_1samp_test(&moments, ln_mu0, alt_hyp, alpha).expect("`number of non-zero observations <= 1` or `self.stdev_ln() == 0` or `alpha` not in open interval `(0, 1)`")
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
        normal::{normal_detm_samp, student_1samp_df, student_1samp_p},
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
    fn test_descriptive_stats() {
        const EPSILON: f64 = 0.001;

        // in ln of microseconds
        let mu_micro = 8.;
        // in ln of seconds: ln(exp(mu_micro)*1e-6) = mu_micro - ln(1e6)
        let mu = mu_micro - 1e6_f64.ln();
        let sigma = *LO_STDEV_LN;
        let samp_size = 20_000;

        let cfg = BenchCfg::default();
        let ru = cfg.recording_unit();

        let lognormal_samp = lognormal_samp(mu, sigma, samp_size);
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(lognormal_samp);

        assert_eq!(ru, LatencyUnit::NANO);
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

        let lognormal_samp = lognormal_samp(mu, sigma, samp_size);
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(lognormal_samp);

        let normal_samp = normal_detm_samp(mu, sigma, samp_size).unwrap();
        let moments_ln = SampleMoments::from_iterator(normal_samp);

        assert_eq!(out.recording_unit(), LatencyUnit::NANO);
        assert_eq!(out.n_r() as usize, samp_size);

        // The true median should lie inside the CI
        let true_median = FpSeconds(mu.exp());
        let position = out.student_value_position_wrt_median_ci(true_median, ALPHA);
        assert_eq!(position, PositionWrtCi::In);

        {
            let ratio_medians: f64 = 1.0;
            let mu0 = mu - ratio_medians.ln();
            let alt_hyp = AltHyp::Ne;
            let exp_accepted_hyp = AcceptedHyp::Null;

            let exp_t = student_1samp_t(&moments_ln, mu0).unwrap();
            let exp_df = student_1samp_df(&moments_ln).unwrap();
            let exp_p = student_1samp_p(&moments_ln, mu0, alt_hyp).unwrap();
            let exp_ln_ci = student_1samp_ci(&moments_ln, ALPHA).unwrap();
            let exp_ci_ns_low = exp_ln_ci.0.exp();
            let exp_ci_ns_high = exp_ln_ci.1.exp();

            approx_eq!(exp_t, out.student_ln_t(mu0), EPSILON);
            approx_eq!(exp_df, out.student_ln_df(), EPSILON);
            rel_approx_eq!(exp_p, out.student_ln_p(mu0, alt_hyp), EPSILON);
            rel_approx_eq_fpsecs!(
                FpSeconds(exp_ci_ns_low),
                out.student_median_ci(ALPHA).0,
                EPSILON
            );
            rel_approx_eq_fpsecs!(
                FpSeconds(exp_ci_ns_high),
                out.student_median_ci(ALPHA).1,
                EPSILON
            );
            let student_test = out.student_ln_test(mu0, alt_hyp, ALPHA);
            println!("out.student_test={student_test:?}");
            assert_eq!(exp_accepted_hyp, student_test.accepted());
        }

        {
            let ratio_medians: f64 = 1.01;
            let mu0 = mu - ratio_medians.ln();
            let alt_hyp = AltHyp::Gt;
            let exp_accepted_hyp = AcceptedHyp::Alt;

            let exp_t = student_1samp_t(&moments_ln, mu0).unwrap();
            let exp_df = student_1samp_df(&moments_ln).unwrap();
            let exp_p = student_1samp_p(&moments_ln, mu0, alt_hyp).unwrap();
            let exp_ln_ci = student_1samp_ci(&moments_ln, ALPHA).unwrap();
            let exp_ci_ns_low = exp_ln_ci.0.exp();
            let exp_ci_ns_high = exp_ln_ci.1.exp();

            rel_approx_eq!(exp_t, out.student_ln_t(mu0), EPSILON);
            approx_eq!(exp_df, out.student_ln_df(), EPSILON);
            approx_eq!(exp_p, out.student_ln_p(mu0, alt_hyp), EPSILON);
            rel_approx_eq_fpsecs!(
                FpSeconds(exp_ci_ns_low),
                out.student_median_ci(ALPHA).0,
                EPSILON
            );
            rel_approx_eq_fpsecs!(
                FpSeconds(exp_ci_ns_high),
                out.student_median_ci(ALPHA).1,
                EPSILON
            );
            let student_test = out.student_ln_test(mu0, alt_hyp, ALPHA);
            println!("out.student_test={student_test:?}");
            assert_eq!(exp_accepted_hyp, student_test.accepted());
        }
    }

    #[test]
    fn test_mean_panics_on_empty() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(std::iter::empty());
        let result = std::panic::catch_unwind(|| out.mean());
        assert!(result.is_err());
    }

    #[test]
    fn test_stdev_panics_on_empty() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(std::iter::empty());
        let result = std::panic::catch_unwind(|| out.stdev());
        assert!(result.is_err());
    }

    #[test]
    fn test_median_panics_on_empty() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(std::iter::empty());
        let result = std::panic::catch_unwind(|| out.median_r());
        assert!(result.is_err());
    }

    #[test]
    fn test_mean_ln_panics_on_empty() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(std::iter::empty());
        let result = std::panic::catch_unwind(|| out.mean_ln_r());
        assert!(result.is_err());
    }

    #[test]
    fn test_stdev_ln_panics_on_empty() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(std::iter::empty());
        let result = std::panic::catch_unwind(|| out.stdev_ln_r());
        assert!(result.is_err());
    }

    #[test]
    fn test_student_ln_t_panics_on_single() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter([FpSeconds::from_millis(1)].into_iter());
        let result = std::panic::catch_unwind(|| out.student_ln_t(0.0));
        assert!(result.is_err());
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
}
