//! Implements functions to collect latency statistics for a closure.

use crate::{
    BenchCfg, BenchOut, RunLength,
    dev_support::bsz,
    multi::{self, LatencySrc1, LatencySrc1b},
    status::Status,
};

/// Repeatedly executes closure `f`, collects the resulting latency data in a [`BenchOut`] object, and
/// *optionally* reports progress status during benchmark execution.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// `cfg.warmup_millis` milliseconds.
///
/// Batching can be used to dilute measurement overhead, but see library documentation about the caveats
/// of batching.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f` - benchmark target closure.
/// - `run_length` - target run length (execution count and/or duration) for data collection.
/// - `s` - status handler for reporting warm-up and main execution progress.
/// - `batch` - determines whether batching is used and, if so, the batch size. `None` means no batching.
pub fn bench_run_x<'a, S: Status<'a>>(
    cfg: &BenchCfg,
    f: impl FnMut(),
    run_length: RunLength,
    s: S,
    batch: Option<usize>,
) -> BenchOut {
    match batch {
        None => multi::bench_run_x(cfg, LatencySrc1::new(f), run_length, s).into(),
        Some(_) => multi::bench_run_x(cfg, LatencySrc1b::new(f, bsz(batch)), run_length, s).into(),
    }
}

/// Repeatedly executes closure `f` and collects the resulting latency data in a [`BenchOut`] object.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
///
/// This function is equivalent to calling [`bench_run_x`] with a pre-defined no-op status object.
///
/// Batching can be used to dilute measurement overhead, but see library documentation about the caveats
/// of batching.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f` - benchmark target.
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
/// - `batch` - determines whether batching is used and, if so, the batch size. `None` means no batching.
pub fn bench_run_arg_cfg(
    cfg: &BenchCfg,
    f: impl FnMut(),
    run_length: RunLength,
    batch: Option<usize>,
) -> BenchOut {
    match batch {
        None => multi::bench_run_arg_cfg(cfg, LatencySrc1::new(f), run_length).into(),
        Some(_) => {
            multi::bench_run_arg_cfg(cfg, LatencySrc1b::new(f, bsz(batch)), run_length).into()
        }
    }
}

/// Repeatedly executes closure `f` and collects the resulting latency data in a [`BenchOut`] object.
/// Runs with the default [`BenchCfg`].
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
///
/// This function calls [`bench_run_arg_cfg`] with the default [`BenchCfg`].
///
/// Batching can be used to dilute measurement overhead, but see library documentation about the caveats
/// of batching.
///
/// Arguments:
/// - `f` - benchmark target.
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
/// - `batch` - determines whether batching is used and, if so, the batch size. `None` means no batching.
pub fn bench_run(f: impl FnMut(), run_length: RunLength, batch: Option<usize>) -> BenchOut {
    let cfg = BenchCfg::default();
    bench_run_arg_cfg(&cfg, f, run_length, batch)
}

/// Repeatedly executes closure `f`, collects the resulting latency data in a [`BenchOut`] object, and
/// outputs information about the benchmark and its execution status.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
///
/// This function is equivalent to calling [`bench_run_x`] with a pre-defined status object that supports
/// the output of benchmark status to `stderr`.
///
/// Batching can be used to dilute measurement overhead, but see library documentation about the caveats
/// of batching.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f` - benchmark target.
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
/// - `batch` - determines whether batching is used and, if so, the batch size. `None` means no batching.
pub fn bench_run_with_status_arg_cfg(
    cfg: &BenchCfg,
    f: impl FnMut(),
    run_length: RunLength,
    batch: Option<usize>,
) -> BenchOut {
    match batch {
        None => multi::bench_run_with_status_arg_cfg(cfg, LatencySrc1::new(f), run_length).into(),
        Some(_) => {
            multi::bench_run_with_status_arg_cfg(cfg, LatencySrc1b::new(f, bsz(batch)), run_length)
                .into()
        }
    }
}

/// Repeatedly executes closure `f`, collects the resulting latency data in a [`BenchOut`] object, and
/// outputs information about the benchmark and its execution status.
/// Runs with the default [`BenchCfg`].
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
///
/// This function calls [`bench_run_with_status_arg_cfg`] with the default [`BenchCfg`].
///
/// Batching can be used to dilute measurement overhead, but see library documentation about the caveats
/// of batching.
///
/// Arguments:
/// - `f` - benchmark target.
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
/// - `batch` - determines whether batching is used and, if so, the batch size. `None` means no batching.
pub fn bench_run_with_status(
    f: impl FnMut(),
    run_length: RunLength,
    batch: Option<usize>,
) -> BenchOut {
    let cfg = BenchCfg::default();
    bench_run_with_status_arg_cfg(&cfg, f, run_length, batch).into()
}

