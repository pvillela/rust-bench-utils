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
        // Used in control of the exit from the iteration loop when both `status_freq` and `exec_count` are too high
        // compared to `run_length`.
        est_count_from_dur: usize,
        mut exec_status: Option<impl FnMut(usize)>,
    ) {
        assert!(status_freq > 0, "status_freq must be > 0");

        let (exec_count, run_time) = run_length.get_exec_count_and_duration();
        assert!(exec_count > 0, "exec_count must be > 0");

        let unit = BenchCfg::get().recording_unit();
        let mut est_remaining_iters = est_count_from_dur;
        let start = Instant::now();

        for i in 1..=exec_count {
            let latency = unit.latency_as_u64(latency(&mut f));
            self.capture_data(latency);
            if est_remaining_iters > 0 {
                est_remaining_iters -= 1;
            }

            if i % status_freq == 0 || i == exec_count || est_remaining_iters == 0 {
                let elapsed = start.elapsed();
                let finished = i == exec_count || elapsed >= run_time;

                if i % status_freq == 0 || finished {
                    if let Some(ref mut exec_status) = exec_status {
                        exec_status(i);
                    }
                }

                if finished {
                    break;
                }

                if est_remaining_iters == 0 {
                    let remaining_time = run_time - elapsed;
                    let avg_time_per_iter = elapsed / i as u32;
                    est_remaining_iters =
                        remaining_time.div_duration_f64(avg_time_per_iter).ceil() as usize;
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
    exec_run_length: RunLength,
    warmup_status: Option<impl FnMut(usize)>,
    exec_status: Option<impl FnMut(usize)>,
    execs_per_milli: f64,
) -> BenchOut {
    let cfg = BenchCfg::get();
    let mut state = BenchOut::new(&cfg);
    let status_freq = cfg.status_freq(execs_per_milli);

    let warmup_run_length = RunLength::Duration(Duration::from_millis(cfg.warmup_millis()));
    let warmup_est_count = warmup_run_length.estimated_count(execs_per_milli);
    let exec_est_count = exec_run_length.estimated_count(execs_per_milli);

    println!("*** status_freq={status_freq}");
    println!("*** warmup_est_count={warmup_est_count}");
    // Warm-up.
    state.execute(
        &mut f,
        warmup_run_length,
        status_freq,
        warmup_est_count,
        warmup_status,
    );
    state.reset();

    state.execute(f, exec_run_length, status_freq, exec_est_count, exec_status);

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
    let execs_per_milli = cfg.execs_per_milli(&mut f);

    bench_run_x(
        f,
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
    println!("*** execs_per_milli={execs_per_milli}");
    let warmup_millis = cfg.warmup_millis();
    let warmup_run_length = RunLength::Duration(Duration::from_millis(warmup_millis));
    let warmup_est_count = warmup_run_length.estimated_count(execs_per_milli);
    let warmup_status = status("Warming up", warmup_millis, warmup_est_count);

    let exec_est_count = exec_run_length.estimated_count(execs_per_milli);
    let exec_est_millis = exec_run_length
        .estimated_duration(execs_per_milli)
        .as_millis() as u64;
    // The `\n` below is to separate warmup status from exec status. Otherwise, they get mixed up due to
    // the `eprint!("{}", "\u{8}".repeat(status_len))` line in the `status` closure.
    let exec_status = status("\nExecuting bench_run", exec_est_millis, exec_est_count);

    header(exec_est_count);

    let out = bench_run_x(
        f,
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
/// Crappy tests created by Claude Code, improved a bit by me.
mod test {
    use super::*;
    use crate::{LatencyUnit, RunLength, test_support::with_safe_bench_cfg};
    use std::{thread, time::Duration};

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
        let out = with_safe_bench_cfg(|| {
            let _cfg = minimal_cfg_snapshot();

            bench_run(
                || thread::sleep(Duration::from_nanos(1)),
                RunLength::Count(5),
            )
        });
        // With 5 count and no timeout, we should have exactly 5 iterations
        assert_eq!(out.n(), 5);
    }

    #[test]
    fn test_bench_run_x() {
        let out = with_safe_bench_cfg(|| {
            let cfg = minimal_cfg_snapshot();
            // Use the snapshot cfg for calibration
            let execs_per_milli = cfg.execs_per_milli(|| thread::sleep(Duration::from_nanos(1)));

            bench_run_x(
                || {},
                RunLength::Count(10),
                None::<fn(usize)>,
                None::<fn(usize)>,
                execs_per_milli,
            )
        });

        assert_eq!(out.n(), 10);
    }

    #[test]
    fn test_bench_run_with_timeout() {
        let out = with_safe_bench_cfg(|| {
            let _cfg = minimal_cfg_snapshot();

            // Use a very short timeout that should be exceeded immediately
            bench_run(
                || thread::sleep(Duration::from_nanos(1)),
                RunLength::Duration(Duration::from_nanos(1)),
            )
        });
        // At least some executions should have been captured
        assert!(out.n() > 0);
    }
}
