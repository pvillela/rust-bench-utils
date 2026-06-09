//! Module defining the key data structure produced by [`crate::bench_run`].

use std::{fmt::Debug, time::Duration};

use crate::{
    BenchCfg, LatencyUnit, PanicIfNeeded, SummaryStats, Timing, multi, new_timing, summary_stats,
};
use basic_stats::{
    aok::Aok,
    core::{AltHyp, Ci, HypTestResult, PositionWrtCi, SampleMoments, sample_mean, sample_stdev},
    normal::{student_1samp_ci, student_1samp_p, student_1samp_t, student_1samp_test},
};

struct FlatIterator<I> {
    it: I,
    value_opt: Option<Duration>,
    count: u64,
}

impl<I: Iterator<Item = (Duration, u64)>> Iterator for FlatIterator<I> {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count > 0 {
            self.count -= 1;
            self.value_opt
        } else {
            match self.it.next() {
                Some((value, count)) => {
                    assert!(
                        count > 0,
                        "counts from iterator with counts must be positive"
                    );
                    self.count = count - 1;
                    self.value_opt = Some(value);
                    self.value_opt
                }
                None => None,
            }
        }
    }
}

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
    pub(crate) n_ln: u64,
    pub(crate) sum_ln: f64,
    pub(crate) sum2_ln: f64,
    panic_on_error: bool,
}

impl BenchOut {
    #[doc(hidden)]
    /// Creates a new empty instance based on `cfg`.
    pub fn new(cfg: &BenchCfg) -> Self {
        let hist = new_timing(20 * 1000 * 1000, cfg.sigfig());
        let sum = 0.;
        let sum2 = 0.;
        let n_ln = 0;
        let sum_ln = 0.;
        let sum2_ln = 0.;

        Self {
            recording_unit: cfg.recording_unit(),
            hist,
            sum,
            sum2,
            n_ln,
            sum_ln,
            sum2_ln,
            panic_on_error: cfg.panic_on_error(),
        }
    }

    #[doc(hidden)]
    /// Creates a [`BenchOut`] from a **finite** iterator of [`Duration`] values.
    ///
    /// Each item in the iterator is recorded as a single latency observation.
    /// The HDR histogram, running sums, and log-latency sums are updated accordingly.
    ///
    /// # Arguments
    ///
    /// - `cfg` - benchmark configuration (recording unit, significant figures, etc.).
    /// - `src` - source of elapsed-time measurements to ingest.
    ///
    /// ## May hang
    /// Hangs if th iterator is not finite.
    pub fn from_iter(cfg: &BenchCfg, src: impl Iterator<Item = Duration>) -> Self {
        let mut out = Self::new(cfg);
        for item in src {
            out.capture_data(item);
        }
        out
    }

    #[doc(hidden)]
    /// Creates a new empty instance.
    pub fn reset(&mut self) {
        self.hist.reset();
        self.sum = 0.;
        self.sum2 = 0.;
        self.n_ln = 0;
        self.sum_ln = 0.;
        self.sum2_ln = 0.
    }

    #[inline(always)]
    /// Updates `self` with an elapsed time observation for the target function.
    pub(crate) fn capture_data(&mut self, latency: Duration) {
        let elapsed = self.recording_unit.latency_as_u64(latency);
        self.hist
            .record(elapsed)
            .expect("can't happen: histogram is auto-resizable");

        self.sum += elapsed as f64;
        self.sum2 += elapsed.pow(2) as f64;

        if latency > Duration::ZERO {
            let ln = self.recording_unit.latency_as_f64(latency).ln();
            self.n_ln += 1;
            self.sum_ln += ln;
            self.sum2_ln += ln.powi(2);
        }
    }

    /// Returns all the latency data collected as an iterator of value-count pairs, where each value is a latency
    /// measurement and each count is the number of occurences of the latency measurment.
    ///
    /// The iterator yields values in strictly increasing order and all counts are positive.
    pub fn iter_with_counts(&self) -> impl Iterator<Item = (Duration, u64)> {
        self.hist.iter_recorded().map(|x| {
            let value = self.recording_unit.latency_from_u64(x.value_iterated_to());
            let count = x.count_at_value();
            (value, count)
        })
    }

    /// Returns all the latency data collected as an iterator of durations.
    ///
    /// The iterator yields values in monotonically non-decreasing order.
    pub fn iter_flat(&self) -> impl Iterator<Item = Duration> {
        FlatIterator {
            it: self.iter_with_counts(),
            value_opt: None,
            count: 0,
        }
    }

