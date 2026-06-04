use hdrhistogram::Histogram;
use std::time::Duration;

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
    /// Arithmetic mean of the latencies observations.
    pub mean: Duration,
    /// Sample standard deviation of the latencies observations.
    pub stdev: Duration,
    /// Minimum observed latency.
    pub min: Duration,
    /// 1st percentile latency.
    pub p1: Duration,
    /// 5th percentile latency.
    pub p5: Duration,
    /// 10th percentile latency.
    pub p10: Duration,
    /// 25th percentile latency.
    pub p25: Duration,
    /// 50th percentile (median) latency.
    pub median: Duration,
    /// 75th percentile latency.
    pub p75: Duration,
    /// 90th percentile latency.
    pub p90: Duration,
    /// 95th percentile latency.
    pub p95: Duration,
    /// 99th percentile latency.
    pub p99: Duration,
    /// Maximum observed latency.
    pub max: Duration,
}

#[doc(hidden)]
/// Computes a [`SummaryStats`] from a [`BenchOut`].
///
/// # Panics
///
/// Panics if the current value of [`crate::BenchCfg::panic_on_error`] is `true` **and** the number of observations is zero.
pub fn summary_stats(out: &BenchOut) -> SummaryStats {
    if out.panic_on_error() && out.n() == 0 {
        panic!("number of observations is zero");
    }

    let hist = &out.hist;
    let ru = out.recording_unit();

    SummaryStats {
        count: hist.len(),
        mean: out.mean(),
        stdev: out.stdev(),
        min: ru.latency_from_u64(hist.min()),
        p1: ru.latency_from_u64(hist.value_at_quantile(0.01)),
        p5: ru.latency_from_u64(hist.value_at_quantile(0.05)),
        p10: ru.latency_from_u64(hist.value_at_quantile(0.10)),
        p25: ru.latency_from_u64(hist.value_at_quantile(0.25)),
        median: ru.latency_from_u64(hist.value_at_quantile(0.50)),
        p75: ru.latency_from_u64(hist.value_at_quantile(0.75)),
        p90: ru.latency_from_u64(hist.value_at_quantile(0.90)),
        p95: ru.latency_from_u64(hist.value_at_quantile(0.95)),
        p99: ru.latency_from_u64(hist.value_at_quantile(0.99)),
        max: ru.latency_from_u64(hist.max()),
    }
}


#[cfg(test)]
#[cfg(feature = "_test")]
mod test {
    use super::*;
    use crate::BenchCfg;

    #[test]
    fn test_summary_stats_panics_on_empty() {
        let cfg = BenchCfg::default().with_panic_on_error(true);
        let out = crate::BenchOut::from_iter(&cfg, std::iter::empty::<std::time::Duration>());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| summary_stats(&out)));
        assert!(result.is_err(), "expected panic on empty sample");
    }
}