//! Implements functions to collect latency statistics for a closure.

use crate::{BenchCfg, BenchOut, RunLength, latency};
use std::{
    io::{Write, stderr},
    time::{Duration, Instant},
};

type BenchState = BenchOut;

impl BenchState {
    /// Executes `f` repeatedly and captures latencies.
    /// `exec_status` is invoked once every `status_freq` invocations of `f`.
    fn execute(
        &mut self,
        mut f: impl FnMut(),
        run_length: RunLength,
        status_freq: usize,
        mut exec_status: Option<impl FnMut(usize)>,
    ) {
        assert!(status_freq > 0, "status_freq must be > 0");

        let (exec_count, run_time) = run_length.get_exec_count_and_duration();
        assert!(exec_count > 0, "exec_count must be > 0");

        let unit = BenchCfg::get().recording_unit();
        let start = Instant::now();

        for i in 1..=exec_count {
            let latency = unit.latency_as_u64(latency(&mut f));
            self.capture_data(latency);

            if i % status_freq == 0 || i == exec_count {
                if let Some(ref mut exec_status) = exec_status {
                    exec_status(i);
                }

                if start.elapsed().ge(&run_time) {
                    break;
                }
            }
        }
    }
}

/// Repeatedly executes closure `f`, collects the resulting latency data in a [`BenchOut`] object, and
/// *optionally* outputs information about the benchmark and its execution status.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// `warmup_millis` milliseconds.
///
/// Arguments:
/// - `f` - benchmark target.
/// - `warmup_millis` - duration (in milliseconds) of warm-up execution.
/// - `exec_run_length` - target run length (iteration count and/or duration) for data collection.
/// - `warmup_status` - optionally invoked periodically during warm-up. Its argument is the current
///   warm-up execution iteration.
/// - `exec_status` - optionally invoked periodically during data collection. Its argument is the
///   current number of executions performed.
/// - `execs_per_milli` - estimate of how many executions of `f` fit in one millisecond.
pub fn bench_run_x(
    mut f: impl FnMut(),
    warmup_millis: u64,
    exec_run_length: RunLength,
    warmup_status: Option<impl FnMut(usize)>,
    exec_status: Option<impl FnMut(usize)>,
    execs_per_milli: f64,
) -> BenchOut {
    let mut state = BenchOut::new(&BenchCfg::get());
    let cfg = BenchCfg::get();
    let status_freq = cfg.status_freq(execs_per_milli);

    // Warm-up.
    state.execute(
        &mut f,
        RunLength::Duration(Duration::from_millis(warmup_millis)),
        status_freq,
        warmup_status,
    );
    state.reset();

    state.execute(f, exec_run_length, status_freq, exec_status);

    state
}

/// Repeatedly executes closure `f` and collects the resulting latency data in a [`BenchOut`] object.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
/// This function calls [`bench_run_x`] with no-op closures for the arguments that support the output of
/// benchmark status.
///
/// Arguments:
/// - `f` - benchmark target.
/// - `exec_run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run(mut f: impl FnMut(), exec_run_length: RunLength) -> BenchOut {
    let cfg = BenchCfg::get();
    let warmup_millis = cfg.warmup_millis();
    let execs_per_milli = cfg.execs_per_milli(&mut f);

    bench_run_x(
        f,
        warmup_millis,
        exec_run_length,
        None::<fn(usize)>,
        None::<fn(usize)>,
        execs_per_milli,
    )
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
/// - `f` - benchmark target.
/// - `exec_run_length` - target run length (iteration count and/or duration) for data collection.
/// - `header` - is invoked once at the start of this function's execution; it can be used, for example,
///   to output information about the function being benchmarked to `stdout` and/or `stderr`. Its argument
///   is the estimated execution count.
pub fn bench_run_with_status(
    mut f: impl FnMut(),
    exec_run_length: RunLength,
    header: impl FnOnce(usize),
) -> BenchOut {
    let cfg = BenchCfg::get();

    let status = |preamble: &'static str, millis: u64, count: usize| {
        let mut status_len: usize = 0;

        move |i: usize| {
            if status_len == 0 {
                eprint!("{preamble} for (approx.) {millis} millis: ");
                stderr().flush().expect("unexpected I/O error");
            }
            eprint!("{}", "\u{8}".repeat(status_len));
            let status = format!("{i} of (approx.) {count} executions.");
            status_len = status.len();
            eprint!("{status}");
            stderr().flush().expect("unexpected I/O error");
        }
    };

    let execs_per_milli = cfg.execs_per_milli(&mut f);

    let warmup_millis = cfg.warmup_millis();
    let warmup_run_length = RunLength::Duration(Duration::from_millis(warmup_millis));
    let warmup_est_count = warmup_run_length.estimated_count(execs_per_milli);
    let warmup_status = status("Warming up", warmup_millis, warmup_est_count);

    let exec_count = exec_run_length.estimated_count(execs_per_milli);
    let exec_millis = exec_run_length
        .estimated_duration(execs_per_milli)
        .as_millis() as u64;
    // The `\n` below is to separate warmup status from exec status. Otherwise, they get mixed up due to
    // the `eprint!("{}", "\u{8}".repeat(status_len))` line in the `status` closure.
    let exec_status = status("\nExecuting bench_run", exec_millis, exec_count);

    header(exec_count);

    let out = bench_run_x(
        f,
        warmup_millis,
        exec_run_length,
        Some(warmup_status),
        Some(exec_status),
        execs_per_milli,
    );
    eprintln!();
    out
}

#[cfg(test)]
#[cfg(feature = "_test_support")]
mod test {
    use super::*;
    use crate::{LatencyUnit, RunLength};
    use std::time::Duration;

    /// Helper to get a clean config with minimal warmup/calibration for fast tests.
    fn minimal_cfg_snapshot() -> BenchCfg {
        let cfg = BenchCfg::get();
        cfg.with_warmup_millis(0)
            .with_status_millis(1)
            .with_recording_unit(LatencyUnit::Nano)
            .with_reporting_unit(LatencyUnit::Nano)
            .set();
        BenchCfg::get()
    }

    #[test]
    fn test_bench_run_with_count() {
        let saved_cfg = BenchCfg::get();
        let _cfg = minimal_cfg_snapshot();

        let out = bench_run(|| {}, RunLength::Count(5));
        // With 5 count and no timeout, we should have exactly 5 iterations
        assert_eq!(out.n(), 5);

        // Reset config
        saved_cfg.set();
    }

    #[test]
    fn test_bench_run_x() {
        let saved_cfg = BenchCfg::get();
        let _cfg = minimal_cfg_snapshot();
        // Use the snapshot cfg for calibration
        let cfg = BenchCfg::get();
        let execs_per_milli = cfg.execs_per_milli(|| {});

        let out = bench_run_x(
            || {},
            0,
            RunLength::Count(10),
            None::<fn(usize)>,
            None::<fn(usize)>,
            execs_per_milli,
        );
        assert_eq!(out.n(), 10);

        saved_cfg.set();
    }

    #[test]
    fn test_bench_run_with_timeout() {
        let saved_cfg = BenchCfg::get();
        let _cfg = minimal_cfg_snapshot();

        // Use a very short timeout that should be exceeded immediately
        let out = bench_run(|| {}, RunLength::Duration(Duration::from_nanos(1)));
        // At least some executions should have been captured
        assert!(out.n() > 0);

        saved_cfg.set();
    }
}