    /// Latency unit used in data collection.
    pub fn recording_unit(&self) -> LatencyUnit {
        self.recording_unit
    }

    /// The value of [`BenchCfg::panic_on_error`] at the time `self` was constructed.
    pub fn panic_on_error(&self) -> bool {
        self.panic_on_error
    }

    /// Number of observations (sample size) for a function, as an integer.
    #[inline(always)]
    pub fn n(&self) -> u64 {
        self.hist.len()
    }

    /// Number of observations (sample size) for a function, as a floating point number.
    #[inline(always)]
    pub fn nf(&self) -> f64 {
        self.hist.len() as f64
    }

    /// Summary descriptive statistics.
    ///
    /// Includes sample size, mean, standard deviation, median, several percentiles, min, and max.
    ///
    /// # Panics
    /// Panics if `self.panic_on_error() == true` **and** the number of observations is zero.
    pub fn summary(&self) -> SummaryStats {
        summary_stats(self)
    }

    /// Sample mean of latencies.
    ///
    /// # Panics
    /// Panics if `self.panic_on_error() == true` **and** the number of observations is zero.
    pub fn mean(&self) -> Duration {
        let mean_rec = sample_mean(self.n(), self.sum)
            .aok()
            .panic_if_needed(self.panic_on_error(), "number of observations is zero");
        self.recording_unit.latency_from_f64(mean_rec)
    }

    /// Sample standard deviation of latencies.
    ///
    /// # Panics
    /// Panics if `self.panic_on_error() == true` **and** the number of observations is zero.
    pub fn stdev(&self) -> Duration {
        let stdev_rec = sample_stdev(self.n(), self.sum, self.sum2)
            .aok()
            .panic_if_needed(self.panic_on_error(), "number of observations is zero");
        self.recording_unit.latency_from_f64(stdev_rec)
    }

    /// Sample median of latencies.
    ///
    /// # Panics
    /// Panics if `self.panic_on_error() == true` **and** the number of observations is zero.
    pub fn median(&self) -> Duration {
        self.summary().median
    }

    /// Sample mean of the natural logarithms of latencies (in the recording unit).
    ///
    /// # Panics
    /// Panics if `self.panic_on_error() == true` **and** the number of observations is zero.
    pub fn mean_ln(&self) -> f64 {
        sample_mean(self.n_ln, self.sum_ln)
            .aok()
            .panic_if_needed(self.panic_on_error(), "number of observations is zero")
    }

    /// Sample standard deviation of the natural logarithms of latencies.
    ///
    /// # Panics
    /// Panics if `self.panic_on_error() == true` **and** the number of observations is zero.
    pub fn stdev_ln(&self) -> f64 {
        sample_stdev(self.n_ln, self.sum_ln, self.sum2_ln)
            .aok()
            .panic_if_needed(self.panic_on_error(), "number of observations is zero")
    }

    /// Student's one-sample t statistic for
    /// the equality of `mean(ln(latency(f)))` and `ln_mu0` (where `ln` is the natural logarithm, in the recording unit),
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
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - `number of observations <= 1`.
    /// - `self.stdev_ln() == 0`.
    pub fn student_ln_t(&self, ln_mu0: f64) -> f64 {
        let moments = SampleMoments::new(self.n_ln, self.sum_ln, self.sum2_ln);
        student_1samp_t(&moments, ln_mu0).aok().panic_if_needed(
            self.panic_on_error(),
            "`number of observations <= 1` or `self.stdev_ln() == 0`",
        )
    }

    /// Degrees of freedom for Student's t statistic for `mean(ln(latency(f)))` (where `ln` is the natural logarithm,
    /// in the recording unit).
    ///
    /// Under the assumption that `latency(f)` is approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    /// Thus, this statistics equivalently pertains to `ln(median(latency(f)))`.
    pub fn student_ln_df(&self) -> f64 {
        self.n_ln as f64 - 1.
    }

    /// p-value of Student's one-sample t-test for
    /// the equality of `mean(ln(latency(f)))` and `ln_mu0` (where `ln` is the natural logarithm, in the recording unit),
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
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - Sample size <= 1.
    /// - `self.stdev_ln()` == 0.
    pub fn student_ln_p(&self, ln_mu0: f64, alt_hyp: AltHyp) -> f64 {
        let moments = SampleMoments::new(self.n_ln, self.sum_ln, self.sum2_ln);
        student_1samp_p(&moments, ln_mu0, alt_hyp)
            .aok()
            .panic_if_needed(
                self.panic_on_error(),
                "`number of observations <= 1` or `self.stdev_ln() == 0`",
            )
    }

