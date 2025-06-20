use hdrhistogram::Histogram;

use crate::BenchOut;

#[cfg(feature = "_friends_only")]
/// Alias of [`Histogram<u64>`].
pub type Timing = Histogram<u64>;

#[cfg(feature = "_friends_only")]
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
    pub count: u64,
    pub mean: f64,
    pub stdev: f64,
    pub min: f64,
    pub p1: f64,
    pub p5: f64,
    pub p10: f64,
    pub p25: f64,
    pub median: f64,
    pub p75: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
}

#[cfg(feature = "_friends_only")]
/// Computes a [`SummaryStats`] from a [`BenchOut`].
pub fn summary_stats(out: &BenchOut) -> SummaryStats {
    let hist = &out.hist;
    let factor = out.converson_factor();
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
