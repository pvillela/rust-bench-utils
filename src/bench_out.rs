//! Module defining the key data structure produced by [`crate::bench_run`].

use crate::{
    BenchCfg, FpSeconds, LatencyUnit, SummaryStats, Timing, multi, new_timing, summary_stats,
};
use basic_stats::{
    core::{AltHyp, Ci, HypTestResult, PositionWrtCi, SampleMoments, sample_mean, sample_stdev},
    normal::{student_1samp_ci, student_1samp_p, student_1samp_t, student_1samp_test},
};
use std::{fmt::Debug, iter};

/// Contains the latency observations resulting from benchmarking a closure.
///
/// It is returned by the core benchmarking functions in this library.
/// Its methods provide access to the raw data sample collected for the benchmarked closure, as well as descriptive and
/// inferential statistics.
///
/// The `*_ln_*` methods provide statistics for `mean(ln(latency(f)))`, where `ln` is the natural logarithm.
/// Under the assumption that `latency(f)` is approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
/// This assumption is widely supported by performance analysis theory and empirical data.
/// Thus, the `*_ln_*` methods are useful for the analysis of median latencies.
pub struct BenchOut {
    pub(crate) recording_unit: LatencyUnit,
    pub(crate) hist: Timing,
    pub(crate) sum: f64,
    pub(crate) sum2: f64,
    pub(crate) n_nz: u64,
    pub(crate) sum_ln: f64,
    pub(crate) sum2_ln: f64,
    pub(crate) batch: Option<usize>,
}