/// Computes the clock overhead of `bench_run` by benchmarking a no-op function.
/// It executes `groups` benchmarks of a no-op function, where each benchmark has
/// a run-length of `RunLength::Count(exec_count)`, with no batching.
///
/// We define "clock overhead" as the time between the two clock reads around each timed
/// execution or batch, incurred once per recorded value whatever the batch size.
/// The values provided by this function depend on the operating system's clock read
/// resolution. On Linux, the resolution is 1 ns.
///
/// This function returns a pair of [`BenchOut`]s.
/// The first `BenchOut` contains the pooled data from the `exec_count * groups` executions
/// of the no-op function.
/// The second `BenchOut` contains one median observation for each of the `groups` runs
/// of `bench_run`.
///
/// The key statistics of interest are the pooled [`median_r`](BenchOut::median_r) and the
/// grouped [`stdev_r`](BenchOut::stdev_r).
/// The first is a robust estimate of the overhead, the second measures the variability
/// of the estimate across runs.
///
/// # Panics
/// If `exec_count == 0` or `groups == 0`.
pub fn overhead_x(exec_count: usize, groups: usize) -> (BenchOut, BenchOut) {
    assert!(
        exec_count > 0 && groups > 0,
        "both `exec_count` and `groups` must be > 0"
    );
    let target_fn = || ();
    let cfg = BenchCfg::default().with_warmup_millis(0);

    // Warmup.
    let cfg_w = BenchCfg::default().with_warmup_millis(1000);
    bench_run_arg_cfg(&cfg_w, target_fn, RunLength::Count(exec_count), None);

    let mut pooled_out = BenchOut::new(&cfg, None);
    let mut grouped_out = BenchOut::new(&cfg, None);
    for _ in 0..groups {
        let out = bench_run_arg_cfg(&cfg, target_fn, RunLength::Count(exec_count), None);
        pooled_out.record_from_iter_with_counts(out.iter_with_counts());
        grouped_out.capture_data_with_counts((out.median_r(), 1));
    }
    (pooled_out, grouped_out)
}

/// Computes the clock overhead of `bench_run` by benchmarking a no-op function
/// with a large pre-configured execution count and no batching.
/// The [`median_r`](BenchOut::median_r) value from the returned `BenchOut` is an estimate
/// of the clock overhead.
///
/// We define "clock overhead" as the time between the two clock reads around each timed
/// execution or batch, incurred once per recorded value whatever the batch size.
/// The values provided by this function depend on the operating system's clock read
/// resolution. On Linux, the resolution is 1 ns.
pub fn overhead() -> BenchOut {
    const EXEC_COUNT: usize = 10_000;
    let target_fn = || ();
    let cfg = BenchCfg::default().with_warmup_millis(1000);
    bench_run_arg_cfg(&cfg, target_fn, RunLength::Count(EXEC_COUNT), None)
}

#[cfg(test)]
#[cfg(feature = "_test")]
/// Tests for `bench_run`, `bench_run_with_status`, `bench_run_x` — K=1 wrappers around `multi` functions.
/// See multi::bench_run for more extensive tests.
mod simple_tests {
    use super::*;
    use crate::{LatencyUnit, RunLength};
    use crate::{status::DefaultStatus, test_support::StringWriter};
    use std::{thread, time::Duration};

    /// Helper to get a clean config with minimal warmup/calibration for fast tests.
    fn quick_cfg() -> BenchCfg {
        BenchCfg::default()
            .with_warmup_millis(0)
            .with_status_millis(1)
            .with_recording_unit(LatencyUnit::NANO)
    }

    #[test]
    fn test_bench_run_with_count() {
        let cfg = quick_cfg();
        let out = bench_run_arg_cfg(&cfg, || (), RunLength::Count(5), None);
        // With 5 count and no timeout, we should have exactly 5 iterations
        assert_eq!(out.n_r(), 5);
    }

    #[test]
    fn test_bench_run_with_time() {
        let cfg = quick_cfg();

        // Use a very short timeout that should be exceeded immediately
        let out = bench_run_arg_cfg(
            &cfg,
            || thread::sleep(Duration::from_nanos(1)),
            RunLength::Time(Duration::from_nanos(1)),
            None,
        );
        // At least some executions should have been captured
        assert!(out.n_r() > 0);
    }

    #[test]
    fn test_bench_run_with_timeout() {
        let cfg = quick_cfg();

        // Use a very short timeout that should be exceeded immediately
        let out = bench_run_arg_cfg(
            &cfg,
            || thread::sleep(Duration::from_nanos(1)),
            RunLength::CountWithTimeout(20, Duration::from_nanos(1)),
            None,
        );
        // At least some executions should have been captured
        assert!(out.n_r() > 0 && out.n_r() < 20);
    }

    #[test]
    /// Takes  3 seconds to run due to default warmup_millis.
    fn test_bench_run_default() {
        let out = bench_run(|| (), RunLength::Count(5), None);
        assert_eq!(out.n_r(), 5);
    }

    #[test]
    /// Takes  3 seconds to run due to default warmup_millis.
    fn test_bench_run_with_status() {
        let out = bench_run_with_status(|| (), RunLength::Count(5), None);
        assert_eq!(out.n_r(), 5);
    }

    #[test]
    fn test_bench_run_x() {
        let cfg = quick_cfg();
        let mut buf = StringWriter::new();
        let status = DefaultStatus::new(&mut buf, "Warming up".to_string(), "Running".to_string());
        let out = bench_run_x(&cfg, || (), RunLength::Count(5), status, None);
        assert_eq!(out.n_r(), 5);
    }
}
