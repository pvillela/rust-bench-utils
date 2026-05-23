//! Module defining the key data structure produced by [`crate::bench_run`].

use crate::{BenchCfg, LatencyUnit, SummaryStats, summary_stats};
use basic_stats::core::{AltHyp, Ci, HypTestResult, PositionWrtCi};
use std::{
    array,
    fmt::Debug,
    ops::{Deref, Index},
};

/// Contains the data resulting from benchmarking a closure.
///
/// It is returned by the core benchmarking functions in this library.
/// Its methods provide descriptive and inferential statistics about the latency sample of the
/// benchmarked closure.
///
/// The `*_ln_*` methods provide statistics for `mean(ln(latency(f)))`, where `ln` is the natural logarithm.
/// Under the assumption that `latency(f)` is approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
/// This assumption is widely supported by performance analysis theory and empirical data.
/// Thus, the `*_ln_*` methods are useful for the analysis of median latencies.
#[derive(Debug)]
pub struct BenchOut<const K: usize> {
    pub(crate) arity: usize,
    pub(crate) arr: [crate::BenchOut; K],
}

impl<const K: usize> Index<usize> for BenchOut<K> {
    type Output = crate::BenchOut;

    fn index(&self, index: usize) -> &Self::Output {
        &self.arr[index]
    }
}

impl Deref for BenchOut<1> {
    type Target = crate::BenchOut;

    fn deref(&self) -> &Self::Target {
        &self.arr[0]
    }
}

impl<const K: usize> BenchOut<K> {
    #[doc(hidden)]
    pub fn new(cfg: &BenchCfg) -> Self {
        Self {
            arity: K,
            arr: array::from_fn(|_| crate::BenchOut::new(cfg)),
        }
    }

    pub fn arity(&self) -> usize {
        self.arity
    }

    pub(crate) fn first(&self) -> &crate::BenchOut {
        &self.arr[0]
    }

    #[doc(hidden)]
    /// Creates a new empty instance.
    pub fn reset(&mut self) {
        for b in &mut self.arr {
            b.reset();
        }
    }

    #[doc(hidden)]
    /// Updates `self` with an elapsed time observation for the functions.
    pub fn capture_data(&mut self, elapsed: [u64; K]) {
        for (i, b) in &mut self.arr.iter_mut().enumerate() {
            b.capture_data(elapsed[i]);
        }
    }

    /// Latency unit used in data collection.
    pub fn recording_unit(&self) -> LatencyUnit {
        self.first().recording_unit()
    }

    /// Latency unit used for reporting benchmark results.
    pub fn reporting_unit(&self) -> LatencyUnit {
        self.first().reporting_unit()
    }

    /// The value of [`BenchCfg::panic_on_error`] at the time `self` was constructed.
    pub fn panic_on_error(&self) -> bool {
        self.first().panic_on_error()
    }

    /// Number of observations (sample size) for a function, as an integer.
    #[inline(always)]
    pub fn n(&self) -> u64 {
        self.first().n()
    }

    /// Number of observations (sample size) for a function, as a floating point number.
    #[inline(always)]
    pub fn nf(&self) -> f64 {
        self.first().nf()
    }

    /// Summary descriptive statistics.
    ///
    /// Includes sample size, mean, standard deviation, median, several percentiles, min, and max.
    pub fn summaries(&self) -> [SummaryStats; K] {
        array::from_fn(|k| summary_stats(&self.arr[k]))
    }

    /// Sample means of latencies.
    ///
    /// # Panics
    /// Panics if `self.panic_on_error() == true` **and** the number of observations is zero.
    pub fn means(&self) -> [f64; K] {
        array::from_fn(|k| self.arr[k].mean())
    }

    /// Sample standard deviations of latencies.
    ///
    /// # Panics
    /// Panics if `self.panic_on_error() == true` **and** the number of observations is zero.
    pub fn stdevs(&self) -> [f64; K] {
        array::from_fn(|k| self.arr[k].stdev())
    }

    /// Sample medians of latencies.
    pub fn medians(&self) -> [f64; K] {
        array::from_fn(|k| self.arr[k].median())
    }

