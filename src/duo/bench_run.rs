use std::thread;

use crate::{
    BenchCfg, RunLength,
    duo::DuoOut,
    multi::{self, BenchOut, LatencySrc, LatencySrc1, LatencySrc1b, LatencySrc2, LatencySrc2b},
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
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
/// - `s` - status handler for reporting warm-up and execution progress.
pub fn bench_run_x<'a, S: Status<'a>>(
    cfg: &BenchCfg,
    f1: impl FnMut(),
    f2: impl FnMut(),
    run_length: RunLength,
    s: S,
) -> DuoOut {
    multi::bench_run_x(cfg, LatencySrc2::new(f1, f2), run_length, s).into()
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
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run(f1: impl FnMut(), f2: impl FnMut(), run_length: RunLength) -> DuoOut {
    multi::bench_run(LatencySrc2::new(f1, f2), run_length).into()
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
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run_arg_cfg(
    cfg: &BenchCfg,
    f1: impl FnMut(),
    f2: impl FnMut(),
    run_length: RunLength,
) -> DuoOut {
    multi::bench_run_arg_cfg(cfg, LatencySrc2::new(f1, f2), run_length).into()
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
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run_with_status(f1: impl FnMut(), f2: impl FnMut(), run_length: RunLength) -> DuoOut {
    multi::bench_run_with_status(LatencySrc2::new(f1, f2), run_length).into()
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
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run_with_status_arg_cfg(
    cfg: &BenchCfg,
    f1: impl FnMut(),
    f2: impl FnMut(),
    run_length: RunLength,
) -> DuoOut {
    multi::bench_run_with_status_arg_cfg(cfg, LatencySrc2::new(f1, f2), run_length).into()
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
    run_length: RunLength,
    s: S,
    batch: u32,
) -> DuoOut {
    multi::bench_run_x(cfg, LatencySrc2b::new(f1, f2, batch), run_length, s).into()
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
    run_length: RunLength,
    batch: u32,
) -> DuoOut {
    multi::bench_run(LatencySrc2b::new(f1, f2, batch), run_length).into()
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
    run_length: RunLength,
    batch: u32,
) -> DuoOut {
    multi::bench_run_arg_cfg(cfg, LatencySrc2b::new(f1, f2, batch), run_length).into()
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
    run_length: RunLength,
    batch: u32,
) -> DuoOut {
    multi::bench_run_with_status(LatencySrc2b::new(f1, f2, batch), run_length).into()
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
    run_length: RunLength,
    batch: u32,
) -> DuoOut {
    multi::bench_run_with_status_arg_cfg(cfg, LatencySrc2b::new(f1, f2, batch), run_length).into()
}

/// Runs benchmarks of `f1` and `f2` on two separate threads, using [bench_run_parallel_arg_cfg],
/// with the default [`BenchCfg`].
///
/// Arguments:
/// - `f1` - first benchmark target.
/// - `f2` - second benchmark target.
/// - `run_length` - target run length (iteration count and/or duration) for data collection. Applies to
///   each thread.
pub fn bench_run_parallel(
    f1: impl FnMut() + Send,
    f2: impl FnMut() + Send,
    run_length: RunLength,
) -> DuoOut {
    let cfg = BenchCfg::default();
    bench_run_parallel_arg_cfg(&cfg, f1, f2, run_length)
}

/// Runs benchmarks of `f1` and `f2` on two separate threads, using [crate::bench_run_arg_cfg] on each thread.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f1` - first benchmark target.
/// - `f2` - second benchmark target.
/// - `run_length` - target run length (iteration count and/or duration) for data collection. Applies to
///   each thread.
pub fn bench_run_parallel_arg_cfg(
    cfg: &BenchCfg,
    f1: impl FnMut() + Send,
    f2: impl FnMut() + Send,
    run_length: RunLength,
) -> DuoOut {
    let src1 = LatencySrc1::new(f1);
    let src2 = LatencySrc1::new(f2);
    bench_run_parallel_src_arg_cfg(cfg, src1, src2, run_length)
}

/// Similar to [`bench_run_parallel`] but batches the executions of `f1` and `f2` into groups of size `batch`.
///
/// Batching may reduce measurement overhead.
/// Each batch results in the batch average being collected `batch` times, so the number of captured
/// latency values is not impacted by grouping.
/// However, a potential consequence is that the statistical tests provided by [`DuoOut`] may be somewhat
/// distorted as the resulting distributions may no longer be approximately logormal.
pub fn bench_run_parallel_b(
    f1: impl FnMut() + Send,
    f2: impl FnMut() + Send,
    run_length: RunLength,
    batch: u32,
) -> DuoOut {
    let cfg = BenchCfg::default();
    bench_run_parallel_arg_cfg_b(&cfg, f1, f2, run_length, batch)
}

/// Similar to [`bench_run_parallel_arg_cfg`] but batches the executions of `f1` and `f2` into groups of size `batch`.
///
/// Batching may reduce measurement overhead.
/// Each batch results in the batch average being collected `batch` times, so the number of captured
/// latency values is not impacted by grouping.
/// However, a potential consequence is that the statistical tests provided by [`DuoOut`] may be somewhat
/// distorted as the resulting distributions may no longer be approximately logormal.
pub fn bench_run_parallel_arg_cfg_b(
    cfg: &BenchCfg,
    f1: impl FnMut() + Send,
    f2: impl FnMut() + Send,
    run_length: RunLength,
    batch: u32,
) -> DuoOut {
    let src1 = LatencySrc1b::new(f1, batch);
    let src2 = LatencySrc1b::new(f2, batch);
    bench_run_parallel_src_arg_cfg(cfg, src1, src2, run_length)
}

#[doc(hidden)]
/// Runs benchmarks of `src1` and `src2` on two separate threads, using [multi::bench_run_arg_cfg] on each thread.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `src1` - first latency source.
/// - `src2` - second latency source.
/// - `run_length` - target run length (iteration count and/or duration) for data collection. Applies to
///   each thread.
pub fn bench_run_parallel_src_arg_cfg(
    cfg: &BenchCfg,
    src1: impl LatencySrc<1> + Send,
    src2: impl LatencySrc<1> + Send,
    run_length: RunLength,
) -> DuoOut {
    let (out1, out2) = thread::scope(|s| {
        let h1 = s.spawn(|| multi::bench_run_arg_cfg(&cfg, src1, run_length));
        let h2 = s.spawn(|| multi::bench_run_arg_cfg(&cfg, src2, run_length));

        let out1 = h1.join().expect("thread running bench for `f1` panicked");
        let out2 = h2.join().expect("thread running bench for `f2` panicked");

        (out1, out2)
    });

    BenchOut {
        arr: [out1.into(), out2.into()],
    }
}

#[cfg(feature = "_test_support")]
pub fn bench_run_x_o<'a, S: Status<'a>>(
    cfg: &BenchCfg,
    f1: impl FnMut(),
    f2: impl FnMut(),
    run_length: RunLength,
    s: S,
    batch: Option<u32>,
) -> DuoOut {
    match batch {
        None => bench_run_x(&cfg, f1, f2, run_length, s),
        Some(batch) => bench_run_x_b(&cfg, f1, f2, run_length, s, batch),
    }
}

#[cfg(feature = "_test_support")]
pub fn bench_run_arg_cfg_o(
    cfg: &BenchCfg,
    f1: impl FnMut(),
    f2: impl FnMut(),
    run_length: RunLength,
    batch: Option<u32>,
) -> DuoOut {
    match batch {
        None => bench_run_arg_cfg(&cfg, f1, f2, run_length),
        Some(batch) => bench_run_arg_cfg_b(&cfg, f1, f2, run_length, batch),
    }
}

#[cfg(feature = "_test_support")]
pub fn bench_run_with_status_arg_cfg_o(
    cfg: &BenchCfg,
    f1: impl FnMut(),
    f2: impl FnMut(),
    run_length: RunLength,
    batch: Option<u32>,
) -> DuoOut {
    match batch {
        None => bench_run_with_status_arg_cfg(&cfg, f1, f2, run_length),
        Some(batch) => bench_run_with_status_arg_cfg_b(&cfg, f1, f2, run_length, batch),
    }
}