    /// Student's one-sample confidence interval for
    /// `mean(ln(latency(f)))` (where `ln` is the natural logarithm, in the recording unit),
    /// with confidence level `(1 - alpha)`.
    ///
    /// Assumes that `latency(f)` is approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// # Panics
    ///
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - `Sample size <= 1`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn student_ln_ci(&self, alpha: f64) -> Ci {
        let moments = SampleMoments::new(self.n_ln, self.sum_ln, self.sum2_ln);
        student_1samp_ci(&moments, alpha).aok().panic_if_needed(
            self.panic_on_error(),
            "`number of observations <= 1` or `alpha` not in open interval `(0, 1)`",
        )
    }

    /// Student's one-sample confidence interval for
    /// `median(latency(f))`,
    /// with confidence level `(1 - alpha)`.
    ///
    /// The confidence interval is expressed as a pair of [`Duration`] representing the
    /// low and high ends of the interval.
    ///
    /// Assumes that `latency(f)` is approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// # Panics
    ///
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - `Sample size <= 1`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn student_median_ci(&self, alpha: f64) -> (Duration, Duration) {
        let Ci(log_low, log_high) = self.student_ln_ci(alpha);
        let low = self.recording_unit.latency_from_f64(log_low.exp());
        let high = self.recording_unit.latency_from_f64(log_high.exp());
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
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - `Sample size <= 1`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn student_value_position_wrt_median_ci(
        &self,
        value: Duration,
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
    /// `mean(ln(latency(f))) == ln_mu0` (where `ln` is the natural logarithm, in the recording unit), or equivalently,
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
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - `Sample size <= 1`.
    /// - `self.stdev_ln()` == 0.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn student_ln_test(&self, ln_mu0: f64, alt_hyp: AltHyp, alpha: f64) -> HypTestResult {
        let moments = SampleMoments::new(self.n_ln, self.sum_ln, self.sum2_ln);
        student_1samp_test(&moments, ln_mu0, alt_hyp, alpha).aok()
    }

    #[cfg(feature = "_bench_diff")]
    #[inline(always)]
    /// Reference to the raw HDR histogram. Gated by feature **"_bench_diff"**.
    pub fn hist(&self) -> &Timing {
        &self.hist
    }

    #[cfg(feature = "_bench_diff")]
    #[inline(always)]
    /// Raw sum of recorded latencies. Gated by feature **"_bench_diff"**.
    pub fn sum(&self) -> f64 {
        self.sum
    }

    #[cfg(feature = "_bench_diff")]
    #[inline(always)]
    /// Raw sum of squares of recorded latencies. Gated by feature **"_bench_diff"**.
    pub fn sum2(&self) -> f64 {
        self.sum2
    }

    #[cfg(feature = "_bench_diff")]
    #[inline(always)]
    /// Sample size for log-latencies. Gated by feature **"_bench_diff"**.
    pub fn n_ln(&self) -> u64 {
        self.n_ln
    }

    #[cfg(feature = "_bench_diff")]
    #[inline(always)]
    /// Raw sum of natural logarithms of latencies. Gated by feature **"_bench_diff"**.
    pub fn sum_ln(&self) -> f64 {
        self.sum_ln
    }

    #[cfg(feature = "_bench_diff")]
    #[inline(always)]
    /// Raw sum of squares of natural logarithms of latencies. Gated by feature **"_bench_diff"**.
    pub fn sum2_ln(&self) -> f64 {
        self.sum2_ln
    }
}

impl Debug for BenchOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("BenchOut {{ recording_unit={:?}, panic_on_error={}, sigfig={}, n={}, sum={}, sum2={}, n_ln={}, sum_ln={}, sum2_ln={}, summary={:?} }}",
            self.recording_unit,
            self.panic_on_error,
            self.hist.sigfig(),
            self.n(),
            self.sum,
            self.sum2,
            self.n_ln,
            self.sum_ln,
            self.sum2_ln,
            self.summary()))
    }
}

impl From<multi::BenchOut<1>> for BenchOut {
    fn from(value: multi::BenchOut<1>) -> Self {
        // Destructure the struct and unpack the single-element array
        let multi::BenchOut { arity: _, arr: [b] } = value;
        b
    }
}

