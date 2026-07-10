use crate::{
    BenchCfg, RunLength,
    dev_support::bsz,
    duo::DuoOut,
    multi::{self, BenchOut, LatencySrc, LatencySrc1, LatencySrc1b, LatencySrc2, LatencySrc2b},
    status::Status,
};
use std::thread;

/// Repeatedly executes both closures `f1` and `f2`, collects the resulting latency data in a [`BenchOut<2>`]
/// object, and *optionally* reports progress status during benchmark execution. Closure executions are interleaved.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// `cfg.warmup_millis` milliseconds.
///
/// Batching can be used to dilute measurement overhead, but see library documentation about the caveats
/// of batching.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f1` - first benchmark target.
/// - `f2` - second benchmark target.
/// - `run_length` - target run length (execution count and/or duration) for data collection.
/// - `s` - status handler for reporting warm-up and main execution progress.
/// - `batch` - determines whether batching is used and, if so, the batch size. `None` means no batching.
pub fn bench_run_x<'a, S: Status<'a>>(
    cfg: &BenchCfg,
    f1: impl FnMut(),
    f2: impl FnMut(),
    run_length: RunLength,
    s: S,
    batch: Option<usize>,
) -> DuoOut {
    match batch {
        None => multi::bench_run_x(cfg, LatencySrc2::new(f1, f2), run_length, s).into(),
        Some(_) => {
            multi::bench_run_x(cfg, LatencySrc2b::new(f1, f2, bsz(batch)), run_length, s).into()
        }
    }
}

/// Repeatedly executes both closures `f1` and `f2`, collects the resulting latency data in a [`BenchOut<2>`]
/// object, and *optionally* reports progress status during benchmark execution. Closure executions are interleaved.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing both closures for
/// `cfg.warmup_millis` milliseconds.
///
/// This function is equivalent to calling [`bench_run_x`] with a pre-defined no-op status object.
///
/// Batching can be used to dilute measurement overhead, but see library documentation about the caveats
/// of batching.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f1` - first benchmark target.
/// - `f2` - second benchmark target.
/// - `run_length` - target run length (execution count and/or duration) for data collection.
/// - `batch` - determines whether batching is used and, if so, the batch size. `None` means no batching.
pub fn bench_run_arg_cfg(
    cfg: &BenchCfg,
    f1: impl FnMut(),
    f2: impl FnMut(),
    run_length: RunLength,
    batch: Option<usize>,
) -> DuoOut {
    match batch {
        None => multi::bench_run_arg_cfg(cfg, LatencySrc2::new(f1, f2), run_length).into(),
        Some(_) => {
            multi::bench_run_arg_cfg(cfg, LatencySrc2b::new(f1, f2, bsz(batch)), run_length).into()
        }
    }
}

/// Executes both closures `f1` and `f2` in each iteration, collects the resulting latency data in a [`BenchOut<2>`]
/// object. Closure executions are interleaved.
/// Runs with the default [`BenchCfg`].
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing both closures for
/// `cfg.warmup_millis` milliseconds.
///
/// This function calls [`bench_run_arg_cfg`] with the default [`BenchCfg`].
///
/// Batching can be used to dilute measurement overhead, but see library documentation about the caveats
/// of batching.
///
/// Arguments:
/// - `f1` - first benchmark target.
/// - `f2` - second benchmark target.
/// - `run_length` - target run length (execution count and/or duration) for data collection.
/// - `batch` - determines whether batching is used and, if so, the batch size. `None` means no batching.
pub fn bench_run(
    f1: impl FnMut(),
    f2: impl FnMut(),
    run_length: RunLength,
    batch: Option<usize>,
) -> DuoOut {
    let cfg = BenchCfg::default();
    bench_run_arg_cfg(&cfg, f1, f2, run_length, batch).into()
}

