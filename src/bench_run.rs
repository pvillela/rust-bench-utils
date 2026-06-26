//! Implements functions to collect latency statistics for a closure.

use crate::{
    BenchCfg, BenchOut, RunLength,
    multi::{self, LatencySrc1, LatencySrc1b},
    status::Status,
};

/// Repeatedly executes closure `f`, collects the resulting latency data in a [`BenchOut`] object, and
/// *optionally* reports progress status during benchmark execution.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// `cfg.warmup_millis` milliseconds.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f` - benchmark target closure.
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
/// - `s` - status handler for reporting warm-up and execution progress.
pub fn bench_run_x<'a, S: Status<'a>>(
    cfg: &BenchCfg,
    f: impl FnMut(),
    run_length: RunLength,
    s: S,
) -> BenchOut {
    multi::bench_run_x(cfg, LatencySrc1::new(f), run_length, s).into()
}

/// Repeatedly executes closure `f` and collects the resulting latency data in a [`BenchOut`] object.
/// Runs with the default [`BenchCfg`].
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
/// This function calls [`bench_run_x`] with no-op closures for the arguments that support the output of
/// benchmark status.
///
/// Arguments:
/// - `f` - benchmark target.
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run(f: impl FnMut(), run_length: RunLength) -> BenchOut {
    multi::bench_run(LatencySrc1::new(f), run_length).into()
}

/// Repeatedly executes closure `f` and collects the resulting latency data in a [`BenchOut`] object.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
/// This function calls [`bench_run_x`] with no-op closures for the arguments that support the output of
/// benchmark status.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f` - benchmark target.
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run_arg_cfg(cfg: &BenchCfg, f: impl FnMut(), run_length: RunLength) -> BenchOut {
    multi::bench_run_arg_cfg(cfg, LatencySrc1::new(f), run_length).into()
}

/// Repeatedly executes closure `f`, collects the resulting latency data in a [`BenchOut`] object, and
/// outputs information about the benchmark and its execution status.
/// Runs with the default [`BenchCfg`].
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
/// This function calls [`bench_run_x`] with pre-defined closures for the arguments that support the output of
/// benchmark status to `stderr`.
///
/// Arguments:
/// - `f` - benchmark target.
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run_with_status(f: impl FnMut(), run_length: RunLength) -> BenchOut {
    multi::bench_run_with_status(LatencySrc1::new(f), run_length).into()
}

/// Repeatedly executes closure `f`, collects the resulting latency data in a [`BenchOut`] object, and
/// outputs information about the benchmark and its execution status.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
/// This function calls [`bench_run_x`] with pre-defined closures for the arguments that support the output of
/// benchmark status to `stderr`.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f` - benchmark target.
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run_with_status_arg_cfg(
    cfg: &BenchCfg,
    f: impl FnMut(),
    run_length: RunLength,
) -> BenchOut {
    multi::bench_run_with_status_arg_cfg(cfg, LatencySrc1::new(f), run_length).into()
}

pub(crate) fn batch_run_length(run_length: RunLength, batch: Option<usize>) -> RunLength {
    match batch {
        None => run_length,
        Some(batch) => match run_length {
            RunLength::Count(count) => RunLength::Count(count.div_ceil(batch)),
            RunLength::Time(_) => run_length,
            RunLength::CountWithTimeout(count, time) => {
                RunLength::CountWithTimeout(count.div_ceil(batch), time)
            }
        },
    }
}

/// Similar to [`bench_run_x`] but batches the executions of `f` into groups of size `batch`.
///
/// Batching may reduce measurement overhead.
/// Each batch results in the batch average being collected `batch` times, so the number of captured
/// latency values is not impacted by grouping.
/// However, a potential consequence is that the statistical tests provided by [`BenchOut`] may be somewhat
/// distorted as the resulting distribution may no longer be approximately logormal.
pub fn bench_run_x_b<'a, S: Status<'a>>(
    cfg: &BenchCfg,
    f: impl FnMut(),
    run_length: RunLength,
    s: S,
    batch: usize,
) -> BenchOut {
    let run_length = batch_run_length(run_length, Some(batch));
    multi::bench_run_x(cfg, LatencySrc1b::new(f, batch), run_length, s).into()
}

/// Similar to [`bench_run`] but batches the executions of `f` into groups of size `batch`.
///
/// Batching may reduce measurement overhead.
/// Each batch results in the batch average being collected `batch` times, so the number of captured
/// latency values is not impacted by grouping.
/// However, a potential consequence is that the statistical tests provided by [`BenchOut`] may be somewhat
/// distorted as the resulting distribution may no longer be approximately logormal.
pub fn bench_run_b(f: impl FnMut(), run_length: RunLength, batch: usize) -> BenchOut {
    let run_length = batch_run_length(run_length, Some(batch));
    multi::bench_run(LatencySrc1b::new(f, batch), run_length).into()
}

