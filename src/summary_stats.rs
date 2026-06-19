use crate::{BenchOut, FpSeconds};
use hdrhistogram::Histogram;

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
    pub mean: FpSeconds,
    /// Sample standard deviation of the latencies observations.
    pub stdev: FpSeconds,
    /// Minimum observed latency.
    pub min: FpSeconds,
    /// 1st percentile latency.
    pub p1: FpSeconds,
    /// 5th percentile latency.
    pub p5: FpSeconds,
    /// 10th percentile latency.
    pub p10: FpSeconds,
    /// 25th percentile latency.
    pub p25: FpSeconds,
    /// 50th percentile (median) latency.
    pub median: FpSeconds,
    /// 75th percentile latency.
    pub p75: FpSeconds,
    /// 90th percentile latency.
    pub p90: FpSeconds,
    /// 95th percentile latency.
    pub p95: FpSeconds,
    /// 99th percentile latency.
    pub p99: FpSeconds,
    /// Maximum observed latency.
    pub max: FpSeconds,
}

#[doc(hidden)]
/// Computes a [`SummaryStats`] from a [`BenchOut`].
///
/// # Panics
///
/// Panics if the number of observations is zero.
pub fn summary_stats(out: &BenchOut) -> SummaryStats {
    let hist = &out.hist;
    let ru = out.recording_unit();

    SummaryStats {
        count: hist.len(),
        mean: out.mean(),
        stdev: out.stdev(),
        min: ru.fpsecs_from_value(hist.min()),
        p1: ru.fpsecs_from_value(hist.value_at_quantile(0.01)),
        p5: ru.fpsecs_from_value(hist.value_at_quantile(0.05)),
        p10: ru.fpsecs_from_value(hist.value_at_quantile(0.10)),
        p25: ru.fpsecs_from_value(hist.value_at_quantile(0.25)),
        median: ru.fpsecs_from_value(hist.value_at_quantile(0.50)),
        p75: ru.fpsecs_from_value(hist.value_at_quantile(0.75)),
        p90: ru.fpsecs_from_value(hist.value_at_quantile(0.90)),
        p95: ru.fpsecs_from_value(hist.value_at_quantile(0.95)),
        p99: ru.fpsecs_from_value(hist.value_at_quantile(0.99)),
        max: ru.fpsecs_from_value(hist.max()),
    }
}

#[cfg(test)]
#[cfg(feature = "_test")]
mod test {
    use super::*;
    use crate::BenchCfg;
    use crate::multi::{LatencySrc, test_support::LognormalLatencySrc};
    use basic_stats::rel_approx_eq;
    use statrs::distribution::{ContinuousCDF, Normal};

    #[test]
    fn test_summary_stats_panics_on_empty() {
        let cfg = BenchCfg::default();
        let out = crate::BenchOut::from_iter(&cfg, std::iter::empty::<FpSeconds>());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| summary_stats(&out)));
        assert!(result.is_err(), "expected panic on empty sample");
    }

    #[test]
    fn test_summary_stats_value_correctness() {
        const SAMPLE_SIZE: usize = 50_000;
        const EPSILON: f64 = 0.01;

        let target = FpSeconds::from_millis(10);
        let sigma = 1.15_f64.ln() / 2.0;
        let mu = target.as_f64().ln();

        let mut src = LognormalLatencySrc::<1>::new(1, [(target, sigma)]);
        let cfg = BenchCfg::default();
        let out = BenchOut::from_iter(&cfg, src.aggregate().take(SAMPLE_SIZE));
        let summary = summary_stats(&out);

        let normal = Normal::new(mu, sigma).unwrap();

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

        rel_approx_eq!(exp_mean, summary.mean.0, EPSILON);
        rel_approx_eq!(exp_stdev, summary.stdev.0, EPSILON);
        rel_approx_eq!(exp_median, summary.median.0, EPSILON);
        rel_approx_eq!(exp_p1, summary.p1.0, EPSILON);
        rel_approx_eq!(exp_p5, summary.p5.0, EPSILON);
        rel_approx_eq!(exp_p10, summary.p10.0, EPSILON);
        rel_approx_eq!(exp_p25, summary.p25.0, EPSILON);
        rel_approx_eq!(exp_p75, summary.p75.0, EPSILON);
        rel_approx_eq!(exp_p90, summary.p90.0, EPSILON);
        rel_approx_eq!(exp_p95, summary.p95.0, EPSILON);
        rel_approx_eq!(exp_p99, summary.p99.0, EPSILON);

        assert_eq!(out.n(), SAMPLE_SIZE as u64);
        assert_eq!(summary.count, SAMPLE_SIZE as u64);
        assert!(summary.min > FpSeconds::ZERO);
        assert!(summary.max > summary.p99);
    }
}
