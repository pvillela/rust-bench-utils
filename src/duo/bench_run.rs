use std::thread;

use crate::{
    BenchCfg, RunLength,
    duo::DuoOut,
    multi::BenchOut,
    multi::{self, LatencySrc2, LatencySrc2b},
    status::Status,
};

/// Executes both closures `f1` and `f2` in each iteration, collects the resulting latency data in a [`BenchOut<2>`]
/// object, and *optionally* reports progress status during benchmark execution. Closure executions are interleaved.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f1` - first benchmark target.
/// - `f2` - second benchmark target.
/// - `exec_run_length` - target run length (iteration count and/or duration) for data collection.
/// - `s` - status handler for reporting warm-up and execution progress.
pub fn bench_run_x<'a, S: Status<'a>>(
    cfg: &BenchCfg,
    f1: impl FnMut(),
    f2: impl FnMut(),
    exec_run_length: RunLength,
    s: S,
) -> DuoOut {
    multi::bench_run_x(cfg, LatencySrc2::new(f1, f2), exec_run_length, s).into()
}

/// Executes both closures `f1` and `f2` in each iteration, collects the resulting latency data in a [`BenchOut<2>`]
/// object. Closure executions are interleaved.
/// Runs with the default [`BenchCfg`].
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
/// This function calls [`bench_run_x`] with no-op closures for the arguments that support the output of
/// benchmark status.
///
/// Arguments:
/// - `f1` - first benchmark target.
/// - `f2` - second benchmark target.
/// - `exec_run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run(f1: impl FnMut(), f2: impl FnMut(), exec_run_length: RunLength) -> DuoOut {
    multi::bench_run(LatencySrc2::new(f1, f2), exec_run_length).into()
}

/// Executes both closures `f1` and `f2` in each iteration, collects the resulting latency data in a [`BenchOut<2>`]
/// object. Closure executions are interleaved.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
/// This function calls [`bench_run_x`] with no-op closures for the arguments that support the output of
/// benchmark status.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f1` - first benchmark target.
/// - `f2` - second benchmark target.
/// - `exec_run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run_arg_cfg(
    cfg: &BenchCfg,
    f1: impl FnMut(),
    f2: impl FnMut(),
    exec_run_length: RunLength,
) -> DuoOut {
    multi::bench_run_arg_cfg(cfg, LatencySrc2::new(f1, f2), exec_run_length).into()
}

/// Executes both closures `f1` and `f2` in each iteration, collects the resulting latency data in a [`BenchOut<2>`]
/// object, and outputs information about the benchmark and its execution status. Closure executions are interleaved.
/// Runs with the default [`BenchCfg`].
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
/// This function calls [`bench_run_x`] with pre-defined closures for the arguments that support the output of
/// benchmark status to `stderr`.
///
/// Arguments:
/// - `f1` - first benchmark target.
/// - `f2` - second benchmark target.
/// - `exec_run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run_with_status(
    f1: impl FnMut(),
    f2: impl FnMut(),
    exec_run_length: RunLength,
) -> DuoOut {
    multi::bench_run_with_status(LatencySrc2::new(f1, f2), exec_run_length).into()
}

/// Executes both closures `f1` and `f2` in each iteration, collects the resulting latency data in a [`BenchOut<2>`]
/// object, and outputs information about the benchmark and its execution status. Closure executions are interleaved.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
/// This function calls [`bench_run_x`] with pre-defined closures for the arguments that support the output of
/// benchmark status to `stderr`.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f1` - first benchmark target.
/// - `f2` - second benchmark target.
/// - `exec_run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run_with_status_arg_cfg(
    cfg: &BenchCfg,
    f1: impl FnMut(),
    f2: impl FnMut(),
    exec_run_length: RunLength,
) -> DuoOut {
    multi::bench_run_with_status_arg_cfg(cfg, LatencySrc2::new(f1, f2), exec_run_length).into()
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
    f1: impl FnMut(),
    f2: impl FnMut(),
    exec_run_length: RunLength,
    s: S,
    batch: u32,
) -> DuoOut {
    multi::bench_run_x(cfg, LatencySrc2b::new(f1, f2, batch), exec_run_length, s).into()
}

