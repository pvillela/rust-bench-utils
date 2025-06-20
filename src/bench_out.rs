//! Module defining the key data structure produced by [`crate::bench_run`].

use crate::{LatencyUnit, SummaryStats, Timing, new_timing, summary_stats};
use basic_stats::{
    aok::{AokBasicStats, AokFloat},
    core::{AltHyp, Ci, HypTestResult, PositionWrtCi, SampleMoments, sample_mean, sample_stdev},
    normal::{student_1samp_ci, student_1samp_p, student_1samp_t, student_1samp_test},
};

/// Contains the data resulting from benchmarking a closure.
///
/// It is returned by the core benchmarking functions in this library.
/// Its methods provide descriptive statistics about the latency sample of the
/// benchmarked closure.
pub struct BenchOut {
    pub(super) unit: LatencyUnit,
    pub(super) hist: Timing,
    pub(super) sum: i64,
    pub(super) sum2: i64,
    pub(super) n_ln: u64,
    pub(super) sum_ln: f64,
    pub(super) sum2_ln: f64,
}

impl BenchOut {
    #[cfg(feature = "_friends_only")]
    /// Creates a new empty instance.
    pub fn new(unit: LatencyUnit) -> Self {
        let hist = new_timing(20 * 1000 * 1000, 5);
        let sum = 0;
        let sum2 = 0;
        let n_ln = 0;
        let sum_ln = 0.;
        let sum2_ln = 0.;

        Self {
            unit,
            hist,
            sum,
            sum2,
            n_ln,
            sum_ln,
            sum2_ln,
        }
    }

    #[cfg(feature = "_friends_only")]
    /// Creates a new empty instance.
    pub fn reset(&mut self) {
        self.hist.reset();
        self.sum = 0;
        self.sum2 = 0;
        self.n_ln = 0;
        self.sum_ln = 0.;
        self.sum2_ln = 0.
    }

    #[cfg(feature = "_friends_only")]
    /// Updates `self` with an elapsed time observation for the function.
    pub fn capture_data(&mut self, elapsed: u64) {
        self.hist
            .record(elapsed)
            .expect("can't happen: histogram is auto-resizable");

        self.sum += elapsed as i64;
        self.sum2 += elapsed.pow(2) as i64;

        if elapsed > 0 {
            let ln = (elapsed as f64).ln();
            self.n_ln += 1;
            self.sum_ln += ln;
            self.sum2_ln += ln.powi(2);
        }
    }

    /// Latency unit used in data collection.
    pub fn unit(&self) -> LatencyUnit {
        self.unit
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
    pub fn summary(&self) -> SummaryStats {
        summary_stats(self)
    }

    /// Mean of latencies.
    pub fn mean(&self) -> f64 {
        sample_mean(self.n(), self.sum as f64).aok()
    }

    /// Standard deviation of latencies.
    pub fn stdev(&self) -> f64 {
        sample_stdev(self.n(), self.sum as f64, self.sum2 as f64).aok()
    }

    /// Median of latencies.
    pub fn median(&self) -> f64 {
        self.summary().median as f64
    }

    /// Mean of the natural logarithms of latencies.
    pub fn mean_ln(&self) -> f64 {
        sample_mean(self.n_ln, self.sum_ln).aok()
    }

    /// Standard deviation of the natural logarithms latencies.
    pub fn stdev_ln(&self) -> f64 {
        sample_stdev(self.n_ln, self.sum_ln, self.sum2_ln).aok()
    }

    /// Student's one-sample t statistic for `mean(ln(latency(f)))` (where `ln` is the natural logarithm).
    pub fn student_ln_t(&self, ln_mu0: f64) -> f64 {
        let moments = SampleMoments::new(self.n_ln, self.sum_ln, self.sum2_ln);
        student_1samp_t(&moments, ln_mu0).aok()
    }

    /// Degrees of freedom for Student's t statistic for `mean(ln(latency(f)))`.
    pub fn student_ln_df(&self) -> f64 {
        self.n_ln as f64 - 1.
    }

    /// p-value of Student's one-sample t-test for equality of
    /// `median(latency(f))` and `med0`.
    ///
    /// Assumes that `latency(f)` is approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    pub fn student_median_p(&self, med0: f64, alt_hyp: AltHyp) -> f64 {
        let moments = SampleMoments::new(self.n_ln, self.sum_ln, self.sum2_ln);
        let mu0 = med0.ln();
        student_1samp_p(&moments, mu0, alt_hyp).aok()
    }

