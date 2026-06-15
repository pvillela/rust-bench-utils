use log::trace;
use std::time::{Duration, Instant};

/// Invokes `f` once and returns its latency.
#[inline(always)]
pub fn latency(f: impl FnOnce()) -> Duration {
    let start = Instant::now();
    f();
    start.elapsed()
}

/// Invokes `f` `n` times and returns its latency.
#[inline(always)]
pub fn latency_n(mut f: impl FnMut(), n: u32) -> Duration {
    let start = Instant::now();
    for _ in 0..n {
        f();
    }
    start.elapsed()
}

/// An infinite iterator that encapsulates a closure `f` and, for each invocation
/// of `next()`, yields the wall-clock latency duration from one invocation of `f`.
pub struct LatencyIter<F: FnMut()>(F);

impl<F: FnMut()> LatencyIter<F> {
    /// Constructs `Self`.
    pub fn new(f: F) -> Self {
        Self(f)
    }
}

impl<F: FnMut()> Iterator for LatencyIter<F> {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        Some(latency(&mut self.0))
    }
}

/// An infinite iterator that encapsulates a closure `f` and, for each invocation
/// of `next()`, yields the wall-clock latency duration from one invocation of `f`.
pub struct LatencyIterN<F: FnMut()>(F, u32);

impl<F: FnMut()> LatencyIterN<F> {
    /// Constructs `Self`.
    pub fn new(f: F, n: u32) -> Self {
        Self(f, n)
    }
}

/// An infinite iterator that encapsulates a closure `f` and, for each invocation
/// of `next()`, yields the wall-clock latency duration from
/// [`n`](Self::n) invocations of `f`.
impl<F: FnMut()> Iterator for LatencyIterN<F> {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        Some(latency_n(&mut self.0, self.1))
    }
}

/// Unit of time used to record latencies. Used as an argument in benchmarking functions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LatencyUnit {
    /// Milliseconds.
    Milli,
    /// Microseconds.
    Micro,
    /// Nanoseconds.
    Nano,
}

impl LatencyUnit {
    /// Converts a `latency` [`Duration`] to a `u64` value according to the unit `self`.
    #[inline(always)]
    pub fn latency_as_u64(&self, latency: Duration) -> u64 {
        match self {
            Self::Nano => latency.as_nanos() as u64,
            Self::Micro => latency.as_micros() as u64,
            Self::Milli => latency.as_millis() as u64,
        }
    }

    /// Converts a `u64` value to a [`Duration`] according to the unit `self`.
    #[inline(always)]
    pub fn latency_from_u64(&self, elapsed: u64) -> Duration {
        match self {
            Self::Nano => Duration::from_nanos(elapsed),
            Self::Micro => Duration::from_micros(elapsed),
            Self::Milli => Duration::from_millis(elapsed),
        }
    }

    /// Converts a `latency` [`Duration`] to an `f64` value according to the unit `self`.
    #[inline(always)]
    pub fn latency_as_f64(&self, latency: Duration) -> f64 {
        match self {
            Self::Nano => latency.as_nanos() as f64,
            Self::Micro => latency.as_nanos() as f64 / 1_000.0,
            Self::Milli => latency.as_nanos() as f64 / 1_000_000.0,
        }
    }

    /// Converts an `f64` value to a [`Duration`] according to the unit `self`.
    ///
    /// `NaN` is converted to a zero duration rather than panic.
    #[inline(always)]
    pub fn latency_from_f64(&self, elapsed: f64) -> Duration {
        // self.latency_from_u64(elapsed as u64)
        match self {
            Self::Nano => Duration::from_nanos(elapsed.round() as u64),
            Self::Micro => Duration::from_nanos((elapsed * 1_000.0).round() as u64),
            Self::Milli => Duration::from_nanos((elapsed * 1_000_000.0).round() as u64),
        }
    }
}

