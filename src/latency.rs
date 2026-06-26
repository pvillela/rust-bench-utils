use crate::dev_support::{midpoint_value, quickmedian};
use log::trace;
use std::{
    fmt::Debug,
    iter::Sum,
    ops::{Add, AddAssign, Deref, Div, Mul, Sub},
    time::{Duration, Instant},
};

/// Invokes `f` once and returns its latency.
#[inline(always)]
pub fn latency(f: impl FnOnce()) -> Duration {
    let start = Instant::now();
    f();
    start.elapsed()
}

/// Invokes `f` `n` times and returns its latency.
#[inline(always)]
pub fn latency_n(mut f: impl FnMut(), n: usize) -> Duration {
    let start = Instant::now();
    for _ in 0..n {
        f();
    }
    start.elapsed()
}

/// Returns the median of the latecies for the execution `n_batches` times of [`latency_n`]`(f, batch)`.
pub fn median_batch_latency(mut f: impl FnMut(), batch: usize, n_batches: usize) -> FpSeconds {
    let mut latencies: Vec<FpSeconds> = Vec::<FpSeconds>::with_capacity(n_batches);
    for _ in 0..n_batches {
        latencies.push(FpSeconds::from_duration(latency_n(&mut f, batch)) / batch);
    }
    quickmedian(&mut latencies);
    midpoint_value(&latencies)
}

/// A floating point duration of seconds. Useful for representing duration values or fractions with
/// finer granularity than 1ns.
#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct FpSeconds(pub f64);

impl FpSeconds {
    pub const ZERO: FpSeconds = FpSeconds(0.0);

    #[inline(always)]
    pub fn as_duration(&self) -> Duration {
        Duration::from_secs_f64(self.0)
    }

    #[inline(always)]
    pub fn as_f64(&self) -> f64 {
        self.0
    }

    pub fn from_duration(dur: Duration) -> Self {
        dur.into()
    }

    pub fn from_secs(value: u64) -> Self {
        (value as f64).into()
    }

    pub fn from_millis(value: u64) -> Self {
        (value as f64 * 1e-3).into()
    }

    pub fn from_micros(value: u64) -> Self {
        (value as f64 * 1e-6).into()
    }

    pub fn from_nanos(value: u64) -> Self {
        (value as f64 * 1e-9).into()
    }

    pub fn from_picos(value: u64) -> Self {
        (value as f64 * 1e-12).into()
    }
}

impl From<f64> for FpSeconds {
    #[inline(always)]
    fn from(value: f64) -> Self {
        FpSeconds(value)
    }
}

impl From<Duration> for FpSeconds {
    #[inline(always)]
    fn from(value: Duration) -> Self {
        FpSeconds(value.as_secs_f64())
    }
}

impl From<FpSeconds> for f64 {
    #[inline(always)]
    fn from(value: FpSeconds) -> Self {
        value.0
    }
}

impl From<FpSeconds> for Duration {
    #[inline(always)]
    fn from(value: FpSeconds) -> Self {
        Duration::from_secs_f64(value.0)
    }
}

impl Deref for FpSeconds {
    type Target = f64;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Add for FpSeconds {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        (self.0 + rhs.0).into()
    }
}

impl AddAssign for FpSeconds {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl Sub for FpSeconds {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        (self.0 - rhs.0).into()
    }
}

impl Mul<f64> for FpSeconds {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        (self.0 * rhs).into()
    }
}

impl Mul<usize> for FpSeconds {
    type Output = Self;

    fn mul(self, rhs: usize) -> Self::Output {
        (self.0 * rhs as f64).into()
    }
}

impl Div<usize> for FpSeconds {
    type Output = Self;

    fn div(self, rhs: usize) -> Self::Output {
        (self.0 / rhs as f64).into()
    }
}

impl Div<f64> for FpSeconds {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        (self.0 / rhs as f64).into()
    }
}