    /// Sample means of the natural logarithms of latencies.
    ///
    /// # Panics
    /// Panics if `self.panic_on_error() == true` **and** the number of observations is zero.
    pub fn mean_lns(&self) -> [f64; K] {
        array::from_fn(|k| self.arr[k].mean_ln())
    }

    /// Sample standard deviations of the natural logarithms of latencies.
    ///
    /// # Panics
    /// Panics if `self.panic_on_error() == true` **and** the number of observations is zero.
    pub fn stdev_lns(&self) -> [f64; K] {
        array::from_fn(|k| self.arr[k].stdev_ln())
    }

    /// Student's one-sample t statistics for
    /// the equality of `mean(ln(latency(f)))` and `ln_mu0` (where `ln` is the natural logarithm), or equivalently,
    /// the equality of `median(latency(f))` and `exp(ln_mu0)`.
    ///
    /// Under the assumption that `latency(f)` is approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// Arguments:
    /// - `ln_mu0`: hypothesized `mean(ln(latency(f)))`, or equivalently, `ln(median(latency(f)))`.
    ///
    /// # Panics
    ///
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - `number of observations <= 1`.
    /// - `self.stdev_ln() == 0`.
    pub fn student_ln_ts(&self, ln_mu0: f64) -> [f64; K] {
        array::from_fn(|k| self.arr[k].student_ln_t(ln_mu0))
    }

    /// Degrees of freedom for Student's t statistics for `mean(ln(latency(f)))` (where `ln` is the natural logarithm).
    ///
    /// Under the assumption that `latency(f)` is approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    /// Thus, this statistics equivalently pertains to `ln(median(latency(f)))`.
    pub fn student_ln_dfs(&self) -> [f64; K] {
        array::from_fn(|k| self.arr[k].student_ln_df())
    }

    /// p-values of Student's one-sample t-tests for
    /// the equality of `mean(ln(latency(f)))` and `ln_mu0` (where `ln` is the natural logarithm), or equivalently,
    /// the equality of `median(latency(f))` and `exp(ln_mu0)`.
    ///
    /// Under the assumption that `latency(f)` is approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// Arguments:
    /// - `ln_mu0`: hypothesized `mean(ln(latency(f)))`, or equivalently, `ln(median(latency(f)))`.
    ///
    /// # Panics
    ///
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - Sample size <= 1.
    /// - `self.stdev_ln()` == 0.
    pub fn student_ln_ps(&self, ln_mu0: f64, alt_hyp: AltHyp) -> [f64; K] {
        array::from_fn(|k| self.arr[k].student_ln_p(ln_mu0, alt_hyp))
    }

    /// Student's one-sample confidence intervals for
    /// `mean(ln(latency(f)))` (where `ln` is the natural logarithm).
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
    pub fn student_ln_cis(&self, alpha: f64) -> [Ci; K] {
        array::from_fn(|k| self.arr[k].student_ln_ci(alpha))
    }

    /// Student's one-sample confidence intervals for
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
    pub fn student_median_cis(&self, alpha: f64) -> [Ci; K] {
        array::from_fn(|k| self.arr[k].student_median_ci(alpha))
    }

    /// Positions of `value` with respect to
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
    pub fn student_value_position_wrt_median_cis(
        &self,
        value: f64,
        alpha: f64,
    ) -> [PositionWrtCi; K] {
        array::from_fn(|k| self.arr[k].student_value_position_wrt_median_ci(value, alpha))
    }

    /// Student's one-sample tests of the hypotheses that
    /// `mean(ln(latency(f))) == ln_mu0` (where `ln` is the natural logarithm), or equivalently,
    /// `median(latency(f)) == exp(ln_mu0)`.
    ///
    /// Under the assumption that `latency(f)` is approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// Arguments:
    /// - `ln_mu0`: hypothesized `mean(ln(latency(f)))`, or equivalently, `ln(median(latency(f)))`.
    /// - `alt_hyp`: alternative hypothesis.
    /// - `alpha`: confidence level is `1 - alpha`.
    ///
    /// # Panics
    ///
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - `Sample size <= 1`.
    /// - `self.stdev_ln()` == 0.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn student_ln_tests(&self, ln_mu0: f64, alt_hyp: AltHyp, alpha: f64) -> [HypTestResult; K] {
        array::from_fn(|k| self.arr[k].student_ln_test(ln_mu0, alt_hyp, alpha))
    }
}