/// Specifies how long a benchmark should run for. Encapsulates a target number of iterations for the benchmark to run
/// and a time duration. The benchmark run length can be set as a number of iterations, a time duration, or
/// a number of iterations with a timeout duration.
#[derive(Debug, Clone, Copy)]
pub enum RunLength {
    /// Run for a fixed number of iterations.
    Count(usize),
    /// Run for a fixed duration.
    Time(Duration),
    /// Run for a fixed number of iterations, but stop early if the given duration is exceeded.
    CountWithTimeout(usize, Duration),
}

impl RunLength {
    /// Returns both the number of iterations and time duration specified for the benchmark to run.
    ///
    /// The benchmark ends when the specified number of iterations is reached (or exceeded)
    /// or when the time duration is reached (or exceeded), whichever comes first.
    pub fn exec_count_and_duration(&self) -> (usize, Duration) {
        match self {
            Self::Count(count) => (*count, Duration::MAX),
            Self::Time(duration) => (usize::MAX, *duration),
            Self::CountWithTimeout(count, duration) => (*count, *duration),
        }
    }

    /// Estimated number of iterations.
    pub(crate) fn estimated_count(&self, execs_per_second: f64) -> usize {
        assert!(execs_per_second > 0.0, "execs_per_second must be positive");
        match self {
            Self::Count(count) => *count,
            Self::Time(duration) => (duration.as_secs_f64() * execs_per_second).round() as usize,
            Self::CountWithTimeout(count, duration) => {
                let count_from_duration =
                    (duration.as_secs_f64() * execs_per_second).round() as usize;
                *count.min(&count_from_duration)
            }
        }
    }

    /// Estimated run duration.
    pub(crate) fn estimated_time(&self, execs_per_second: f64) -> Duration {
        match self {
            Self::Count(count) => Duration::from_secs_f64(*count as f64 / execs_per_second),
            Self::Time(duration) => *duration,
            Self::CountWithTimeout(count, duration) => {
                let duration_from_count = Duration::from_secs_f64(*count as f64 / execs_per_second);
                *duration.min(&duration_from_count)
            }
        }
    }
}

/// Estimates how many iterations of `src` can be done in one second by iterating one or more times
/// and doing a proportionality calculation.
/// The iterator `src` is expected to encapsulate closure invocations such that each
/// invocation of `next()` yields the latency observed for a closure invocation.
///
/// # Arguments
///
/// `src` - the latency source.
/// `budget` - the budget for the estimation process, in terms of duration and/or iterations.
///
/// # May return [`f64::INFINITY`]:
/// Returns `f64::INFINITY` if the aggregate latency for any iteration is zero.
/// In particular, this can happen if `src` is finite and its length is less than or equal to one half
/// of the estimation budget count.
pub(crate) fn execs_per_sec(mut src: impl Iterator<Item = Duration>, budget: RunLength) -> f64 {
    let (budget_count, budget_dur) = budget.exec_count_and_duration();
    let mut acc_latency = Duration::ZERO;
    let mut acc_execs: usize = 0;

    for i in 1.. {
        let iter_execs = 2usize.pow(i - 1);
        let iter_latency = (&mut src).take(iter_execs as usize).sum();

        acc_latency += iter_latency;
        acc_execs += iter_execs;
        trace!("src_execs_per_sec >>> i={i}");
        // Castings to f64 to avoid integer overflow or truncation to zero.
        if iter_latency >= budget_dur / 3
            || acc_latency.as_secs_f64() >= budget_dur.as_secs_f64() * (2.0 / 3.0)
            || acc_execs as f64 >= budget_count as f64 * (2.0 / 3.0)
        {
            let iter_execs_per_sec = iter_execs as f64 / iter_latency.as_secs_f64();
            let acc_execs_per_sec = acc_execs as f64 / acc_latency.as_secs_f64();
            let execs_per_sec = iter_execs_per_sec.max(acc_execs_per_sec);
            trace!("execs_per_sec >>> execs_per_sec={execs_per_sec}",);
            return execs_per_sec;
        }
    }

    unreachable!("above loop must return at some point")
}