/// Similar to [`bench_run_arg_cfg`] but batches the executions of `f` into groups of size `batch`.
///
/// Batching may reduce measurement overhead.
/// Each batch results in the batch average being collected `batch` times, so the number of captured
/// latency values is not impacted by grouping.
/// However, a potential consequence is that the statistical tests provided by [`BenchOut`] may be somewhat
/// distorted as the resulting distribution may no longer be approximately logormal.
pub fn bench_run_arg_cfg_b(
    cfg: &BenchCfg,
    f: impl FnMut(),
    run_length: RunLength,
    batch: usize,
) -> BenchOut {
    let run_length = batch_run_length(run_length, Some(batch));
    multi::bench_run_arg_cfg(cfg, LatencySrc1b::new(f, batch), run_length).into()
}

/// Similar to [`bench_run_with_status`] but batches the executions of `f` into groups of size `batch`.
///
/// Batching may reduce measurement overhead.
/// Each batch results in the batch average being collected `batch` times, so the number of captured
/// latency values is not impacted by grouping.
/// However, a potential consequence is that the statistical tests provided by [`BenchOut`] may be somewhat
/// distorted as the resulting distribution may no longer be approximately logormal.
pub fn bench_run_with_status_b(f: impl FnMut(), run_length: RunLength, batch: usize) -> BenchOut {
    let run_length = batch_run_length(run_length, Some(batch));
    multi::bench_run_with_status(LatencySrc1b::new(f, batch), run_length).into()
}

/// Similar to [`bench_run_with_status_arg_cfg`] but batches the executions of `f` into groups of size `batch`.
///
/// Batching may reduce measurement overhead.
/// Each batch results in the batch average being collected `batch` times, so the number of captured
/// latency values is not impacted by grouping.
/// However, a potential consequence is that the statistical tests provided by [`BenchOut`] may be somewhat
/// distorted as the resulting distribution may no longer be approximately logormal.
pub fn bench_run_with_status_arg_cfg_b(
    cfg: &BenchCfg,
    f: impl FnMut(),
    run_length: RunLength,
    batch: usize,
) -> BenchOut {
    let run_length = batch_run_length(run_length, Some(batch));
    multi::bench_run_with_status_arg_cfg(cfg, LatencySrc1b::new(f, batch), run_length).into()
}

#[cfg(feature = "_test_support")]
pub fn bench_run_x_o<'a, S: Status<'a>>(
    cfg: &BenchCfg,
    f: impl FnMut(),
    run_length: RunLength,
    s: S,
    batch: Option<usize>,
) -> BenchOut {
    match batch {
        None => bench_run_x(&cfg, f, run_length, s),
        Some(batch) => bench_run_x_b(&cfg, f, run_length, s, batch),
    }
}

#[cfg(feature = "_test_support")]
pub fn bench_run_arg_cfg_o(
    cfg: &BenchCfg,
    f: impl FnMut(),
    run_length: RunLength,
    batch: Option<usize>,
) -> BenchOut {
    match batch {
        None => bench_run_arg_cfg(&cfg, f, run_length),
        Some(batch) => bench_run_arg_cfg_b(&cfg, f, run_length, batch),
    }
}

#[cfg(feature = "_test_support")]
pub fn bench_run_with_status_arg_cfg_o(
    cfg: &BenchCfg,
    f: impl FnMut(),
    run_length: RunLength,
    batch: Option<usize>,
) -> BenchOut {
    match batch {
        None => bench_run_with_status_arg_cfg(&cfg, f, run_length),
        Some(batch) => bench_run_with_status_arg_cfg_b(&cfg, f, run_length, batch),
    }
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
        let out = bench_run_arg_cfg(&cfg, || (), RunLength::Count(5));
        // With 5 count and no timeout, we should have exactly 5 iterations
        assert_eq!(out.n(), 5);
    }

    #[test]
    fn test_bench_run_with_time() {
        let cfg = quick_cfg();

        // Use a very short timeout that should be exceeded immediately
        let out = bench_run_arg_cfg(
            &cfg,
            || thread::sleep(Duration::from_nanos(1)),
            RunLength::Time(Duration::from_nanos(1)),
        );
        // At least some executions should have been captured
        assert!(out.n() > 0);
    }

    #[test]
    fn test_bench_run_with_timeout() {
        let cfg = quick_cfg();

        // Use a very short timeout that should be exceeded immediately
        let out = bench_run_arg_cfg(
            &cfg,
            || thread::sleep(Duration::from_nanos(1)),
            RunLength::CountWithTimeout(20, Duration::from_nanos(1)),
        );
        // At least some executions should have been captured
        assert!(out.n() > 0 && out.n() < 20);
    }

    #[test]
    /// Takes  3 seconds to run due to default warmup_millis.
    fn test_bench_run_default() {
        let out = bench_run(|| (), RunLength::Count(5));
        assert_eq!(out.n(), 5);
    }

    #[test]
    /// Takes  3 seconds to run due to default warmup_millis.
    fn test_bench_run_with_status() {
        let out = bench_run_with_status(|| (), RunLength::Count(5));
        assert_eq!(out.n(), 5);
    }

    #[test]
    fn test_bench_run_x() {
        let cfg = quick_cfg();
        let mut buf = StringWriter::new();
        let status = DefaultStatus::new(&mut buf, "Warming up".to_string(), "Running".to_string());
        let out = bench_run_x(&cfg, || (), RunLength::Count(5), status);
        assert_eq!(out.n(), 5);
    }
}
