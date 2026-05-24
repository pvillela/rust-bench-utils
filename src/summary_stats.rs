use hdrhistogram::Histogram;

use crate::BenchOut;

#[doc(hidden)]
/// Alias of [`Histogram<u64>`].
pub type Timing = Histogram<u64>;

#[doc(hidden)]
/// Constructs a [`Timing`]. The arguments correspond to [Histogram::high] and [Histogram::sigfig].
pub fn new_timing(hist_high: u64, hist_sigfig: u8) -> Timing {
    let mut hist = Histogram::<u64>::new_with_max(hist_high, hist_sigfig)
        .expect("should not happen given histogram construction");
    hist.auto(true);
    hist
}

/// Common summary statistics useful in latency testing/benchmarking.
///
/// Includes sample size, mean, standard deviation, median, several percentiles, min, and max.
#[derive(Debug, Clone, PartialEq)]
pub struct SummaryStats {
    /// Sample size (number of observations).
    pub count: u64,
    /// Arithmetic mean of the latency observations (in the reporting unit).
    pub mean: f64,
    /// Sample standard deviation of the latency observations (in the reporting unit).
    pub stdev: f64,
    /// Minimum observed latency (in the reporting unit).
    pub min: f64,
    /// 1st percentile latency (in the reporting unit).
    pub p1: f64,
    /// 5th percentile latency (in the reporting unit).
    pub p5: f64,
    /// 10th percentile latency (in the reporting unit).
    pub p10: f64,
    /// 25th percentile latency (in the reporting unit).
    pub p25: f64,
    /// 50th percentile (median) latency (in the reporting unit).
    pub median: f64,
    /// 75th percentile latency (in the reporting unit).
    pub p75: f64,
    /// 90th percentile latency (in the reporting unit).
    pub p90: f64,
    /// 95th percentile latency (in the reporting unit).
    pub p95: f64,
    /// 99th percentile latency (in the reporting unit).
    pub p99: f64,
    /// Maximum observed latency (in the reporting unit).
    pub max: f64,
}

#[doc(hidden)]
/// Computes a [`SummaryStats`] from a [`BenchOut`].
///
/// # Panics
///
/// Panics if the current value of [`crate::BenchCfg::panic_on_error`] is `true` **and** the number of observations is zero.
pub fn summary_stats(out: &BenchOut) -> SummaryStats {
    let hist = &out.hist;
    let factor = out.conversion_factor();

    SummaryStats {
        count: hist.len(),
        mean: out.mean(),
        stdev: out.stdev(),
        min: hist.min() as f64 * factor,
        p1: hist.value_at_quantile(0.01) as f64 * factor,
        p5: hist.value_at_quantile(0.05) as f64 * factor,
        p10: hist.value_at_quantile(0.10) as f64 * factor,
        p25: hist.value_at_quantile(0.25) as f64 * factor,
        median: hist.value_at_quantile(0.50) as f64 * factor,
        p75: hist.value_at_quantile(0.75) as f64 * factor,
        p90: hist.value_at_quantile(0.90) as f64 * factor,
        p95: hist.value_at_quantile(0.95) as f64 * factor,
        p99: hist.value_at_quantile(0.99) as f64 * factor,
        max: hist.max() as f64 * factor,
    }
}