impl Sum for FpSeconds {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.map(|v| v.0).sum::<f64>().into()
    }
}

impl Debug for FpSeconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = self.0;
        let str = match () {
            _ if 1e3 <= v => format!("{0:.3e}s", v),
            _ if 1.0 <= v => format!("{0:.3}s", v),
            _ if 1e-3 <= v => format!("{0:.3}ms", v * 1e3),
            _ if 1e-6 <= v => format!("{0:.3}μs", v * 1e6),
            _ if 1e-9 <= v => format!("{0:.3}ns", v * 1e9),
            _ if 1e-12 <= v => format!("{0:.3}ps", v * 1e12),
            _ if v < 1e-9 => format!("{0:.3e}ps", v * 1e12),
            _ => unreachable!(),
        };
        f.write_str(&str)
    }
}

/// Unit of time used to record latencies. Used as an argument in benchmarking functions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LatencyUnit(u8);

impl LatencyUnit {
    /// Seconds * 1e-n, where `n` is the variant's single field.
    pub const fn sub_sec(n: u8) -> LatencyUnit {
        LatencyUnit(n)
    }

    /// Picoseconds.
    pub const PICO: LatencyUnit = Self::sub_sec(12);
    /// Nanoseconds.
    pub const NANO: LatencyUnit = Self::sub_sec(9);
    /// Microseconds.
    pub const MICRO: LatencyUnit = Self::sub_sec(6);
    /// Milliseconds.
    pub const MILLI: LatencyUnit = Self::sub_sec(3);
    /// Seconds.
    pub const SEC: LatencyUnit = Self::sub_sec(0);

    /// Converts a [`Duration`] to a `u64` value according to the unit `self`.
    #[inline(always)]
    pub fn value_from_duration(&self, dur: Duration) -> u64 {
        let n = self.0 as u32;
        match n {
            0 => Duration::as_secs(&dur),
            _ if n <= 3 => (Duration::as_millis(&dur) / 10_u128.pow(3 - n)) as u64,
            _ if n <= 6 => (Duration::as_micros(&dur) / 10_u128.pow(6 - 3)) as u64,
            _ if n <= 9 => (Duration::as_nanos(&dur) / 10_u128.pow(9 - n)) as u64,
            _ if 9 < n => (Duration::as_nanos(&dur) * 10_u128.pow(n - 9)) as u64,
            _ => unreachable!(),
        }
    }

    /// Converts a `u64` value to a [`Duration`] according to the unit `self`.
    #[inline(always)]
    pub fn duration_from_value(&self, elapsed: u64) -> Duration {
        let n = self.0 as u32;
        match n {
            0 => Duration::from_secs(elapsed),
            _ if n <= 3 => Duration::from_millis(elapsed * 10_u64.pow(3 - n)),
            _ if n <= 6 => Duration::from_micros(elapsed * 10_u64.pow(6 - n)),
            _ if n <= 9 => Duration::from_micros(elapsed * 10_u64.pow(9 - n)),
            _ if 9 < n => Duration::from_nanos(elapsed / 10_u64.pow(n - 9)),
            _ => unreachable!(),
        }
    }

    /// Converts a [`Duration`] to a `u64` value according to the unit `self`.
    #[inline(always)]
    pub fn value_from_fpsecs(&self, fpsecs: FpSeconds) -> u64 {
        (fpsecs.0 * self.factor_from_secs()).round() as u64
    }

    /// Converts a `u64` value to a [`Duration`] according to the unit `self`.
    #[inline(always)]
    pub fn fpsecs_from_value(&self, value: u64) -> FpSeconds {
        (value as f64 * self.factor_to_secs()).into()
    }

    /// Multiplicative factor to convert seconds to the latency unit.
    pub fn factor_from_secs(&self) -> f64 {
        10.0_f64.powi(self.0 as i32)
    }

