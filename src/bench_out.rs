//! Module defining the key data structure produced by [`crate::bench_one`].

use crate::{LatencyUnit, SummaryStats, Timing, new_timing, summary_stats};
use basic_stats::{
    aok::AokFloat,
    core::{sample_mean, sample_stdev},
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
        let sum_ln = 0.;
        let sum2_ln = 0.;

        Self {
            unit,
            hist,
            sum,
            sum2,
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
        self.sum_ln = 0.;
        self.sum2_ln = 0.
    }

    #[cfg(feature = "_friends_only")]
    /// Updates `self` with an elapsed time observation for the function.
    pub fn capture_data(&mut self, elapsed: u64) {
        self.hist
            .record(elapsed)
            .expect("can't happen: histogram is auto-resizable");

        assert!(elapsed > 0, "latency must be > 0");
        self.sum += elapsed as i64;
        self.sum2 += elapsed.pow(2) as i64;
        let ln = (elapsed as f64).ln();
        self.sum_ln += ln;
        self.sum2_ln += ln.powi(2);
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
        summary_stats(&self)
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
        sample_mean(self.n(), self.sum_ln).aok()
    }

    /// Standard deviation of the natural logarithms latecies.
    pub fn stdev_ln(&self) -> f64 {
        sample_stdev(self.n(), self.sum_ln, self.sum2_ln).aok()
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
    use basic_stats::{dev_utils::ApproxEq, normal::deterministic_normal_sample};
    use statrs::distribution::{ContinuousCDF, Normal};

    impl BenchOut {
        fn collect_data(&mut self, mut src: impl Iterator<Item = u64>) {
            while let Some(item) = src.next() {
                self.capture_data(item);
            }
        }
    }

    const EPSILON: f64 = 0.001;

    #[test]
    fn test() {
        let mu = 8.;
        let sigma = 1.;
        let k = 1000;

        let normal_samp = deterministic_normal_sample(mu, sigma, k).unwrap();
        let lognormal_samp = normal_samp.map(|x| x.exp().max(1.) as u64);
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

        assert!(
            exp_mean.rel_approx_eq(out.mean(), EPSILON),
            "exp_mean={exp_mean}, mean={}",
            out.mean()
        );
        assert!(
            exp_stdev.rel_approx_eq(out.stdev(), EPSILON),
            "exp_stdev={exp_stdev}, stdev={}",
            out.stdev()
        );
        assert!(
            exp_median.rel_approx_eq(out.median(), EPSILON),
            "exp_median={exp_median}, median={}",
            out.median()
        );
        assert!(exp_mean_ln.approx_eq(out.mean_ln(), EPSILON));
        assert!(exp_stdev_ln.approx_eq(out.stdev_ln(), EPSILON));

        let summary = out.summary();
        assert!(exp_mean.rel_approx_eq(summary.mean, EPSILON));
        assert!(exp_stdev.rel_approx_eq(summary.stdev, EPSILON));
        assert!(exp_p1.rel_approx_eq(summary.p1 as f64, EPSILON));
        assert!(exp_p5.rel_approx_eq(summary.p5 as f64, EPSILON));
        assert!(exp_p10.rel_approx_eq(summary.p10 as f64, EPSILON));
        assert!(exp_p25.rel_approx_eq(summary.p25 as f64, EPSILON));
        assert!(exp_median.rel_approx_eq(summary.median as f64, EPSILON));
        assert!(exp_p75.rel_approx_eq(summary.p75 as f64, EPSILON));
        assert!(exp_p90.rel_approx_eq(summary.p90 as f64, EPSILON));
        assert!(exp_p95.rel_approx_eq(summary.p95 as f64, EPSILON));
        assert!(exp_p99.rel_approx_eq(summary.p99 as f64, EPSILON));
    }
}