impl BenchOut {
    #[doc(hidden)]
    /// Creates a new empty instance based on `cfg`.
    pub fn new(cfg: &BenchCfg, batch: Option<usize>) -> Self {
        let hist = new_timing(20 * 1000 * 1000, cfg.sigfig());
        let sum = 0.;
        let sum2 = 0.;
        let n_nz = 0;
        let sum_ln = 0.;
        let sum2_ln = 0.;
        let batch = batch.map(|n| n.max(1));

        Self {
            recording_unit: cfg.recording_unit(),
            hist,
            sum,
            sum2,
            n_nz,
            sum_ln,
            sum2_ln,
            batch,
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

    #[doc(hidden)]
    /// Creates a new empty instance.
    pub fn reset(&mut self) {
        self.hist.reset();
        self.sum = 0.;
        self.sum2 = 0.;
        self.n_nz = 0;
        self.sum_ln = 0.;
        self.sum2_ln = 0.
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
    pub(crate) fn batch_size(&self) -> usize {
        self.batch.unwrap_or(1).max(1)
    }

    /// Number of (batched) latency data items recorded.
    #[inline(always)]
    pub fn n(&self) -> u64 {
        self.hist.len()
    }

    /// Total number of function executions taking into account batching.
    #[inline(always)]
    pub fn executions(&self) -> u64 {
        self.n() * self.batch_size() as u64
    }

    /// Summary descriptive statistics.
    ///
    /// Includes sample size, mean, standard deviation, median, several percentiles, min, and max.
    ///
    /// # Panics
    /// Panics if the number of observations is zero.
    pub fn summary(&self) -> SummaryStats {
        summary_stats(self)
    }

    /// Sample mean of latencies.
    ///
    /// # Panics
    /// Panics if the number of observations is zero.
    pub fn mean(&self) -> FpSeconds {
        let mean_rec = sample_mean(self.n(), self.sum).expect("number of observations is zero");
        mean_rec.into()
    }

    /// Sample standard deviation of latencies.
    ///
    /// # Panics
    /// Panics if the number of observations is zero.
    pub fn stdev(&self) -> FpSeconds {
        let stdev_rec =
            sample_stdev(self.n(), self.sum, self.sum2).expect("number of observations is zero");
        stdev_rec.into()
    }

    /// Sample median of latencies.
    ///
    /// # Panics
    /// Panics if the number of observations is zero.
    pub fn median(&self) -> FpSeconds {
        self.summary().median
    }

    /// Sample mean of the natural logarithms of latency [`FpSeconds`].
    ///
    /// # Panics
    /// Panics if the number of non-zero observations is zero.
    pub fn mean_ln(&self) -> f64 {
        sample_mean(self.n_nz, self.sum_ln).expect("number of non-zero observations is zero")
    }

    /// Sample standard deviation of the natural logarithms of latency [`FpSeconds`].
    ///
    /// # Panics
    /// Panics if the number of non-zero observations is zero.
    pub fn stdev_ln(&self) -> f64 {
        sample_stdev(self.n_nz, self.sum_ln, self.sum2_ln)
            .expect("number of non-zero observations is zero")
    }

    /// Student's one-sample t statistic for
    /// the equality of `mean(ln(latency(f)))` and `ln_mu0` (where `ln` is the natural logarithm in [`FpSeconds`]),
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
    /// - `number of non-zero observations <= 1`.
    /// - `self.stdev_ln() == 0`.
    pub fn student_ln_t(&self, ln_mu0: f64) -> f64 {
        let moments = SampleMoments::new(self.n_nz, self.sum_ln, self.sum2_ln);
        student_1samp_t(&moments, ln_mu0)
            .expect("`number of non-zero observations <= 1` or `self.stdev_ln() == 0`")
    }

    /// Degrees of freedom for Student's t statistic for `mean(ln(latency(f)))` (where `ln` is the natural logarithm,
    /// in [`FpSeconds`]).
    ///
    /// Under the assumption that `latency(f)` is approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    /// Thus, this statistics equivalently pertains to `ln(median(latency(f)))`.
    pub fn student_ln_df(&self) -> f64 {
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
    pub fn hist(&self) -> &Timing {
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
            self.n(),
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
        assert_eq!(out.n() as usize, samp_size);

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
            exp_median, summary.median
        );
        println!("exp_p75={:?}, summary.p75={:?}", exp_p75, summary.p75);
        println!("exp_p90={:?}, summary.p90={:?}", exp_p90, summary.p90);
        println!("exp_p95={:?}, summary.p95={:?}", exp_p95, summary.p95);
        println!("exp_p99={:?}, summary.p99={:?}", exp_p99, summary.p99);

        rel_approx_eq!(exp_mean, out.mean().0, EPSILON);
        rel_approx_eq!(exp_stdev, out.stdev().0, EPSILON);
        rel_approx_eq!(exp_median, out.median().as_f64(), EPSILON);
        approx_eq!(exp_mean_ln, out.mean_ln(), EPSILON);
        approx_eq!(exp_stdev_ln, out.stdev_ln(), EPSILON);

        rel_approx_eq!(exp_mean, summary.mean.0, EPSILON);
        rel_approx_eq!(exp_stdev, summary.stdev.0, EPSILON);
        rel_approx_eq!(exp_p1, summary.p1.as_f64(), EPSILON);
        rel_approx_eq!(exp_p5, summary.p5.as_f64(), EPSILON);
        rel_approx_eq!(exp_p10, summary.p10.as_f64(), EPSILON);
        rel_approx_eq!(exp_p25, summary.p25.as_f64(), EPSILON);
        rel_approx_eq!(exp_median, summary.median.as_f64(), EPSILON);
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
        assert_eq!(out.n() as usize, samp_size);

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
        let result = std::panic::catch_unwind(|| out.median());
        assert!(result.is_err());
    }

    #[test]
    fn test_mean_ln_panics_on_empty() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(std::iter::empty());
        let result = std::panic::catch_unwind(|| out.mean_ln());
        assert!(result.is_err());
    }

    #[test]
    fn test_stdev_ln_panics_on_empty() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(std::iter::empty());
        let result = std::panic::catch_unwind(|| out.stdev_ln());
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
        assert_eq!(out.n(), 2);
        out.reset();
        assert_eq!(out.n(), 0);
    }
}