    /// Student's one-sample confidence interval for
    /// `mean(ln(latency(f)))` (where `ln` is the natural logarithm).
    /// with confidence level `(1 - alpha)`.
    ///
    /// Assumes that `latency(f)` is approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    pub fn student_ln_ci(&self, alpha: f64) -> Ci {
        let moments = SampleMoments::new(self.n_ln, self.sum_ln, self.sum2_ln);
        student_1samp_ci(&moments, alpha).aok()
    }

    /// Student's one-sample confidence interval for
    /// `median(latency(f))`,
    /// with confidence level `(1 - alpha)`.
    ///
    /// Assumes that `latency(f)` is approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    pub fn student_median_ci(&self, alpha: f64) -> Ci {
        let Ci(log_low, log_high) = self.student_ln_ci(alpha);
        let low = log_low.exp();
        let high = log_high.exp();
        Ci(low, high)
    }

    /// Position of `value` with respect to
    /// Student's one-sample confidence interval for
    /// `median(latency(f))`,
    /// with confidence level `(1 - alpha)`.
    ///
    /// Assumes that `latency(f)` is approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    pub fn student_value_position_wrt_median_ci(&self, value: f64, alpha: f64) -> PositionWrtCi {
        let ci = self.student_median_ci(alpha);
        ci.position_of(value)
    }

    /// Student's one-sample test of the hypothesis that
    /// `median(latency(f)) == med0`,
    /// with alternative hypothesis `alt_hyp` and confidence level `(1 - alpha)`.
    ///
    /// Assumes that `latency(f)` is approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    pub fn student_median_test(&self, med0: f64, alt_hyp: AltHyp, alpha: f64) -> HypTestResult {
        let moments = SampleMoments::new(self.n_ln, self.sum_ln, self.sum2_ln);
        let mu0 = med0.ln();
        student_1samp_test(&moments, mu0, alt_hyp, alpha).aok()
    }

    #[cfg(feature = "_bench_diff")]
    #[inline(always)]
    pub fn hist(&self) -> &Timing {
        &self.hist
    }

    #[cfg(feature = "_bench_diff")]
    #[inline(always)]
    pub fn sum(&self) -> i64 {
        self.sum
    }

    #[cfg(feature = "_bench_diff")]
    #[inline(always)]
    pub fn sum2(&self) -> i64 {
        self.sum2
    }

    #[cfg(feature = "_bench_diff")]
    #[inline(always)]
    pub fn n_ln(&self) -> u64 {
        self.n_ln
    }

    #[cfg(feature = "_bench_diff")]
    #[inline(always)]
    pub fn sum_ln(&self) -> f64 {
        self.sum_ln
    }

    #[cfg(feature = "_bench_diff")]
    #[inline(always)]
    pub fn sum2_ln(&self) -> f64 {
        self.sum2_ln
    }
}

#[cfg(test)]
#[cfg(feature = "_dev_utils")]
mod test {
    use super::*;
    use crate::test_support::{LO_STDEV_LN, lognormal_samp};
    use basic_stats::{
        approx_eq,
        core::AcceptedHyp,
        normal::{deterministic_normal_sample, student_1samp_df, student_1samp_p},
        rel_approx_eq,
    };
    use statrs::distribution::{ContinuousCDF, Normal};

    const ALPHA: f64 = 0.05;

    #[test]
    fn test_bench_out_descriptive_stats() {
        const EPSILON: f64 = 0.001;

        let mu = 8.;
        let sigma = *LO_STDEV_LN;
        let k = 100;

        let lognormal_samp = lognormal_samp(mu, sigma, k);
        let mut out = BenchOut::new(LatencyUnit::Micro);
        out.collect_data(lognormal_samp);

        assert_eq!(out.unit(), LatencyUnit::Micro);
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

        let summary = out.summary();
        println!("exp_mean={}, summary.mean={}", exp_mean, summary.mean);
        println!("exp_stdev={}, summary.stdev={}", exp_stdev, summary.stdev);
        println!("exp_p1={}, summary.p1={}", exp_p1, summary.p1);
        println!("exp_p5={}, summary.p5={}", exp_p5, summary.p5);
        println!("exp_p10={}, summary.p10={}", exp_p10, summary.p10);
        println!("exp_p25={}, summary.p25={}", exp_p25, summary.p25);
        println!(
            "exp_median={}, summary.median={}",
            exp_median, summary.median
        );
        println!("exp_p75={}, summary.p75={}", exp_p75, summary.p75);
        println!("exp_p90={}, summary.p90={}", exp_p90, summary.p90);
        println!("exp_p95={}, summary.p95={}", exp_p95, summary.p95);
        println!("exp_p99={}, summary.p99={}", exp_p99, summary.p99);

        rel_approx_eq!(exp_mean, out.mean(), EPSILON);
        rel_approx_eq!(exp_stdev, out.stdev(), EPSILON);
        rel_approx_eq!(exp_median, out.median(), EPSILON);
        approx_eq!(exp_mean_ln, out.mean_ln(), EPSILON);
        approx_eq!(exp_stdev_ln, out.stdev_ln(), EPSILON);

        rel_approx_eq!(exp_mean, summary.mean, EPSILON);
        rel_approx_eq!(exp_stdev, summary.stdev, EPSILON);
        rel_approx_eq!(exp_p1, summary.p1 as f64, EPSILON);
        rel_approx_eq!(exp_p5, summary.p5 as f64, EPSILON);
        rel_approx_eq!(exp_p10, summary.p10 as f64, EPSILON);
        rel_approx_eq!(exp_p25, summary.p25 as f64, EPSILON);
        rel_approx_eq!(exp_median, summary.median as f64, EPSILON);
        rel_approx_eq!(exp_p75, summary.p75 as f64, EPSILON);
        rel_approx_eq!(exp_p90, summary.p90 as f64, EPSILON);
        rel_approx_eq!(exp_p95, summary.p95 as f64, EPSILON);
        rel_approx_eq!(exp_p99, summary.p99 as f64, EPSILON);
    }

