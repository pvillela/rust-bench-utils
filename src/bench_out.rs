//! Module defining the key data structure produced by [`crate::bench_one`].

use crate::{SummaryStats, Timing, summary_stats};
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
    pub(super) hist: Timing,
    pub(super) sum: i64,
    pub(super) sum_ln: f64,
    pub(super) sum2_ln: f64,
}

impl BenchOut {
    #[cfg(feature = "_collect")]
    /// Creates a new empty instance.
    pub fn new() -> Self {
        use crate::new_timing;

        let hist = new_timing(20 * 1000 * 1000, 5);
        let sum = 0;
        let sum_ln = 0.;
        let sum2_ln = 0.;

        Self {
            hist,
            sum,
            sum_ln,
            sum2_ln,
        }
    }

    #[cfg(feature = "_collect")]
    /// Updates `self` with an elapsed time observation for the function.
    #[inline(always)]
    pub fn capture_data(&mut self, elapsed: u64) {
        self.hist
            .record(elapsed)
            .expect("can't happen: histogram is auto-resizable");

        assert!(elapsed > 0, "latency must be > 0");
        self.sum += elapsed as i64;
        let ln = (elapsed as f64).ln();
        self.sum_ln += ln;
        self.sum2_ln += ln.powi(2);
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
        summary_stats(&self.hist)
    }

    /// Mean of `f1`'s latencies.
    pub fn mean(&self) -> f64 {
        self.summary().mean
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
}
