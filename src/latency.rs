use crate::{RunLength, multi::LatencySrc1};
use log::trace;
use std::time::{Duration, Instant};

/// Invokes `f` once and returns its latency.
#[inline(always)]
pub fn latency(f: impl FnOnce()) -> Duration {
    let start = Instant::now();
    f();
    start.elapsed()
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
    /// Rounds the floating point value, so precision can be lost in a round trip starting with `self.latency_as_f64`,
    /// followed by `self.latency_from_f64`, followed by `self.latency_as_f64`.
    #[inline(always)]
    pub fn latency_from_f64(&self, elapsed: f64) -> Duration {
        // self.latency_from_u64(elapsed as u64)
        match self {
            Self::Nano => Duration::from_nanos(elapsed as u64),
            Self::Micro => Duration::from_nanos((elapsed * 1_000.0) as u64),
            Self::Milli => Duration::from_nanos((elapsed * 1_000_000.0) as u64),
        }
    }
}

/// Estimates how many executions of `f` fit in one millisecond by executing the function one or more times
/// and doing a proportionality calculation.
///
/// # Arguments
///
/// `f` - the target function.
/// `budget` - the budget for the estimation process, in terms of duration and/or iterations.
pub fn fn_executions_per_milli(f: impl FnMut(), budget: RunLength) -> f64 {
    let src = LatencySrc1(f).map(|arr| arr[0]);
    ltn_src_executions_per_milli(src, budget)
}

/// Estimates how many iterations of `src` can be done in one millisecond by iterating one or more times
/// and doing a proportionality calculation.
/// The iterator `src` is expected to encapsulate closure invocations such that each
/// invocation of `next()` yields the latency observed for a closure invocation.
///
/// # Arguments
///
/// `src` - the latency source.
/// `budget` - the budget for the estimation process, in terms of duration and/or iterations.
pub fn ltn_src_executions_per_milli(
    mut src: impl Iterator<Item = Duration>,
    budget: RunLength,
) -> f64 {
    let mut acc_latency = Duration::from_nanos(0);
    let mut acc_execs: u64 = 0;

    for i in 1.. {
        let iter_execs = 2u64.pow(i - 1);
        let iter_latency = (&mut src).take(iter_execs as usize).sum();

        acc_latency += iter_latency;
        acc_execs += iter_execs;
        let (budget_count, budget_dur) = budget.get_exec_count_and_duration();
        trace!("ltn_src_executions_per_milli: i={i}");
        if iter_latency >= budget_dur / 2 || acc_latency >= budget_dur || acc_execs >= budget_count
        {
            let iter_execs_per_milli = iter_execs as f64 / (iter_latency.as_secs_f64() * 1000.0);
            let acc_execs_per_milli = acc_execs as f64 / (acc_latency.as_secs_f64() * 1000.0);
            trace!(
                "ltn_src_executions_per_milli={}",
                iter_execs_per_milli.max(acc_execs_per_milli)
            );
            return iter_execs_per_milli.max(acc_execs_per_milli);
        }
    }

    unreachable!("above loop must return at some point")
}

#[cfg(test)]
#[cfg(feature = "_bench_long_test")]
/// cargo test -r --package bench_utils --lib --all-features -- latency::test --nocapture
mod test {
    use super::*;
    use crate::{BenchCfg, bench_support::validate_latency_overhead, rel_approx_eq_dur};
    use basic_stats::approx_eq;

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
            // let cfg = BenchCfg::default().with_warmup_millis(50); // was failing with this
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

    #[test]
    fn test_latency_unit_as_u64() {
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
    fn test_latency_unit_from_u64_roundtrip() {
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
    fn test_latency_unit_as_f64() {
        let dur = Duration::from_nanos(2_001_001);
        approx_eq!(2_001_001.0, LatencyUnit::Nano.latency_as_f64(dur), 1e-6);
        approx_eq!(2_001.001, LatencyUnit::Micro.latency_as_f64(dur), 1e-9);
        approx_eq!(2.001_001, LatencyUnit::Milli.latency_as_f64(dur), 1e-12);

        // Duration in nanos less then 1 micro
        let small = Duration::from_nanos(999);
        approx_eq!(0.999, LatencyUnit::Micro.latency_as_f64(small), 1e-12);
    }

    #[test]
    fn test_latency_unit_from_f64() {
        assert_eq!(
            LatencyUnit::Nano.latency_from_f64(500.7),
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
    fn test_latency_round_trip_f64() {
        // Round trip
        let nanos_u = 999_u64;
        let dur = Duration::from_nanos(nanos_u);

        let nanos = LatencyUnit::Nano.latency_as_f64(dur);
        let micros = LatencyUnit::Micro.latency_as_f64(dur);
        let millis = LatencyUnit::Milli.latency_as_f64(dur);

        let dur_nan = LatencyUnit::Nano.latency_from_f64(nanos);
        let dur_mic = LatencyUnit::Micro.latency_from_f64(micros);
        let dur_mil = LatencyUnit::Milli.latency_from_f64(millis);

        assert_eq!(dur_nan, dur_mic);
        assert_eq!(dur_nan, dur_mil);
    }
}
#[cfg(test)]
#[cfg(feature = "_test")]
// cargo test --package bench_utils --lib --all-features -- latency::test_executions_per_milli --nocapture
mod test_executions_per_milli {
    use super::*;
    use basic_stats::rel_approx_eq;

    struct ConvergingIterator {
        limit: Duration,
        iteration: u64,
    }
    impl Iterator for ConvergingIterator {
        type Item = Duration;

        fn next(&mut self) -> Option<Self::Item> {
            self.iteration += 1;
            let value = self.limit.mul_f64(1.0 + 1.0 / self.iteration as f64);
            Some(value)
        }
    }

    #[test]
    fn test_ltn_src_executions_per_milli() {
        const EPSILON: f64 = 0.01;

        let limit = Duration::from_millis(10);
        let exp_epm = 0.1;
        let src = ConvergingIterator {
            limit,
            iteration: 0,
        };
        let epm = ltn_src_executions_per_milli(src, RunLength::Count(1000));

        rel_approx_eq!(exp_epm, epm, EPSILON);
    }

    #[test]
    fn no_op_yields_positive_finite_estimate() {
        let e = fn_executions_per_milli(|| {}, RunLength::Count(1000));
        assert!(e > 0.0, "no-op should yield positive: {}", e);
        assert!(e.is_finite(), "no-op estimate should be finite: {}", e);
    }

    #[test]
    fn ltn_src_no_op_yields_positive_finite_estimate() {
        let src = LatencySrc1(|| {}).map(|arr| arr[0]);
        let e = ltn_src_executions_per_milli(src, RunLength::Count(1000));
        assert!(e > 0.0, "ltn_src no-op should yield positive: {}", e);
        assert!(
            e.is_finite(),
            "ltn_src no-op estimate should be finite: {}",
            e
        );
    }

    #[test]
    fn fn_and_ltn_src_agree_for_no_op() {
        let fn_e = fn_executions_per_milli(|| {}, RunLength::Count(1000));
        let src_e = ltn_src_executions_per_milli(
            LatencySrc1(|| {}).map(|arr| arr[0]),
            RunLength::Count(1000),
        );
        let ratio = fn_e / src_e;
        assert!(
            ratio > 0.5 && ratio < 2.0,
            "fn and ltn_src estimates should agree: fn={}, src={}",
            fn_e,
            src_e,
        );
    }
}