    #[test]
    fn test_bench_out_student() {
        const EPSILON: f64 = 0.001;

        let mu = 14.; // = ln(442413.392), high enough to mitigate impact of f64 to u64 coercion
        let sigma = *LO_STDEV_LN;
        let k = 100;

        let normal_samp = deterministic_normal_sample(mu, sigma, k).unwrap();
        let moments_ln = SampleMoments::from_iterator(normal_samp);

        let lognormal_samp = lognormal_samp(mu, sigma, k);
        let mut out = BenchOut::new(LatencyUnit::Micro);
        out.collect_data(lognormal_samp);

        assert_eq!(out.unit(), LatencyUnit::Micro);
        assert_eq!(out.n(), 2 * k * k - 1);
        assert_eq!(out.nf(), out.n() as f64);

        {
            let ratio_medians: f64 = 1.0;
            let mu0 = mu - ratio_medians.ln();
            let median0 = mu0.exp();
            let alt_hyp = AltHyp::Ne;
            let exp_accepted_hyp = AcceptedHyp::Null;

            let exp_t = student_1samp_t(&moments_ln, mu0).unwrap();
            let exp_df = student_1samp_df(&moments_ln).unwrap();
            let exp_p = student_1samp_p(&moments_ln, mu0, alt_hyp).unwrap();
            let exp_ln_ci = student_1samp_ci(&moments_ln, ALPHA).unwrap();
            let exp_ci = Ci(exp_ln_ci.0.exp(), exp_ln_ci.1.exp());

            rel_approx_eq!(exp_t, out.student_ln_t(mu0), EPSILON); // doesn't pass
            approx_eq!(exp_df, out.student_ln_df(), EPSILON);
            rel_approx_eq!(exp_p, out.student_median_p(median0, alt_hyp), EPSILON);
            rel_approx_eq!(exp_ci.0, out.student_median_ci(ALPHA).0, EPSILON);
            rel_approx_eq!(exp_ci.1, out.student_median_ci(ALPHA).1, EPSILON);
            let student_test = out.student_median_test(median0, alt_hyp, ALPHA);
            println!("out.student_test={student_test:?}");
            assert_eq!(exp_accepted_hyp, student_test.accepted());
        }

        {
            let ratio_medians: f64 = 1.01;
            let mu0 = mu - ratio_medians.ln();
            let median0 = mu0.exp();
            let alt_hyp = AltHyp::Gt;
            let exp_accepted_hyp = AcceptedHyp::Alt;

            let exp_t = student_1samp_t(&moments_ln, mu0).unwrap();
            let exp_df = student_1samp_df(&moments_ln).unwrap();
            let exp_p = student_1samp_p(&moments_ln, mu0, alt_hyp).unwrap();
            let exp_ln_ci = student_1samp_ci(&moments_ln, ALPHA).unwrap();
            let exp_ci = Ci(exp_ln_ci.0.exp(), exp_ln_ci.1.exp());

            rel_approx_eq!(exp_t, out.student_ln_t(mu0), EPSILON); // doesn't pass
            approx_eq!(exp_df, out.student_ln_df(), EPSILON);
            rel_approx_eq!(exp_p, out.student_median_p(median0, alt_hyp), EPSILON);
            rel_approx_eq!(exp_ci.0, out.student_median_ci(ALPHA).0, EPSILON);
            rel_approx_eq!(exp_ci.1, out.student_median_ci(ALPHA).1, EPSILON);
            let student_test = out.student_median_test(median0, alt_hyp, ALPHA);
            println!("out.student_test={student_test:?}");
            assert_eq!(exp_accepted_hyp, student_test.accepted());
        }
    }
}
