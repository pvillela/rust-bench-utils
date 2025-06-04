use hdrhistogram::Histogram;

/// Alias of [`Histogram<u64>`].
pub(crate) type Timing = Histogram<u64>;

/// Constructs a [`Timing`]. The arguments correspond to [Histogram::high] and [Histogram::sigfig].
pub(crate) fn new_timing(hist_high: u64, hist_sigfig: u8) -> Timing {
    let mut hist = Histogram::<u64>::new_with_max(hist_high, hist_sigfig)
        .expect("should not happen given histogram construction");
    hist.auto(true);
    hist
}

/// Common summary statistics useful in latency testing/benchmarking.
///
/// Includes sample size, mean, standard deviation, median, several percentiles, min, and max.
#[derive(Debug, Clone)]
pub struct SummaryStats {
    pub count: u64,
    pub mean: f64,
    pub stdev: f64,
    pub min: u64,
    pub p1: u64,
    pub p5: u64,
    pub p10: u64,
    pub p25: u64,
    pub median: u64,
    pub p75: u64,
    pub p90: u64,
    pub p95: u64,
    pub p99: u64,
    pub max: u64,
}

impl SummaryStats {
    /// Computes summary statistics from the given histogram.
    fn new(hist: &Timing) -> Self {
        Self {
            count: hist.len(),
            mean: hist.mean(),
            stdev: hist.stdev(),
            min: hist.min(),
            p1: hist.value_at_quantile(0.01),
            p5: hist.value_at_quantile(0.05),
            p10: hist.value_at_quantile(0.10),
            p25: hist.value_at_quantile(0.25),
            median: hist.value_at_quantile(0.50),
            p75: hist.value_at_quantile(0.75),
            p90: hist.value_at_quantile(0.90),
            p95: hist.value_at_quantile(0.95),
            p99: hist.value_at_quantile(0.99),
            max: hist.max(),
        }
    }
}

/// Computes a [`SummaryStats`] from a [`Histogram<u64>`].
pub(crate) fn summary_stats(hist: &Histogram<u64>) -> SummaryStats {
    SummaryStats::new(hist)
}