#[cfg(test)]
#[cfg(feature = "_bench")]
/// cargo test -r --package bench_utils --lib --all-features -- latency::test --nocapture
mod validate {
    use super::*;
    use crate::{BenchCfg, bench_support::validate_latency_overhead, rel_approx_eq_dur};

    // SEE ALSO: tests for `fake_work` and `busy_work`.

    #[test]
    fn test_latency_overhead() {
        const EPSILON: f64 = 0.05;

        struct Medians {
            solo_median_20: Duration,
            solo_median_100: Duration,
            group_median_20: Duration,
            group_median_100: Duration,
        }

        let start = Instant::now();

        let Medians {
            solo_median_20,
            solo_median_100,
            group_median_20,
            group_median_100,
        } = {
            let cfg = BenchCfg::default().with_warmup_millis(100);

            let bench_duration = Duration::from_millis(100);
            let target_latency = Duration::from_micros(50);

            let (solo_median_20, group_median_20) =
                validate_latency_overhead(&cfg, bench_duration, target_latency, 20);
            let (solo_median_100, group_median_100) =
                validate_latency_overhead(&cfg, bench_duration, target_latency, 100);

            Medians {
                solo_median_20,
                solo_median_100,
                group_median_20,
                group_median_100,
            }
        };

        println!("elapsed time: {} millis", start.elapsed().as_millis());

        rel_approx_eq_dur!(solo_median_20 * 20, group_median_20, EPSILON);
        rel_approx_eq_dur!(solo_median_100 * 100, group_median_100, EPSILON);
    }
}

#[cfg(test)]
#[cfg(feature = "_test")]
mod test_latency_unit {
    use super::*;
    use basic_stats::approx_eq;

    #[test]
    fn latency_unit_as_u64() {
        let dur = Duration::new(1, 500_000_000); // 1.5 seconds
        assert_eq!(1500, LatencyUnit::Milli.latency_as_u64(dur));
        assert_eq!(1_500_000, LatencyUnit::Micro.latency_as_u64(dur));
        assert_eq!(1_500_000_000, LatencyUnit::Nano.latency_as_u64(dur));

        let zero = Duration::ZERO;
        assert_eq!(0, LatencyUnit::Milli.latency_as_u64(zero));
        assert_eq!(0, LatencyUnit::Micro.latency_as_u64(zero));
        assert_eq!(0, LatencyUnit::Nano.latency_as_u64(zero));
    }

    #[test]
    fn latency_unit_from_u64_roundtrip() {
        // Milli round-trip
        let dur = LatencyUnit::Milli.latency_from_u64(42);
        assert_eq!(42, LatencyUnit::Milli.latency_as_u64(dur));

        // Micro round-trip
        let dur = LatencyUnit::Micro.latency_from_u64(42);
        assert_eq!(42, LatencyUnit::Micro.latency_as_u64(dur));

        // Nano round-trip
        let dur = LatencyUnit::Nano.latency_from_u64(42);
        assert_eq!(42, LatencyUnit::Nano.latency_as_u64(dur));

        // Zero
        let dur = LatencyUnit::Micro.latency_from_u64(0);
        assert_eq!(0, LatencyUnit::Micro.latency_as_u64(dur));

        // Large value (fits exactly in Duration)
        let val: u64 = 1_000_000_000_000_000; // 10^15 nanos = ~11.6 days
        let dur = LatencyUnit::Nano.latency_from_u64(val);
        assert_eq!(val, LatencyUnit::Nano.latency_as_u64(dur));
    }

