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

pub struct BenchStatus<F1, F2> {
    pub warmup_status: F1,
    pub exec_status: F2,
}

/// Repeatedly executes closure `f`, collects the resulting latency data in a [`BenchOut`] object, and
/// *optionally* outputs information about the benchmark and its execution status.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`get_warmup_millis`] milliseconds.
///
/// Arguments:
/// - `f` - benchmark target.
/// - `exec_count` - number of executions (sample size) for the function.
/// - `warmup_status` - is invoked every so often during warm-up and can be used to output the warm-up status,
///   e.g., how much warm-up time has elapsed and the target warm-up time. The first argument is the warm-up
///   execution iteration, the second is the elapsed warm-up time, and the third is the target warm-up time.
///   (See the source code of [`bench_run_with_status`] for an example.)
/// - `pre_exec` - is invoked once at the beginning of data collection, after warm-up. It can be used,
///   for example, to output a preamble to the execution status (see `exec_status` below).
/// - `exec_status` - is invoked after each execution of `f` and can be used to output the execution
///   status, e.g., how many observations have been collected versus `exec_count`.
///   Its argument is the current number of executions performed.
///   (See the source code of [`bench_run_with_status`] for an example.)
pub fn bench_run_x(
    mut f: impl FnMut(),
    warmup_execs: usize,
    exec_count: usize,
    bench_status: Option<BenchStatus<impl FnMut(usize), impl FnMut(usize)>>,
) -> BenchOut {
    let mut state = BenchOut::default();
    let cfg = get_bench_cfg();
    let status_freq = cfg.status_freq(&mut f);
    let (warmup_status, exec_status) = match bench_status {
        Some(s) => (Some(s.warmup_status), Some(s.exec_status)),
        None => (None, None),
    };

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
        None::<BenchStatus<fn(usize), fn(usize)>>,
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
///   to output information about the function being benchmarked to `stdout` and/or `stderr`. The first
///   argument is the the `LatencyUnit` and the second argument is the `exec_count`.
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

    let bench_status = BenchStatus {
        warmup_status,
        exec_status,
    };

    let out = bench_run_x(f, warmup_execs, exec_count, Some(bench_status));
    out
}