#[cfg(test)]
#[cfg(feature = "_test")]
mod test {
    use super::*;
    use crate::{
        BenchCfg,
        test_support::{LO_STDEV_LN, lognormal_samp},
    };
    use basic_stats::{
        approx_eq,
        core::{AcceptedHyp, PositionWrtCi, SampleMoments},
        normal::{
            normal_detm_samp, student_1samp_ci, student_1samp_df, student_1samp_p, student_1samp_t,
        },
        rel_approx_eq,
    };
    use statrs::distribution::{ContinuousCDF, Normal};

    const ALPHA: f64 = 0.05;

    #[test]
    fn test_bench_out_descriptive_stats() {
        const EPSILON: f64 = 0.001;

        let mu = 8.; // in ln of microseconds
        let sigma = *LO_STDEV_LN;
        let k = 100;

        let conv_factor = BenchCfg::default().conversion_factor();
        println!("conv_factor={conv_factor}");
        let rec_mu = mu - conv_factor.ln(); // in ln of nanoseconds

        let mut out = BenchOut::<2>::new(&BenchCfg::default());
        out.collect_data(array::from_fn(|_| lognormal_samp(rec_mu, sigma, k)));

        assert_eq!(out.recording_unit(), LatencyUnit::Nano);
        assert_eq!(out.reporting_unit(), LatencyUnit::Micro);
        assert_eq!(out.n(), 2 * k * k - 1);
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

        let summaries = out.summaries();

        println!("exp_mean={}, out.means={:?}", exp_mean, out.means());
        println!("exp_stdev={}, out.stdevs={:?}", exp_stdev, out.stdevs());
        println!(
            "exp_p1={}, summaries.p1={:?}",
            exp_p1,
            summaries.iter().map(|s| s.p1)
        );
        println!(
            "exp_p5={}, summaries.p5={:?}",
            exp_p5,
            summaries.iter().map(|s| s.p5)
        );
        println!(
            "exp_p10={}, summaries.p10={:?}",
            exp_p10,
            summaries.iter().map(|s| s.p10)
        );
        println!(
            "exp_p25={}, summaries.p25={:?}",
            exp_p25,
            summaries.iter().map(|s| s.p25)
        );
        println!(
            "exp_median={}, summaries.median={:?}",
            exp_median,
            summaries.iter().map(|s| s.median)
        );
        println!(
            "exp_p75={}, summaries.p75={:?}",
            exp_p75,
            summaries.iter().map(|s| s.p75)
        );
        println!(
            "exp_p90={}, summaries.p90={:?}",
            exp_p90,
            summaries.iter().map(|s| s.p90)
        );
        println!(
            "exp_p95={}, summaries.p95={:?}",
            exp_p95,
            summaries.iter().map(|s| s.p95)
        );
        println!(
            "exp_p99={}, summaries.p99={:?}",
            exp_p99,
            summaries.iter().map(|s| s.p99)
        );

        for k in 0..out.arity() {
            rel_approx_eq!(exp_mean, out[k].mean(), EPSILON);
            rel_approx_eq!(exp_stdev, out[k].stdev(), EPSILON);
            rel_approx_eq!(exp_median, out[k].median(), EPSILON);
            approx_eq!(exp_mean_ln, out[k].mean_ln(), EPSILON);
            approx_eq!(exp_stdev_ln, out[k].stdev_ln(), EPSILON);

            rel_approx_eq!(exp_mean, summaries[k].mean, EPSILON);
            rel_approx_eq!(exp_stdev, summaries[k].stdev, EPSILON);
            rel_approx_eq!(exp_p1, summaries[k].p1, EPSILON);
            rel_approx_eq!(exp_p5, summaries[k].p5, EPSILON);
            rel_approx_eq!(exp_p10, summaries[k].p10, EPSILON);
            rel_approx_eq!(exp_p25, summaries[k].p25, EPSILON);
            rel_approx_eq!(exp_median, summaries[k].median, EPSILON);
            rel_approx_eq!(exp_p75, summaries[k].p75, EPSILON);
            rel_approx_eq!(exp_p90, summaries[k].p90, EPSILON);
            rel_approx_eq!(exp_p95, summaries[k].p95, EPSILON);
            rel_approx_eq!(exp_p99, summaries[k].p99, EPSILON);
        }
    }

