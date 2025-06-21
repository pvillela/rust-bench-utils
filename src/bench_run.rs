//! Implements functions to collect latency statistics for a closure.

use crate::{BenchCfg, BenchOut, LatencyUnit, latency};
use std::{
    io::{Write, stderr},
    ops::Deref,
    sync::Mutex,
    time::{Duration, Instant},
};

static BENCH_CFG: Mutex<BenchCfg> = Mutex::new(BenchCfg::new(
    3000,
    LatencyUnit::Nano,
    LatencyUnit::Micro,
    3,
    &BENCH_CFG,
));

pub fn get_bench_cfg() -> BenchCfg {
    let guard = BENCH_CFG.lock().unwrap();
    guard.deref().clone()
}

const WARMUP_INCREMENT_COUNT: usize = 20;

type BenchState = BenchOut;

impl BenchState {
    /// Executes `f` repeatedly and captures latencies.
    /// `pre_exec` is invoked once just before the invocation of `f`, and `exec_status` is invoked at the
    /// end of each invocation.
    fn execute(
        &mut self,
        mut f: impl FnMut(),
        exec_count: usize,
        pre_exec: impl FnOnce(),
        mut exec_status: impl FnMut(usize),
        init_status_count: usize,
    ) {
        pre_exec();

        let unit = get_bench_cfg().recording_unit();
        for i in 1..=exec_count {
            let elapsed = unit.latency_as_u64(latency(&mut f));
            self.capture_data(elapsed);
            exec_status(init_status_count + i);
        }
    }

    /// Warms-up the benchmark by invoking [`Self::execute`] repeatedly, each time with an `exec_count` value of
    /// [`WARMUP_INCREMENT_COUNT`], until the globally set number of warm-up millisecods [`WARMUP_MILLIS`] is
    /// reached or exceeded. `warmup_status` is invoked at the end of each invocation of [`Self::execute`].
    fn warmup(&mut self, mut f: impl FnMut(), mut warmup_status: impl FnMut(usize, u64, u64)) {
        let warmup_millis = get_bench_cfg().warmup_millis();
        let start = Instant::now();
        for i in 1.. {
            self.execute(&mut f, WARMUP_INCREMENT_COUNT, || {}, |_| {}, 0);
            let elapsed = Instant::now().duration_since(start);
            warmup_status(i, elapsed.as_millis() as u64, warmup_millis);
            if elapsed.ge(&Duration::from_millis(warmup_millis)) {
                break;
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
    exec_count: usize,
    mut warmup_status: impl FnMut(usize, u64, u64),
    pre_exec: impl FnOnce(),
    mut exec_status: impl FnMut(usize),
) -> BenchOut {
    let mut state = BenchOut::default();

    state.warmup(&mut f, &mut warmup_status);
    state.reset();
    state.execute(&mut f, exec_count, pre_exec, &mut exec_status, 0);

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
pub fn bench_run(f: impl FnMut(), exec_count: usize) -> BenchOut {
    bench_run_x(f, exec_count, |_, _, _| {}, || (), |_| ())
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
    f: impl FnMut(),
    exec_count: usize,
    header: impl FnOnce(usize),
) -> BenchOut {
    header(exec_count);

    let warmup_status = {
        let mut status_len: usize = 0;

        move |_: usize, elapsed_millis: u64, warmup_millis: u64| {
            if status_len == 0 {
                eprint!("Warming up ... ");
                stderr().flush().expect("unexpected I/O error");
            }
            eprint!("{}", "\u{8}".repeat(status_len));
            let status = format!("{elapsed_millis} millis of {warmup_millis}.");
            if elapsed_millis.lt(&warmup_millis) {
                status_len = status.len();
            } else {
                status_len = 0; // reset status in case of multiple warm-up phases
            };
            eprint!("{status}");
            stderr().flush().expect("unexpected I/O error");
        }
    };

    let pre_exec = || {
        eprint!(" Executing bench_run ... ");
        stderr().flush().expect("unexpected I/O error");
    };

    let exec_status = {
        let mut status_len: usize = 0;

        move |i| {
            eprint!("{}", "\u{8}".repeat(status_len));
            let status = format!("{i} of {exec_count}.");
            status_len = status.len();
            eprint!("{status}");
            stderr().flush().expect("unexpected I/O error");
        }
    };

    let out = bench_run_x(f, exec_count, warmup_status, pre_exec, exec_status);
    println!();
    out
}