#[cfg(test)]
#[cfg(feature = "_test")]
// cargo test --package bench_utils --lib --all-features -- bench_out::test --nocapture
mod test {
    use super::*;
    use crate::rel_approx_eq_dur;
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
    fn test_bench_out_descriptive_stats() {
        const EPSILON: f64 = 0.001;

        // in ln of microseconds
        let mu_micro = 8.;
        // in ln of nanoseconds (recording unit is Nano by default)
        let mu = mu_micro + (1000_f64).ln();
        let sigma = *LO_STDEV_LN;
        let samp_size = 20_000;

        let cfg = BenchCfg::default();
        let ru = cfg.recording_unit();

        let lognormal_samp = lognormal_samp(mu, sigma, samp_size).map(|x| ru.latency_from_f64(x));
        let out = BenchOut::from_iter(&cfg, lognormal_samp);

        assert_eq!(ru, LatencyUnit::Nano);
        assert_eq!(out.n() as usize, samp_size);
        assert_eq!(out.nf(), out.n() as f64);

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

        println!(
            "exp_mean={:?}, out.mean={:?}",
            ru.latency_from_f64(exp_mean),
            out.mean()
        );
        println!("exp_stdev={:?}, out.stdev={:?}", exp_stdev, out.stdev());
        println!(
            "exp_p1={:?}, summary.p1={:?}",
            ru.latency_from_f64(exp_p1),
            summary.p1
        );
        println!(
            "exp_p5={:?}, summary.p5={:?}",
            ru.latency_from_f64(exp_p5),
            summary.p5
        );
        println!(
            "exp_p10={:?}, summary.p10={:?}",
            ru.latency_from_f64(exp_p10),
            summary.p10
        );
        println!(
            "exp_p25={:?}, summary.p25={:?}",
            ru.latency_from_f64(exp_p25),
            summary.p25
        );
        println!(
            "exp_median={:?}, summary.median={:?}",
            ru.latency_from_f64(exp_median),
            summary.median
        );
        println!(
            "exp_p75={:?}, summary.p75={:?}",
            ru.latency_from_f64(exp_p75),
            summary.p75
        );
        println!(
            "exp_p90={:?}, summary.p90={:?}",
            ru.latency_from_f64(exp_p90),
            summary.p90
        );
        println!(
            "exp_p95={:?}, summary.p95={:?}",
            ru.latency_from_f64(exp_p95),
            summary.p95
        );
        println!(
            "exp_p99={:?}, summary.p99={:?}",
            ru.latency_from_f64(exp_p99),
            summary.p99
        );

        rel_approx_eq_dur!(ru.latency_from_f64(exp_mean), out.mean(), EPSILON);
        rel_approx_eq_dur!(ru.latency_from_f64(exp_stdev), out.stdev(), EPSILON);
        rel_approx_eq_dur!(ru.latency_from_f64(exp_median), out.median(), EPSILON);
        approx_eq!(exp_mean_ln, out.mean_ln(), EPSILON);
        approx_eq!(exp_stdev_ln, out.stdev_ln(), EPSILON);

        rel_approx_eq_dur!(ru.latency_from_f64(exp_mean), summary.mean, EPSILON);
        rel_approx_eq_dur!(ru.latency_from_f64(exp_stdev), summary.stdev, EPSILON);
        rel_approx_eq_dur!(ru.latency_from_f64(exp_p1), summary.p1, EPSILON);
        rel_approx_eq_dur!(ru.latency_from_f64(exp_p5), summary.p5, EPSILON);
        rel_approx_eq_dur!(ru.latency_from_f64(exp_p10), summary.p10, EPSILON);
        rel_approx_eq_dur!(ru.latency_from_f64(exp_p25), summary.p25, EPSILON);
        rel_approx_eq_dur!(ru.latency_from_f64(exp_median), summary.median, EPSILON);
        rel_approx_eq_dur!(ru.latency_from_f64(exp_p75), summary.p75, EPSILON);
        rel_approx_eq_dur!(ru.latency_from_f64(exp_p90), summary.p90, EPSILON);
        rel_approx_eq_dur!(ru.latency_from_f64(exp_p95), summary.p95, EPSILON);
        rel_approx_eq_dur!(ru.latency_from_f64(exp_p99), summary.p99, EPSILON);
    }

