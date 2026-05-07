//! Implements functions to collect latency statistics for a closure.

use crate::{BenchCfg, BenchOut, LatencyUnit, latency};
use std::{
    io::{Write, stderr},
    ops::Deref,
    sync::Mutex,
};

static BENCH_CFG: Mutex<BenchCfg> = Mutex::new(BenchCfg::new(
    3000,
    LatencyUnit::Nano,
    LatencyUnit::Micro,
    3,
    3,
    1000,
    &BENCH_CFG,
));

pub fn get_bench_cfg() -> BenchCfg {
    let guard = BENCH_CFG.lock().unwrap();
    guard.deref().clone()
}

type BenchState = BenchOut;

impl BenchState {
    /// Executes `f` repeatedly and captures latencies.
    /// `exec_status` is invoked once every `status_freq` invocations of `f`.
    fn execute(
        &mut self,
        mut f: impl FnMut(),
        exec_count: usize,
        status_freq: usize,
        mut exec_status: Option<impl FnMut(usize)>,
    ) {
        assert!(status_freq > 0, "status_freq must be > 0");

        let unit = get_bench_cfg().recording_unit();

        for i in 1..=exec_count {
            let latency = unit.latency_as_u64(latency(&mut f));
            self.capture_data(latency);

            if i % status_freq == 0 || i == exec_count {
                if let Some(ref mut exec_status) = exec_status {
                    exec_status(i);
                }
            }
        }
    }
}

/// Repeatedly executes closure `f`, collects the resulting latency data in a [`BenchOut`] object, and
/// *optionally* outputs information about the benchmark and its execution status.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`get_warmup_millis`] milliseconds.
///
/// Arguments:
/// - `f` - benchmark target.
/// - `warmup_execs` - number of warm-up executions to perform.
/// - `exec_count` - number of executions (sample size) for the function.
/// - `warmup_status` - optionally invoked periodically during warm-up. Its argument is the current
///   warm-up execution iteration.
/// - `exec_status` - optionally invoked periodically during data collection. Its argument is the
///   current number of executions performed.
pub fn bench_run_x(
    mut f: impl FnMut(),
    warmup_execs: usize,
    exec_count: usize,
    warmup_status: Option<impl FnMut(usize)>,
    exec_status: Option<impl FnMut(usize)>,
) -> BenchOut {
    let mut state = BenchOut::default();
    let cfg = get_bench_cfg();
    let status_freq = cfg.status_freq(&mut f);

    // Warm-up.
    state.execute(&mut f, warmup_execs, status_freq, warmup_status);
    state.reset();

    state.execute(f, exec_count, status_freq, exec_status);

    state
}

/// Repeatedly executes closure `f` and collects the resulting latency data in a [`BenchOut`] object.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`get_warmup_millis`] milliseconds.
/// This function calls [`bench_run_x`] with no-op closures for the arguments that support the output of
/// benchmark status.
///
/// Arguments:
/// - `f` - benchmark target.
/// - `exec_count` - number of executions (sample size) for the function.
pub fn bench_run(mut f: impl FnMut(), exec_count: usize) -> BenchOut {
    let warmup_execs = get_bench_cfg().warmup_execs(&mut f);

    bench_run_x(
        f,
        warmup_execs,
        exec_count,
        None::<fn(usize)>,
        None::<fn(usize)>,
    )
}

/// Repeatedly executes closure `f`, collects the resulting latency data in a [`BenchOut`] object, and
/// outputs information about the benchmark and its execution status.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`get_warmup_millis`] milliseconds.
/// This function calls [`bench_run_x`] with pre-defined closures for the arguments that support the output of
/// benchmark status to `stderr`.
///
/// Arguments:
/// - `f` - benchmark target.
/// - `exec_count` - number of executions (sample size) for the function.
/// - `header` - is invoked once at the start of this function's execution; it can be used, for example,
///   to output information about the function being benchmarked to `stdout` and/or `stderr`. Its argument
///   is `exec_count`.
pub fn bench_run_with_status(
    mut f: impl FnMut(),
    exec_count: usize,
    header: impl FnOnce(usize),
) -> BenchOut {
    header(exec_count);

    let cfg = get_bench_cfg();
    let warmup_execs = cfg.warmup_execs(&mut f);

    let warmup_status = {
        let mut status_len: usize = 0;
        let warmup_millis = cfg.warmup_millis();

        move |i: usize| {
            if status_len == 0 {
                eprint!("Warming up for approximately {warmup_millis} millis: ");
                stderr().flush().expect("unexpected I/O error");
            }
            eprint!("{}", "\u{8}".repeat(status_len));
            let status = format!("{i} of {warmup_execs}.");
            status_len = status.len();
            eprint!("{status}");
            stderr().flush().expect("unexpected I/O error");
        }
    };

    let exec_status = {
        let mut status_len: usize = 0;

        move |i: usize| {
            if status_len == 0 {
                eprint!(" Executing bench_run: ");
                stderr().flush().expect("unexpected I/O error");
            }
            eprint!("{}", "\u{8}".repeat(status_len));
            let status = format!("{i} of {exec_count}. ");
            status_len = status.len();
            eprint!("{status}");
            stderr().flush().expect("unexpected I/O error");
        }
    };

    let out = bench_run_x(
        f,
        warmup_execs,
        exec_count,
        Some(warmup_status),
        Some(exec_status),
    );
    eprintln!();
    out
}