/// Repeatedly executes both closures `f1` and `f2`, collects the resulting latency data in a [`BenchOut<2>`]
/// object, and *optionally* reports progress status during benchmark execution. Closure executions are interleaved.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing both closures for
/// `cfg.warmup_millis` milliseconds.
///
/// This function is equivalent to calling [`bench_run_x`] with a pre-defined status object that supports
/// the output of benchmark status to `stderr`.
///
/// Batching can be used to dilute measurement overhead, but see library documentation about the caveats
/// of batching.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f1` - first benchmark target.
/// - `f2` - second benchmark target.
/// - `run_length` - target run length (execution count and/or duration) for data collection.
/// - `batch` - determines whether batching is used and, if so, the batch size. `None` means no batching.
pub fn bench_run_with_status_arg_cfg(
    cfg: &BenchCfg,
    f1: impl FnMut(),
    f2: impl FnMut(),
    run_length: RunLength,
    batch: Option<usize>,
) -> DuoOut {
    match batch {
        None => {
            multi::bench_run_with_status_arg_cfg(cfg, LatencySrc2::new(f1, f2), run_length).into()
        }
        Some(_) => multi::bench_run_with_status_arg_cfg(
            cfg,
            LatencySrc2b::new(f1, f2, bsz(batch)),
            run_length,
        )
        .into(),
    }
}

/// Repeatedly executes both closures `f1` and `f2`, collects the resulting latency data in a [`BenchOut<2>`]
/// object, and *optionally* reports progress status during benchmark execution. Closure executions are interleaved.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing both closures for
/// `cfg.warmup_millis` milliseconds.
///
/// This function calls [`bench_run_with_status_arg_cfg`] with the default [`BenchCfg`].
///
/// Batching can be used to dilute measurement overhead, but see library documentation about the caveats
/// of batching.
///
/// Arguments:
/// - `f1` - first benchmark target.
/// - `f2` - second benchmark target.
/// - `run_length` - target run length (execution count and/or duration) for data collection.
/// - `batch` - determines whether batching is used and, if so, the batch size. `None` means no batching.
pub fn bench_run_with_status(
    f1: impl FnMut(),
    f2: impl FnMut(),
    run_length: RunLength,
    batch: Option<usize>,
) -> DuoOut {
    let cfg = BenchCfg::default();
    bench_run_with_status_arg_cfg(&cfg, f1, f2, run_length, batch).into()
}

/// Runs benchmarks of `f1` and `f2` on two separate threads, using [crate::bench_run_arg_cfg] on each thread.
///
/// Batching can be used to dilute measurement overhead, but see library documentation about the caveats
/// of batching.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f1` - first benchmark target.
/// - `f2` - second benchmark target.
/// - `run_length` - target run length (iteration count and/or duration) for data collection. Applies to
///   each thread.
/// - `batch` - determines whether batching is used and, if so, the batch size. `None` means no batching.
pub fn bench_run_parallel_arg_cfg(
    cfg: &BenchCfg,
    f1: impl FnMut() + Send,
    f2: impl FnMut() + Send,
    run_length: RunLength,
    batch: Option<usize>,
) -> DuoOut {
    match batch {
        None => {
            let src1 = LatencySrc1::new(f1);
            let src2 = LatencySrc1::new(f2);
            bench_run_parallel_src_arg_cfg(cfg, src1, src2, run_length)
        }
        Some(_) => {
            let src1 = LatencySrc1b::new(f1, bsz(batch));
            let src2 = LatencySrc1b::new(f2, bsz(batch));
            bench_run_parallel_src_arg_cfg(cfg, src1, src2, run_length)
        }
    }
}

/// Runs benchmarks of `f1` and `f2` on two separate threads, using [bench_run_parallel_arg_cfg],
/// with the default [`BenchCfg`].
///
/// Batching can be used to dilute measurement overhead, but see library documentation about the caveats
/// of batching.
///
/// Arguments:
/// - `f1` - first benchmark target.
/// - `f2` - second benchmark target.
/// - `run_length` - target run length (iteration count and/or duration) for data collection. Applies to
///   each thread.
/// - `batch` - determines whether batching is used and, if so, the batch size. `None` means no batching.
pub fn bench_run_parallel(
    f1: impl FnMut() + Send,
    f2: impl FnMut() + Send,
    run_length: RunLength,
    batch: Option<usize>,
) -> DuoOut {
    let cfg = BenchCfg::default();
    bench_run_parallel_arg_cfg(&cfg, f1, f2, run_length, batch)
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