/// Similar to [`bench_run`] but batches the executions of `f` into groups of size `batch`.
///
/// Batching may reduce measurement overhead.
/// Each batch results in the batch average being collected `batch` times, so the number of captured
/// latency values is not impacted by grouping.
/// However, a potential consequence is that the statistical tests provided by [`BenchOut`] may be somewhat
/// distorted as the resulting distribution may no longer be approximately logormal.
pub fn bench_run_b(
    f1: impl FnMut(),
    f2: impl FnMut(),
    exec_run_length: RunLength,
    batch: u32,
) -> DuoOut {
    multi::bench_run(LatencySrc2b::new(f1, f2, batch), exec_run_length).into()
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
    f1: impl FnMut(),
    f2: impl FnMut(),
    exec_run_length: RunLength,
    batch: u32,
) -> DuoOut {
    multi::bench_run_arg_cfg(cfg, LatencySrc2b::new(f1, f2, batch), exec_run_length).into()
}

/// Similar to [`bench_run_with_status`] but batches the executions of `f` into groups of size `batch`.
///
/// Batching may reduce measurement overhead.
/// Each batch results in the batch average being collected `batch` times, so the number of captured
/// latency values is not impacted by grouping.
/// However, a potential consequence is that the statistical tests provided by [`BenchOut`] may be somewhat
/// distorted as the resulting distribution may no longer be approximately logormal.
pub fn bench_run_with_status_b(
    f1: impl FnMut(),
    f2: impl FnMut(),
    exec_run_length: RunLength,
    batch: u32,
) -> DuoOut {
    multi::bench_run_with_status(LatencySrc2b::new(f1, f2, batch), exec_run_length).into()
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
    f1: impl FnMut(),
    f2: impl FnMut(),
    exec_run_length: RunLength,
    batch: u32,
) -> DuoOut {
    multi::bench_run_with_status_arg_cfg(cfg, LatencySrc2b::new(f1, f2, batch), exec_run_length)
        .into()
}

/// Runs benchmarks of `f1` and `f2` on two separate threads, using [bench_run_parallel_arg_cfg],
/// with the default [`BenchCfg`].
///
/// Arguments:
/// - `f1` - first benchmark target.
/// - `f2` - second benchmark target.
/// - `exec_run_length` - target run length (iteration count and/or duration) for data collection. Applies to
///   each thread.
pub fn bench_run_parallel(
    f1: impl FnMut() + Send,
    f2: impl FnMut() + Send,
    exec_run_length: RunLength,
) -> DuoOut {
    let cfg = BenchCfg::default();
    bench_run_parallel_arg_cfg(&cfg, f1, f2, exec_run_length)
}

/// Runs benchmarks of `f1` and `f2` on two separate threads, using [crate::bench_run_arg_cfg] on each thread.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f1` - first benchmark target.
/// - `f2` - second benchmark target.
/// - `exec_run_length` - target run length (iteration count and/or duration) for data collection. Applies to
///   each thread.
pub fn bench_run_parallel_arg_cfg(
    cfg: &BenchCfg,
    f1: impl FnMut() + Send,
    f2: impl FnMut() + Send,
    exec_run_length: RunLength,
) -> DuoOut {
    let (out1, out2) = thread::scope(|s| {
        let h1 = s.spawn(|| crate::bench_run_arg_cfg(&cfg, f1, exec_run_length));
        let h2 = s.spawn(|| crate::bench_run_arg_cfg(&cfg, f2, exec_run_length));

        let out1 = h1.join().expect("thread running bench for `f1` panicked");
        let out2 = h2.join().expect("thread running bench for `f2` panicked");

        (out1, out2)
    });

    BenchOut {
        arity: 2,
        arr: [out1, out2],
    }
}