    /// Multiplicative factor to convert the latency unit to seconds.
    pub fn factor_to_secs(&self) -> f64 {
        10.0_f64.powi(-(self.0 as i32))
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
pub(crate) fn execs_per_sec(mut src: impl Iterator<Item = FpSeconds>, budget: RunLength) -> f64 {
    let (warmup_count, warmup_fps, exec_count, exec_fps) = {
        let (budget_count, budget_dur) = budget.exec_count_and_duration();
        let warmup_count = budget_count / 2;
        let exec_count = budget_count - warmup_count;
        let warmup_fps: FpSeconds = (budget_dur / 2).into();
        let exec_fps = FpSeconds::from_duration(budget_dur) - warmup_fps;
        (warmup_count, warmup_fps, exec_count, exec_fps)
    };

    // Warm-up
    {
        for i in 1.. {
            let mut acc_latency = FpSeconds::ZERO;
            let mut acc_execs: usize = 0;

            let iter_execs = 2usize.pow(i - 1);
            let iter_latency = (&mut src).take(iter_execs as usize).sum();
            trace!(
                "execs_per_sec >>> warmup: iter_execs={iter_execs}, iter_latency={iter_latency:?},",
            );

            acc_latency += iter_latency;
            acc_execs += iter_execs;
            trace!("execs_per_sec >>> i={i}");
            if iter_latency >= warmup_fps / 3
                || acc_latency >= warmup_fps * (2.0 / 3.0)
                || acc_execs as f64 >= warmup_count as f64 * (2.0 / 3.0)
            {
                break;
            }
        }
    }

    // Execution
    {
        for i in 1.. {
            let mut acc_latency = FpSeconds::ZERO;
            let mut acc_execs: usize = 0;

            let iter_execs = 2usize.pow(i - 1);
            let iter_latency = (&mut src).take(iter_execs as usize).sum();
            trace!("execs_per_sec >>> iter_execs={iter_execs}, iter_latency={iter_latency:?},",);

            acc_latency += iter_latency;
            acc_execs += iter_execs;
            trace!("execs_per_sec >>> i={i}");
            // Castings to f64 to avoid integer overflow or truncation to zero.
            if iter_latency >= exec_fps / 3
                || acc_latency >= exec_fps * (2.0 / 3.0)
                || acc_execs as f64 >= exec_count as f64 * (2.0 / 3.0)
            {
                let iter_execs_per_sec = iter_execs as f64 / iter_latency.as_f64();
                let acc_execs_per_sec = acc_execs as f64 / acc_latency.as_f64();
                let execs_per_sec = iter_execs_per_sec.max(acc_execs_per_sec);
                trace!(
                    "execs_per_sec >>> iter_execs_per_sec={iter_execs_per_sec}, acc_execs_per_sec={acc_execs_per_sec}, execs_per_sec={execs_per_sec}",
                );
                return execs_per_sec;
            }
        }

        unreachable!("above loop must return at some point")
    }
}

#[cfg(test)]
#[cfg(feature = "_bench")]
/// cargo test -r --package bench_utils --lib --all-features -- latency::validate --nocapture
mod validate {
    use super::*;
    use crate::{BenchCfg, bench_support::validate_latency_overhead, rel_approx_eq_fpsecs};

    // SEE ALSO: tests for `fake_work` and `busy_work`.

    #[test]
    fn test_latency_overhead() {
        const EPSILON: f64 = 0.05;

        struct Medians {
            solo_median_20: FpSeconds,
            solo_median_100: FpSeconds,
            group_median_20: FpSeconds,
            group_median_100: FpSeconds,
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

        rel_approx_eq_fpsecs!(solo_median_20 * 20, group_median_20, EPSILON);
        rel_approx_eq_fpsecs!(solo_median_100 * 100, group_median_100, EPSILON);
    }
}

#[cfg(test)]
#[cfg(feature = "_test")]
mod test_latency_unit {
    use super::*;

    #[test]
    fn latency_unit_as_u64() {
        let dur = Duration::new(1, 500_000_000); // 1.5 seconds
        assert_eq!(1500, LatencyUnit::MILLI.value_from_duration(dur));
        assert_eq!(1_500_000, LatencyUnit::MICRO.value_from_duration(dur));
        assert_eq!(1_500_000_000, LatencyUnit::NANO.value_from_duration(dur));

        let zero = Duration::ZERO;
        assert_eq!(0, LatencyUnit::MILLI.value_from_duration(zero));
        assert_eq!(0, LatencyUnit::MICRO.value_from_duration(zero));
        assert_eq!(0, LatencyUnit::NANO.value_from_duration(zero));
    }

    #[test]
    fn latency_unit_from_u64_roundtrip() {
        // Milli round-trip
        let dur = LatencyUnit::MILLI.duration_from_value(42);
        assert_eq!(42, LatencyUnit::MILLI.value_from_duration(dur));

        // Micro round-trip
        let dur = LatencyUnit::MICRO.duration_from_value(42);
        assert_eq!(42, LatencyUnit::MICRO.value_from_duration(dur));

        // Nano round-trip
        let dur = LatencyUnit::NANO.duration_from_value(42);
        assert_eq!(42, LatencyUnit::NANO.value_from_duration(dur));

        // Zero
        let dur = LatencyUnit::MICRO.duration_from_value(0);
        assert_eq!(0, LatencyUnit::MICRO.value_from_duration(dur));

        // Large value (fits exactly in Duration)
        let val: u64 = 1_000_000_000_000_000; // 10^15 nanos = ~11.6 days
        let dur = LatencyUnit::NANO.duration_from_value(val);
        assert_eq!(val, LatencyUnit::NANO.value_from_duration(dur));
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
    use std::iter;

    #[test]
    fn src_lognormal() {
        const EPSILON: f64 = 0.01;

        let target_latency = FpSeconds::from_millis(10);
        let exp_eps = 100.0;
        let mut src = LognormalLatencySrc::new_with_default_sigmas(1, [target_latency]);
        let eps = execs_per_sec(src.aggregate(), RunLength::Count(1000));

        rel_approx_eq!(exp_eps, eps, EPSILON);
    }

    #[test]
    fn src_empty() {
        let mut src = EmptyLatencySrc::<1>;
        let eps = execs_per_sec(src.aggregate(), RunLength::Count(1000));
        assert!(eps.is_infinite(), "eps={eps}");
    }

    // cargo test --package bench_utils --lib --all-features -- latency::test_execs_per_second::src_small_finite --exact --nocapture --include-ignored
    #[test]
    fn src_small_finite() {
        _ = env_logger::try_init();

        const COUNT: usize = 1;
        let iter_len = (COUNT as f64).sqrt() as usize;
        let target_latency = FpSeconds::from_secs(10);
        let mut src = LognormalLatencySrc::new_with_default_sigmas(1, [target_latency]);
        let eps = execs_per_sec(src.aggregate().take(iter_len), RunLength::Count(1000));
        assert!(eps.is_finite(), "should be finite: eps={eps}");
    }

    // cargo test --package bench_utils --lib --all-features -- latency::test_execs_per_second::src_infinite_zero --exact --nocapture --include-ignored
    #[test]
    fn src_infinite_zero() {
        let mut src = ConstLatencySrc::new(1, [FpSeconds::ZERO]);
        let eps = execs_per_sec(src.aggregate(), RunLength::Count(1000));
        assert!(eps.is_infinite(), "should be infinite: eps={eps}");
    }

    #[test]
    fn no_op_yields_positive_finite_estimate() {
        let src = iter::from_fn(|| Some(latency(|| ()).into()));
        let e = execs_per_sec(src, RunLength::Count(1000));
        assert!(e > 0.0, "src no-op should yield positive: {}", e);
        assert!(e.is_finite(), "src no-op estimate should be finite: {}", e);
    }
}
