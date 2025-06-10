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
mod test {
    use super::*;
    use basic_stats::normal::deterministic_normal_sample;

    impl BenchOut {
        fn collect_data(&mut self, mut src: impl Iterator<Item = u64>) {
            while let Some(item) = src.next() {
                self.capture_data(item);
            }
        }
    }

    #[test]
    fn test() {
        let normal_samp = deterministic_normal_sample(0., 1., 10).unwrap();
        let lognormal_samp = normal_samp.map(|x| x.exp().ceil() as u64);
        let mut bout = BenchOut::new(LatencyUnit::Micro);
        bout.collect_data(lognormal_samp);

        assert_eq!(bout.unit(), LatencyUnit::Micro);
        assert_eq!(bout.n(), 199);
        assert_eq!(bout.nf(), 199.);

        let summary = bout.summary();
        // assert_eq!(summary.p1.)
    }
}