    #[test]
    fn test_bench_out_student() {
        const EPSILON: f64 = 0.001;

        let mu = 8.; // in ln of microseconds
        let sigma = *LO_STDEV_LN;
        let k = 100;

        let conv_factor = BenchCfg::default().conversion_factor();
        println!("conv_factor={conv_factor}");
        let rec_mu = mu - conv_factor.ln(); // in ln of nanoseconds

        let mut out = BenchOut::<2>::new(&BenchCfg::default());
        out.collect_data(array::from_fn(|_| lognormal_samp(rec_mu, sigma, k)));

        let normal_samp = normal_detm_samp(mu, sigma, k).unwrap();
        let moments_ln = SampleMoments::from_iterator(normal_samp);

        assert_eq!(out.recording_unit(), LatencyUnit::Nano);
        assert_eq!(out.reporting_unit(), LatencyUnit::Micro);
        assert_eq!(out.n(), 2 * k * k - 1);
        assert_eq!(out.nf(), out.n() as f64);

        // The true median (exp(mu)) should lie inside the CI
        let true_median = mu.exp();
        let positions = out.student_value_position_wrt_median_cis(true_median, ALPHA);
        assert_eq!(positions, array::from_fn(|_| PositionWrtCi::In));

        {
            let ratio_medians: f64 = 1.0;
            let mu0 = mu - ratio_medians.ln();
            let alt_hyp = AltHyp::Ne;
            let exp_accepted_hyp = AcceptedHyp::Null;

            let exp_t = student_1samp_t(&moments_ln, mu0).unwrap();
            let exp_df = student_1samp_df(&moments_ln).unwrap();
            let exp_p = student_1samp_p(&moments_ln, mu0, alt_hyp).unwrap();
            let exp_ln_ci = student_1samp_ci(&moments_ln, ALPHA).unwrap();
            let exp_ci = Ci(exp_ln_ci.0.exp(), exp_ln_ci.1.exp());

            for k in 0..out.arity() {
                approx_eq!(exp_t, out[k].student_ln_t(mu0), EPSILON);
                approx_eq!(exp_df, out[k].student_ln_df(), EPSILON);
                rel_approx_eq!(exp_p, out[k].student_ln_p(mu0, alt_hyp), EPSILON);
                rel_approx_eq!(exp_ci.0, out[k].student_median_ci(ALPHA).0, EPSILON);
                rel_approx_eq!(exp_ci.1, out[k].student_median_ci(ALPHA).1, EPSILON);
                let student_test = out[k].student_ln_test(mu0, alt_hyp, ALPHA);
                println!("out[k].student_test={student_test:?}");
                assert_eq!(exp_accepted_hyp, student_test.accepted());
            }
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
            let exp_ci = Ci(exp_ln_ci.0.exp(), exp_ln_ci.1.exp());

            for k in 0..out.arity() {
                rel_approx_eq!(exp_t, out[k].student_ln_t(mu0), EPSILON);
                approx_eq!(exp_df, out[k].student_ln_df(), EPSILON);
                approx_eq!(exp_p, out[k].student_ln_p(mu0, alt_hyp), EPSILON);
                rel_approx_eq!(exp_ci.0, out[k].student_median_ci(ALPHA).0, EPSILON);
                rel_approx_eq!(exp_ci.1, out[k].student_median_ci(ALPHA).1, EPSILON);
                let student_test = out[k].student_ln_test(mu0, alt_hyp, ALPHA);
                println!("out.student_test={student_test:?}");
                assert_eq!(exp_accepted_hyp, student_test.accepted());
            }
        }
    }

    #[test]
    fn test_deref() {
        let cfg = &BenchCfg::default()
            .with_recording_unit(LatencyUnit::Nano)
            .with_reporting_unit(LatencyUnit::Micro);
        let mut out1 = BenchOut::<1>::new(cfg);

        out1.capture_data([5]);
        out1.capture_data([7]);

        assert_eq!(out1.mean(), 0.006);
    }
}
