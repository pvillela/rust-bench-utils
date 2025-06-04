//! Implements functions to collect latency statistics for a closure.

use super::BenchOut;
use crate::latency;
use std::{
    io::{Write, stderr},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

static WARMUP_MILLIS: AtomicU64 = AtomicU64::new(3_000);

/// The currently defined number of milliseconds used to "warm-up" the benchmark. The default is 3,000 ms.
///
/// Use [`set_warmup_millis`] to change the value.
pub fn get_warmup_millis() -> u64 {
    WARMUP_MILLIS.load(Ordering::Relaxed)
}

/// Changes the number of milliseconds used to "warm-up" the benchmark. The default is 3,000 ms.
pub fn set_warmup_millis(millis: u64) {
    WARMUP_MILLIS.store(millis, Ordering::Relaxed);
}

const WARMUP_INCREMENT_COUNT: usize = 20;

/// Unit of time used to record latencies. Used as an argument in benchmarking functions.
#[derive(Clone, Copy, Debug)]
pub enum LatencyUnit {
    Milli,
    Micro,
    Nano,
}

impl LatencyUnit {
    /// Converts a `latency` [`Duration`] to a `u64` value according to the unit `self`.
    #[inline(always)]
    pub fn latency_as_u64(&self, latency: Duration) -> u64 {
        match self {
            Self::Nano => latency.as_nanos() as u64,
            Self::Micro => latency.as_micros() as u64,
            Self::Milli => latency.as_millis() as u64,
        }
    }

    /// Converts a `u64` value to a [`Duration`] according to the unit `self`.
    #[inline(always)]
    pub fn latency_from_u64(&self, elapsed: u64) -> Duration {
        match self {
            Self::Nano => Duration::from_nanos(elapsed),
            Self::Micro => Duration::from_micros(elapsed),
            Self::Milli => Duration::from_millis(elapsed),
        }
    }

    /// Converts a `latency` [`Duration`] to an `f64` value according to the unit `self`.
    #[inline(always)]
    pub fn latency_as_f64(&self, latency: Duration) -> f64 {
        self.latency_as_u64(latency) as f64
    }

    /// Converts an `f64` value to a [`Duration`] according to the unit `self`.
    #[inline(always)]
    pub fn latency_from_f64(&self, elapsed: f64) -> Duration {
        self.latency_from_u64(elapsed as u64)
    }
}

type BenchState = BenchOut;

impl BenchState {
    /// Executes `f` repeatedly and captures latencies.
    /// `pre_exec` is invoked once just before the invocation of `f`, and `exec_status` is invoked at the
    /// end of each invocation.
    fn execute(
        &mut self,
        unit: LatencyUnit,
        mut f: impl FnMut(),
        exec_count: usize,
        pre_exec: impl FnOnce(),
        mut exec_status: impl FnMut(usize),
        init_status_count: usize,
    ) {
        pre_exec();

        for i in 1..=exec_count {
            let elapsed = unit.latency_as_u64(latency(&mut f));
            self.capture_data(elapsed);
            exec_status(init_status_count + i);
        }
    }

    /// Warms-up the benchmark by invoking [`Self::execute`] repeatedly, each time with an `exec_count` value of
    /// [`WARMUP_INCREMENT_COUNT`], until the globally set number of warm-up millisecods [`WARMUP_MILLIS`] is
    /// reached or exceeded. `warmup_status` is invoked at the end of each invocation of [`Self::execute`].
    fn warmup(
        &mut self,
        unit: LatencyUnit,
        mut f: impl FnMut(),
        mut warmup_status: impl FnMut(usize, u64, u64),
    ) {
        let warmup_millis = get_warmup_millis();
        let start = Instant::now();
        for i in 1.. {
            self.execute(unit, &mut f, WARMUP_INCREMENT_COUNT, || {}, |_| {}, 0);
            let elapsed = Instant::now().duration_since(start);
            warmup_status(i, elapsed.as_millis() as u64, warmup_millis);
            if elapsed.ge(&Duration::from_millis(warmup_millis)) {
                break;
            }
        }
    }

    fn reset(&mut self) {
        self.hist.reset();
        self.sum = 0;
        self.sum_ln = 0.;
        self.sum2_ln = 0.;
    }
}

/// Repeatedly executes closure `f`, collects the resulting latency data in a [`BenchOut`] object, and
/// *optionally* outputs information about the benchmark and its execution status.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`get_warmup_millis`] milliseconds.
///
/// Arguments:
/// - `unit` - the unit used for data collection.
/// - `f` - benchmark target.
/// - `exec_count` - number of executions (sample size) for the function.
/// - `warmup_status` - is invoked every so often during warm-up and can be used to output the warm-up status,
///   e.g., how much warm-up time has elapsed and the target warm-up time. The first argument is the warm-up
///   execution iteration, the second is the elapsed warm-up time, and the third is the target warm-up time.
///   (See the source code of [`bench_one_with_status`] for an example.)
/// - `pre_exec` - is invoked once at the beginning of data collection, after warm-up. It can be used,
///   for example, to output a preamble to the execution status (see `exec_status` below).
/// - `exec_status` - is invoked after each execution of `f` and can be used to output the execution
///   status, e.g., how many observations have been collected versus `exec_count`.
///   Its argument is the current number of executions performed.
///   (See the source code of [`bench_one_with_status`] for an example.)
pub fn bench_one_x(
    unit: LatencyUnit,
    mut f: impl FnMut(),
    exec_count: usize,
    mut warmup_status: impl FnMut(usize, u64, u64),
    pre_exec: impl FnOnce(),
    mut exec_status: impl FnMut(usize),
) -> BenchOut {
    let mut state = BenchOut::new();

    state.warmup(unit, &mut f, &mut warmup_status);
    state.reset();
    state.execute(unit, &mut f, exec_count, pre_exec, &mut exec_status, 0);

    state
}

/// Repeatedly executes closure `f` and collects the resulting latency data in a [`BenchOut`] object.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`get_warmup_millis`] milliseconds.
/// This function calls [`bench_one_x`] with no-op closures for the arguments that support the output of
/// benchmark status.
///
/// Arguments:
/// - `unit` - the unit used for data collection.
/// - `f` - benchmark target.
/// - `exec_count` - number of executions (sample size) for the function.
pub fn bench_one(unit: LatencyUnit, f: impl FnMut(), exec_count: usize) -> BenchOut {
    bench_one_x(unit, f, exec_count, |_, _, _| {}, || (), |_| ())
}

/// Repeatedly executes closure `f`, collects the resulting latency data in a [`BenchOut`] object, and
/// outputs information about the benchmark and its execution status.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`get_warmup_millis`] milliseconds.
/// This function calls [`bench_one_x`] with pre-defined closures for the arguments that support the output of
/// benchmark status to `stderr`.
///
/// Arguments:
/// - `unit` - the unit used for data collection.
/// - `f` - benchmark target.
/// - `exec_count` - number of executions (sample size) for the function.
/// - `header` - is invoked once at the start of this function's execution; it can be used, for example,
///   to output information about the function being benchmarked to `stdout` and/or `stderr`. The first
///   argument is the the `LatencyUnit` and the second argument is the `exec_count`.
pub fn bench_one_with_status(
    unit: LatencyUnit,
    f: impl FnMut(),
    exec_count: usize,
    header: impl FnOnce(LatencyUnit, usize),
) -> BenchOut {
    header(unit, exec_count);

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
        eprint!(" Executing bench_one ... ");
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

    bench_one_x(unit, f, exec_count, warmup_status, pre_exec, exec_status)
}