    #[test]
    fn latency_unit_as_f64() {
        let dur = Duration::from_nanos(2_001_001);
        approx_eq!(2_001_001.0, LatencyUnit::Nano.latency_as_f64(dur), 1e-6);
        approx_eq!(2_001.001, LatencyUnit::Micro.latency_as_f64(dur), 1e-9);
        approx_eq!(2.001_001, LatencyUnit::Milli.latency_as_f64(dur), 1e-12);

        // Duration in nanos less then 1 micro
        let small = Duration::from_nanos(999);
        approx_eq!(0.999, LatencyUnit::Micro.latency_as_f64(small), 1e-12);
    }

    #[test]
    fn latency_unit_from_f64() {
        assert_eq!(
            LatencyUnit::Nano.latency_from_f64(500.45),
            Duration::from_nanos(500),
        );
        assert_eq!(
            LatencyUnit::Micro.latency_from_f64(500.7),
            Duration::from_nanos(500_700),
        );
        assert_eq!(
            LatencyUnit::Milli.latency_from_f64(1_000.999_999),
            Duration::from_nanos(1_000_999_999),
        );
        assert_eq!(
            LatencyUnit::Milli.latency_from_f64(1_000.000_001),
            Duration::from_nanos(1_000_000_001),
        );
    }

    #[test]
    fn latency_unit_round_trip_f64() {
        let nanos_u64 = 999_u64;
        let dur = Duration::from_nanos(nanos_u64);

        let nanos = LatencyUnit::Nano.latency_as_f64(dur);
        let micros = LatencyUnit::Micro.latency_as_f64(dur);
        let millis = LatencyUnit::Milli.latency_as_f64(dur);

        let dur_nan = LatencyUnit::Nano.latency_from_f64(nanos);
        let dur_mic = LatencyUnit::Micro.latency_from_f64(micros);
        let dur_mil = LatencyUnit::Milli.latency_from_f64(millis);

        assert_eq!(dur, dur_nan);
        assert_eq!(dur, dur_mic);
        assert_eq!(dur, dur_mil);
    }
}

#[cfg(test)]
#[cfg(feature = "_test")]
// cargo test --package bench_utils --lib --all-features -- latency::test_executions_per_second --nocapture
mod test_execs_per_second {
    use super::*;
    use crate::multi::{
        LatencySrc,
        test_support::{ConstLatencySrc, EmptyLatencySrc, LognormalLatencySrc},
    };
    use basic_stats::rel_approx_eq;

    #[test]
    fn src_lognormal() {
        const EPSILON: f64 = 0.01;

        let target_latency = Duration::from_millis(10);
        let exp_eps = 100.0;
        let mut src = LognormalLatencySrc::new_with_default_sigmas([target_latency]);
        let eps = execs_per_sec(src.aggregate(), RunLength::Count(1000));

        rel_approx_eq!(exp_eps, eps, EPSILON);
    }

    #[test]
    fn src_empty() {
        let mut src = EmptyLatencySrc::<1>;
        let eps = execs_per_sec(src.aggregate(), RunLength::Count(1000));
        assert!(eps.is_infinite(), "eps={eps}");
    }

    #[test]
    fn src_small_finite() {
        const COUNT: usize = 1000;
        let iter_len = (COUNT as f64).sqrt() as usize;
        let target_latency = Duration::from_secs(10);
        let mut src = LognormalLatencySrc::new_with_default_sigmas([target_latency]);
        let eps = execs_per_sec(src.aggregate().take(iter_len), RunLength::Count(1000));
        assert!(eps.is_infinite(), "eps={eps}");
    }

    #[test]
    fn src_infinite_zero() {
        let mut src = ConstLatencySrc::new([Duration::ZERO]);
        let eps = execs_per_sec(src.aggregate(), RunLength::Count(1000));
        assert!(eps.is_infinite(), "eps={eps}");
    }

    #[test]
    fn no_op_src_yields_positive_finite_estimate() {
        let src = LatencyIter::new(|| ());
        let e = execs_per_sec(src, RunLength::Count(1000));
        assert!(e > 0.0, "src no-op should yield positive: {}", e);
        assert!(e.is_finite(), "src no-op estimate should be finite: {}", e);
    }
}