    #[test]
    fn test_bench_out_student() {
        const EPSILON: f64 = 0.001;

        // in ln of microseconds
        let mu_micro = 8.;
        // in ln of nanoseconds (recording unit is Nano by default)
        let mu = mu_micro + (1000_f64).ln();
        let sigma = *LO_STDEV_LN;
        let samp_size = 20_000;

        let cfg = BenchCfg::default();
        let ru = cfg.recording_unit();

        let lognormal_samp = lognormal_samp(mu, sigma, samp_size).map(|x| ru.latency_from_f64(x));
        let out = BenchOut::from_iter(&cfg, lognormal_samp);

        let normal_samp = normal_detm_samp(mu, sigma, samp_size).unwrap();
        let moments_ln = SampleMoments::from_iterator(normal_samp);

        assert_eq!(out.recording_unit(), LatencyUnit::Nano);
        assert_eq!(out.n() as usize, samp_size);
        assert_eq!(out.nf(), out.n() as f64);

        // The true median should lie inside the CI
        let true_median = ru.latency_from_f64(mu.exp());
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
            rel_approx_eq_dur!(
                ru.latency_from_f64(exp_ci_ns_low),
                out.student_median_ci(ALPHA).0,
                EPSILON
            );
            rel_approx_eq_dur!(
                ru.latency_from_f64(exp_ci_ns_high),
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
            rel_approx_eq_dur!(
                ru.latency_from_f64(exp_ci_ns_low),
                out.student_median_ci(ALPHA).0,
                EPSILON
            );
            rel_approx_eq_dur!(
                ru.latency_from_f64(exp_ci_ns_high),
                out.student_median_ci(ALPHA).1,
                EPSILON
            );
            let student_test = out.student_ln_test(mu0, alt_hyp, ALPHA);
            println!("out.student_test={student_test:?}");
            assert_eq!(exp_accepted_hyp, student_test.accepted());
        }
    }

    #[test]
    fn test_bench_out_mean_panics_on_empty() {
        let cfg = BenchCfg::default().with_panic_on_error(true);
        let out = BenchOut::from_iter(&cfg, std::iter::empty());
        let result = std::panic::catch_unwind(|| out.mean());
        assert!(result.is_err());
    }

    #[test]
    fn test_bench_out_stdev_panics_on_empty() {
        let cfg = BenchCfg::default().with_panic_on_error(true);
        let out = BenchOut::from_iter(&cfg, std::iter::empty());
        let result = std::panic::catch_unwind(|| out.stdev());
        assert!(result.is_err());
    }

    #[test]
    fn test_bench_out_median_panics_on_empty() {
        let cfg = BenchCfg::default().with_panic_on_error(true);
        let out = BenchOut::from_iter(&cfg, std::iter::empty());
        let result = std::panic::catch_unwind(|| out.median());
        assert!(result.is_err());
    }

    #[test]
    fn test_bench_out_mean_ln_panics_on_empty() {
        let cfg = BenchCfg::default().with_panic_on_error(true);
        let out = BenchOut::from_iter(&cfg, std::iter::empty());
        let result = std::panic::catch_unwind(|| out.mean_ln());
        assert!(result.is_err());
    }

    #[test]
    fn test_bench_out_stdev_ln_panics_on_empty() {
        let cfg = BenchCfg::default().with_panic_on_error(true);
        let out = BenchOut::from_iter(&cfg, std::iter::empty());
        let result = std::panic::catch_unwind(|| out.stdev_ln());
        assert!(result.is_err());
    }

    #[test]
    fn test_bench_out_student_ln_t_panics_on_single() {
        let cfg = BenchCfg::default().with_panic_on_error(true);
        let out = BenchOut::from_iter(&cfg, [Duration::from_millis(1)].into_iter());
        let result = std::panic::catch_unwind(|| out.student_ln_t(0.0));
        assert!(result.is_err());
    }

    #[test]
    fn test_bench_out_iter_with_counts() {
        const EPSILON: f64 = 0.001;
        let cfg = BenchCfg::default();
        let out = BenchOut::from_iter(
            &cfg,
            [
                Duration::from_millis(1),
                Duration::from_millis(1),
                Duration::from_millis(2),
            ]
            .into_iter(),
        );
        let pairs: Vec<_> = out.iter_with_counts().collect();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].1, 2);
        assert_eq!(pairs[1].1, 1);
        // HDR histogram has slight quantization; compare approximately
        rel_approx_eq_dur!(pairs[0].0, Duration::from_millis(1), EPSILON);
        rel_approx_eq_dur!(pairs[1].0, Duration::from_millis(2), EPSILON);
    }

    #[test]
    fn test_bench_out_iter_flat() {
        let cfg = BenchCfg::default();
        let out = BenchOut::from_iter(
            &cfg,
            [
                Duration::from_millis(1),
                Duration::from_millis(1),
                Duration::from_millis(2),
            ]
            .into_iter(),
        );
        assert_eq!(out.iter_flat().count(), 3);
    }

    #[test]
    fn test_bench_out_reset() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::from_iter(
            &cfg,
            [Duration::from_millis(1), Duration::from_millis(2)].into_iter(),
        );
        assert_eq!(out.n(), 2);
        out.reset();
        assert_eq!(out.n(), 0);
    }
}
