//! Implements functions to collect latency statistics for a closure.

use crate::{BenchCfg, BenchOut, LatencyUnit, latency};
use std::{
    io::{Write, stderr},
    ops::Deref,
    sync::Mutex,
    time::Instant,
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

/// Execution finishing criterion.
#[derive(PartialEq)]
enum FinishCrit {
    Count(usize),
    Millis(u64),
}

/// Reporting status for `BenchState::execute` method.
struct ExecStatus<F1, F2> {
    pre_exec: F1,
    exec_status: F2,
}

impl BenchState {
    /// Executes `f` repeatedly and captures latencies.
    /// if `rept_status.is_some()`, `pre_exec` is invoked once just before the executions,
    /// and `exec_status` is invoked once every `status_freq` invocations of `f`.
    fn execute(
        &mut self,
        mut f: impl FnMut(),
        status_freq: usize,
        finish_crit: FinishCrit,
        mut exec_status: Option<ExecStatus<impl Fn(), impl FnMut(usize)>>,
    ) {
        assert!(status_freq > 0, "status_freq must be > 0");

        let start = Instant::now();

        if let Some(ExecStatus {
            ref pre_exec,
            exec_status: _,
        }) = exec_status
        {
            pre_exec();
        }

        let unit = get_bench_cfg().recording_unit();

        for i in 1.. {
            let latency = unit.latency_as_u64(latency(&mut f));
            self.capture_data(latency);

            if i % status_freq == 0 || finish_crit == FinishCrit::Count(i) {
                if let Some(ExecStatus {
                    pre_exec: _,
                    ref mut exec_status,
                }) = exec_status
                {
                    exec_status(i);
                }

                match finish_crit {
                    FinishCrit::Count(count) if count == i => break,
                    FinishCrit::Millis(millis) => {
                        let elapsed = Instant::now().duration_since(start).as_millis() as usize;
                        // Round up elapsed: elapsed + (elapsed/i * status_freq)/2 without loss of precision.
                        if (elapsed + elapsed * status_freq / (i * 2)) >= millis as usize {
                            break;
                        }
                    }
                    _ => continue,
                }
            }
        }
    }

    /// Warms-up the benchmark by invoking [`Self::execute`] with `finish_crit = FinishCrit::Millis(&BENCH_CFG.warmup_millis)`.
    /// The arguments are passed to [`Self::execute`].
    fn warmup(
        &mut self,
        f: impl FnMut(),
        status_freq: usize,
        exec_status: Option<ExecStatus<impl Fn(), impl FnMut(usize)>>,
    ) {
        let warmup_millis = get_bench_cfg().warmup_millis();
        self.execute(
            f,
            status_freq,
            FinishCrit::Millis(warmup_millis),
            exec_status,
        );
    }
}

pub struct BenchStatus<F1, F2, F3, F4> {
    pre_warmup: F1,
    warmup_status: F2,
    pre_exec: F3,
    exec_status: F4,
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
    bench_status: Option<BenchStatus<impl Fn(), impl FnMut(usize), impl Fn(), impl FnMut(usize)>>,
) -> BenchOut {
    let mut state = BenchOut::default();
    let cfg = get_bench_cfg();
    let status_freq = cfg.status_freq(&mut f);
    let (warmup_status, exec_status) = match bench_status {
        Some(s) => (
            Some(ExecStatus {
                pre_exec: s.pre_warmup,
                exec_status: s.warmup_status,
            }),
            Some(ExecStatus {
                pre_exec: s.pre_exec,
                exec_status: s.exec_status,
            }),
        ),
        None => (None, None),
    };

    state.warmup(&mut f, status_freq, warmup_status);
    state.reset();
    state.execute(
        &mut f,
        status_freq,
        FinishCrit::Count(exec_count),
        exec_status,
    );

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
    bench_run_x(
        f,
        exec_count,
        None::<BenchStatus<fn(), fn(usize), fn(), fn(usize)>>,
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
    f: impl FnMut(),
    exec_count: usize,
    header: impl FnOnce(usize),
) -> BenchOut {
    header(exec_count);

    let warmup_status = {
        let mut status_len: usize = 0;
        let warmup_millis = get_bench_cfg().warmup_millis();

        move |i: usize| {
            if status_len == 0 {
                eprint!("Warming up for {warmup_millis} millis ... ");
                stderr().flush().expect("unexpected I/O error");
            }
            eprint!("{}", "\u{8}".repeat(status_len));
            let status = format!("{i}.");
            status_len = status.len();
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

        move |i: usize| {
            eprint!("{}", "\u{8}".repeat(status_len));
            let status = format!("{i} of {exec_count}.");
            status_len = status.len();
            eprint!("{status}");
            stderr().flush().expect("unexpected I/O error");
        }
    };

    let bench_status = BenchStatus {
        pre_warmup: || (),
        warmup_status,
        pre_exec,
        exec_status,
    };

    let out = bench_run_x(f, exec_count, Some(bench_status));
    out
}
