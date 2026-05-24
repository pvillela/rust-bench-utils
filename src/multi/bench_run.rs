//! Implements functions to collect latency statistics for a closure.

use crate::{BenchCfg, RunLength, latency, multi::BenchOut};
use std::{
    array,
    io::{Stderr, Write, stderr},
    time::{Duration, Instant},
};

type BenchState<const K: usize> = BenchOut<K>;

impl<const K: usize> BenchState<K> {
    /// Executes `f` repeatedly and captures latencies.
    /// `exec_status` is invoked once every `status_freq` invocations of `f`.
    fn execute(
        &mut self,
        fs: &mut [impl FnMut(); K],
        run_length: RunLength,
        status_freq: usize,
        // Used in control of the exit from the iteration loop when both `status_freq` and `exec_count` are too high
        // compared to `run_length` duration.
        est_count: usize,
        mut status_rept: Option<impl FnMut(usize)>,
    ) {
        assert!(status_freq > 0, "status_freq must be > 0");

        let (exec_count, run_time) = run_length.get_exec_count_and_duration();
        assert!(exec_count > 0, "exec_count must be > 0");

        let mut est_remaining_iters = est_count;
        let start = Instant::now();

        for i in 1..=exec_count {
            let latencies = array::from_fn(|k| latency(&mut fs[k]));

            self.capture_data(latencies);

            if est_remaining_iters > 0 {
                est_remaining_iters -= 1;
            }

            if i % status_freq == 0 || i == exec_count || est_remaining_iters == 0 {
                let elapsed = start.elapsed();
                let finished = i == exec_count || elapsed >= run_time;

                if i % status_freq == 0 || finished {
                    if let Some(ref mut exec_status) = status_rept {
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
/// Runs with the default [`BenchCfg`].
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
pub fn bench_run_x<const K: usize, F: FnMut(usize)>(
    fs: &mut [impl FnMut(); K],
    exec_run_length: RunLength,
    warmup_status_fn: impl Fn(usize, Duration, usize) -> Option<F>,
    exec_status_fn: impl Fn(usize, Duration, usize) -> Option<F>,
) -> BenchOut<K> {
    let cfg = BenchCfg::default();
    bench_run_x_arg_cfg(&cfg, fs, exec_run_length, warmup_status_fn, exec_status_fn)
}

/// Repeatedly executes closure `f`, collects the resulting latency data in a [`BenchOut`] object, and
/// *optionally* outputs information about the benchmark and its execution status.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// `warmup_millis` milliseconds.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f` - benchmark target.
/// - `warmup_millis` - duration (in milliseconds) of warm-up execution.
/// - `exec_run_length` - target run length (iteration count and/or duration) for data collection.
/// - `warmup_status` - optionally invoked periodically during warm-up. Its argument is the current
///   warm-up execution iteration.
/// - `exec_status` - optionally invoked periodically during data collection. Its argument is the
///   current number of executions performed.
/// - `execs_per_milli` - estimate of how many executions of `f` fit in one millisecond.
pub fn bench_run_x_args_cfg_writer<const K: usize, W: Write>(
    cfg: &BenchCfg,w: &mut W,
    fs: &mut [impl FnMut(); K],
    exec_run_length: RunLength,
    warmup_status: Option<impl Fn(&mut W, Duration, usize, usize)>,
    exec_status: Option<impl Fn(&mut W, Duration, usize, usize)>,
) -> BenchOut<K> {
    let mut state = BenchOut::new(cfg);
    let execs_per_milli = cfg.execs_per_milli(|| fs.iter_mut().for_each(|f| f()));
    let status_freq = cfg.status_freq(execs_per_milli);

    let warmup_run_length = RunLength::Duration(Duration::from_millis(cfg.warmup_millis()));
    let warmup_est_dur = warmup_run_length.estimated_duration(execs_per_milli);
    let warmup_est_count = warmup_run_length.estimated_count(execs_per_milli);
    let exec_est_dur = exec_run_length.estimated_duration(execs_per_milli);
    let exec_est_count = exec_run_length.estimated_count(execs_per_milli);

    let warmup_status = |w|warmup_status.map(|s|move |i|s(w,warmup_est_dur, warmup_est_count, i));
    let exec_status =|w| exec_status.map(|s|move |i|s(w,exec_est_dur, exec_est_count, i));

    // Warm-up.
    state.execute(
        fs,
        warmup_run_length,
        status_freq,
        warmup_est_count,
        warmup_status,
    );
    state.reset();

    state.execute(
        fs,
        exec_run_length,
        status_freq,
        exec_est_count,
        exec_status,
    );

    state
}

/// Repeatedly executes closure `f` and collects the resulting latency data in a [`BenchOut`] object.
/// Runs with the default [`BenchCfg`].
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
/// This function calls [`bench_run_x`] with no-op closures for the arguments that support the output of
/// benchmark status.
///
/// Arguments:
/// - `f` - benchmark target.
/// - `exec_run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run<const K: usize>(
    fs: &mut [impl FnMut(); K],
    exec_run_length: RunLength,
) -> BenchOut<K> {
    let cfg = BenchCfg::default();
    bench_run_arg_cfg(&cfg, fs, exec_run_length)
}

/// Repeatedly executes closure `f` and collects the resulting latency data in a [`BenchOut`] object.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
/// This function calls [`bench_run_x`] with no-op closures for the arguments that support the output of
/// benchmark status.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f` - benchmark target.
/// - `exec_run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run_arg_cfg<const K: usize>(
    cfg: &BenchCfg,
    fs: &mut [impl FnMut(); K],
    exec_run_length: RunLength,
) -> BenchOut<K> {
    bench_run_x_arg_cfg(
        cfg,
        fs,
        exec_run_length,
        None::<fn( _, _, _)>,
         None::<fn( _, _, _)>,
    )
}

/// Repeatedly executes closure `f`, collects the resulting latency data in a [`BenchOut`] object, and
/// outputs information about the benchmark and its execution status.
/// Runs with the default [`BenchCfg`].
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
/// This function calls [`bench_run_x`] with pre-defined closures for the arguments that support the output of
/// benchmark status to `stderr`.
///
/// Arguments:
/// - `f` - benchmark target.
/// - `exec_run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run_with_status<const K: usize>(
    fs: &mut [impl FnMut(); K],
    exec_run_length: RunLength,
) -> BenchOut<K> {
    let cfg = BenchCfg::default();
    bench_run_with_status_arg_cfg(&cfg, fs, exec_run_length)
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
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f` - benchmark target.
/// - `exec_run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run_with_status_arg_cfg<const K: usize>(
    cfg: &BenchCfg,
    fs: &mut [impl FnMut(); K],
    exec_run_length: RunLength,
) -> BenchOut<K> {
    let mut w = stderr();

    let warmup_status = make_status(&mut w,"Warming up".to_owned());
    // The `\n` below is to separate warmup status from exec status. Otherwise, they get mixed up due to
    // the `eprint!("{}", "\u{8}".repeat(status_len))` line in the `status` closure.
    let exec_status = make_status(&mut w,"\nExecuting bench_run".to_owned());

    bench_run_with_status_args_cfg_writer(cfg, &mut w, fs, exec_run_length)
}

/// Used to implement [`bench_run_with_status_arg_cfg`] and to support testing.
pub(crate) fn bench_run_with_status_args_cfg_writer<const K: usize, W: Write>(
    cfg: &BenchCfg,
    w: &mut W,
    fs: &mut [impl FnMut(); K],
    exec_run_length: RunLength,
) -> BenchOut<K> {
    let execs_per_milli = cfg.execs_per_milli(|| fs.iter_mut().for_each(|f| f()));

    let warmup_millis = cfg.warmup_millis();
    let warmup_run_length = RunLength::Duration(Duration::from_millis(warmup_millis));
    let warmup_est_count = warmup_run_length.estimated_count(execs_per_milli);

    let warmup_status = make_status("Warming up", warmup_millis, warmup_est_count);

    let exec_est_count = exec_run_length.estimated_count(execs_per_milli);
    let exec_est_millis = exec_run_length
        .estimated_duration(execs_per_milli)
        .as_millis() as u64;

    // The `\n` below is to separate warmup status from exec status. Otherwise, they get mixed up due to
    // the `eprint!("{}", "\u{8}".repeat(status_len))` line in the `status` closure.
    let exec_status = make_status("\nExecuting bench_run", exec_est_millis, exec_est_count);

    let out = bench_run_x_args_cfg_writer(
        cfg,
        w,
        fs,
        exec_run_length,
        Some(warmup_status),
        Some(exec_status),
    );

    out
}

#[doc(hidden)]
pub struct NullWrite;

impl Write for NullWrite {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub trait StatusReportMaker {
    fn make(&mut self)-> impl FnMut(usize)
}

pub struct DefaultStatusReportMaker<'a, W: Write> {
    pub preamble: String,
    pub warmup_est_count: usize,
    pub exec_est_dur: Duration,
    pub exec_est_count: usize,
    pub w: &'a mut W,
}

impl<'a, W: Write> StatusReportMaker for DefaultStatusReportMaker<'a, W> {
    fn make(&mut self) -> impl FnMut(usize) {
        let mut status_len: usize = 0;

        move |i: usize| {
            if status_len == 0 {
                write!(
                    self.w,
                    "{} for (approx.) {} millis: ",
                    self.preamble.clone(),
                    self.exec_est_dur.as_millis()
                )
                .expect("unexpected error writing to `Write` object `w`");
                self.w.flush().expect("unexpected I/O error");
            }
            write!(self.w, "{}", "\u{8}".repeat(status_len))
                .expect("unexpected error writing to `Write` object `w`");
            let status = format!("{} of (approx.) {} executions.", i, self.exec_est_count);
            status_len = status.len();
            write!(self.w, "{status}").expect("unexpected error writing to `Write` object `w`");
            self.w.flush().expect("unexpected I/O error");
        }
    }
}

 pub   fn make_status<'a, W: Write>(     
     w: &'a mut W,
    preamble: String,
) -> impl FnMut(  Duration, usize, usize) {
        let mut status_len: usize = 0;

        move |est_dur: Duration, est_count: usize, i: usize | {
            if status_len == 0 {
                write!(
                    w,
                    "{} for (approx.) {} millis: ",
                    preamble.clone(),
                    est_dur.as_millis()
                )
                .expect("unexpected error writing to `Write` object `w`");
                w.flush().expect("unexpected I/O error");
            }
            write!(w, "{}", "\u{8}".repeat(status_len))
                .expect("unexpected error writing to `Write` object `w`");
            let status = format!("{} of (approx.) {} executions.", i, est_count);
            status_len = status.len();
            write!(w, "{status}").expect("unexpected error writing to `Write` object `w`");
            w.flush().expect("unexpected I/O error");
        }
    }


#[cfg(test)]
#[cfg(feature = "_bench")]
#[cfg(feature = "busy_work")]
mod validate {
    use crate::{BenchCfg, BusyWork, RunLength, bench_run_with_status_arg_cfg};
    use basic_stats::{dev_utils::ApproxEq, rel_approx_eq};
    use std::time::Duration;

    const BENCH_TIME: Duration = Duration::from_millis(500);

    fn run_bench(warmup_millis: u64, target_latency: Duration, epsilon: f64) {
        let name = format!("sleep_{}_micros", target_latency.as_micros());

        let reporting_unit = BenchCfg::default().reporting_unit();
        let target_median = reporting_unit.latency_as_f64(target_latency);
        let exec_count = (reporting_unit.latency_as_f64(BENCH_TIME) / target_median) as usize;

        println!("validate_bench_run: {name}");

        let cfg = BenchCfg::default().with_warmup_millis(warmup_millis);
        let out = bench_run_with_status_arg_cfg(
            &cfg,
            BusyWork::new(target_latency).fun(),
            RunLength::Count(exec_count),
        );

        println!(
            "target_median={target_median}, out.median()={}, rel_diff={}",
            out.median(),
            target_median.abs_rel_diff(out.median())
        );
        println!("{:?}", out.summary());
        println!();

        rel_approx_eq!(target_median, out.median(), epsilon);
    }

    #[test]
    fn test_millis() {
        const EPSILON: f64 = 0.05;
        run_bench(1200, Duration::from_millis(60), EPSILON);
    }

    #[test]
    fn test_micros() {
        const EPSILON: f64 = 0.05;
        run_bench(100, Duration::from_micros(60), EPSILON);
    }
}

#[cfg(test)]
#[cfg(feature = "_bench")]
// cargo test --package bench_utils --lib --all-features -- multi::bench_run::status --nocapture
mod status {
    use super::*;
    use crate::{BusyWork, LatencyUnit, RunLength, test_support::StringWriter};
    use basic_stats::rel_approx_eq;
    use regex::Regex;
    use std::time::Duration;

    fn run_test(
        warmup_millis: u64,
        exec_run_length: RunLength,
        status_millis: u64,
        target_latency: Duration,
        epsilon: f64,
    ) {
        println!(
            "\n***** Testing: warmup_millis={warmup_millis}, exec_run_length={exec_run_length:?}, status_millis={status_millis}, target_latency={target_latency:?}, epsilon={epsilon}"
        );

        let warmup_run_length = RunLength::Duration(Duration::from_millis(warmup_millis));

        let cfg = BenchCfg::default()
            .with_warmup_millis(warmup_millis)
            .with_status_millis(status_millis)
            .with_recording_unit(LatencyUnit::Nano)
            .with_reporting_unit(LatencyUnit::Micro);

        let mut w = StringWriter::new();
        let fs = &mut array::from_fn::<_, 2, _>(|_| BusyWork::new(target_latency / 2).fun());

        let execs_per_milli = cfg.execs_per_milli(|| fs.iter_mut().for_each(|f| f()));

        let out = bench_run_with_status_args_cfg_writer(&cfg, &mut w, fs, exec_run_length);

        let status_str = w.as_str().expect("StringWriter doesn't contain string");
        println!("** {status_str}");

        let re = Regex::new(
            r"Warming up for \(approx.\) (\d+) millis: (\d+) of \(approx.\) (\d+) executions.
Executing bench_run for \(approx.\) (\d+) millis: (\d+) of \(approx.\) (\d+) executions.",
        )
        .expect("invalid regex");

        let caps = re
            .captures_iter(status_str)
            .next()
            .expect("no captures in regex");

        for i in 1..=6 {
            print!("&caps[{i}]={}, ", &caps[i]);
        }
        println!();

        {
            assert_eq!(caps[1], warmup_millis.to_string());
            let warmup_last = usize::from_str_radix(&caps[2], 10).unwrap();
            let warmup_est_count = usize::from_str_radix(&caps[3], 10).unwrap();
            rel_approx_eq!(
                warmup_est_count as f64,
                warmup_run_length.estimated_count(execs_per_milli) as f64,
                epsilon
            );
            rel_approx_eq!(warmup_last as f64, warmup_est_count as f64, epsilon);
        }

        {
            rel_approx_eq!(
                usize::from_str_radix(&caps[4], 10).unwrap() as f64,
                exec_run_length
                    .estimated_duration(execs_per_milli)
                    .as_millis() as f64,
                epsilon
            );
            let exec_last = usize::from_str_radix(&caps[5], 10).unwrap();
            let exec_est_count = usize::from_str_radix(&caps[6], 10).unwrap();
            rel_approx_eq!(
                exec_est_count as f64,
                exec_run_length.estimated_count(execs_per_milli) as f64,
                epsilon
            );
            rel_approx_eq!(exec_last as f64, exec_est_count as f64, epsilon);
            assert_eq!(out.n(), exec_last as u64);
        }
    }

    #[test]
    fn test_0_10_0_200() {
        const EPSILON: f64 = 2.0;

        let warmup_millis: u64 = 0;
        let target_latency = Duration::from_micros(10);
        let status_millis: u64 = 0;
        let exec_run_length = RunLength::Count(200);

        run_test(
            warmup_millis,
            exec_run_length,
            status_millis,
            target_latency,
            EPSILON,
        );
    }

    #[test]
    fn test_0_10_0_0() {
        const EPSILON: f64 = f64::NAN;

        let warmup_millis: u64 = 0;
        let target_latency = Duration::from_micros(10);
        let status_millis: u64 = 0;
        let exec_run_length = RunLength::Count(0);

        // exec_count must be > 0 but exec_run_length makes it 0
        let result = {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_test(
                    warmup_millis,
                    exec_run_length,
                    status_millis,
                    target_latency,
                    EPSILON,
                );
            }))
        };
        println!("RECOVERED");

        assert!(
            result.is_err(),
            "expected panic because exec_count must be > 0"
        );
    }

    #[test]
    fn test_0_10_1_200() {
        const EPSILON: f64 = 2.0;

        let warmup_millis: u64 = 0;
        let target_latency = Duration::from_micros(10);
        let status_millis: u64 = 1;
        let exec_run_length = RunLength::Count(200);

        run_test(
            warmup_millis,
            exec_run_length,
            status_millis,
            target_latency,
            EPSILON,
        );
    }

    #[test]
    fn test_1_10_0_200() {
        const EPSILON: f64 = 0.50;

        let warmup_millis: u64 = 1;
        let target_latency = Duration::from_micros(10);
        let status_millis: u64 = 0;
        let exec_run_length = RunLength::Count(200);

        run_test(
            warmup_millis,
            exec_run_length,
            status_millis,
            target_latency,
            EPSILON,
        );
    }

    #[test]
    fn test_1_10_1_200() {
        const EPSILON: f64 = 0.50;

        let warmup_millis: u64 = 1;
        let target_latency = Duration::from_micros(10);
        let status_millis: u64 = 1;
        let exec_run_length = RunLength::Count(200);

        run_test(
            warmup_millis,
            exec_run_length,
            status_millis,
            target_latency,
            EPSILON,
        );
    }

    #[test]
    fn test_1_10_1_300() {
        const EPSILON: f64 = 0.10;

        let warmup_millis: u64 = 1;
        let target_latency = Duration::from_micros(10);
        let status_millis: u64 = 1;
        let exec_run_length = RunLength::Count(300);

        run_test(
            warmup_millis,
            exec_run_length,
            status_millis,
            target_latency,
            EPSILON,
        );
    }

    #[test]
    fn test_1_10_1_2000() {
        const EPSILON: f64 = 0.10;

        let warmup_millis: u64 = 1;
        let target_latency = Duration::from_micros(10);
        let status_millis: u64 = 1;
        let exec_run_length = RunLength::CountWithTimeout(300, Duration::from_micros(2000));

        run_test(
            warmup_millis,
            exec_run_length,
            status_millis,
            target_latency,
            EPSILON,
        );
    }
}

#[cfg(test)]
#[cfg(feature = "_test")]
/// Tests created by Claude Code, improved a bit by me.
mod simple_tests {
    use super::*;
    use crate::{LatencyUnit, RunLength};
    use std::{thread, time::Duration};

    /// Helper to get a clean config with minimal warmup/calibration for fast tests.
    fn quick_cfg() -> BenchCfg {
        BenchCfg::default()
            .with_warmup_millis(0)
            .with_status_millis(1)
            .with_recording_unit(LatencyUnit::Nano)
            .with_reporting_unit(LatencyUnit::Nano)
    }

    #[test]
    fn test_bench_run_with_count() {
        let cfg = quick_cfg();
        let out = bench_run_arg_cfg(
            &cfg,
            &mut [|| thread::sleep(Duration::from_nanos(1))],
            RunLength::Count(5),
        );
        // With 5 count and no timeout, we should have exactly 5 iterations
        assert_eq!(out.n(), 5);
    }

    #[test]
    fn test_bench_run_x() {
        let cfg = quick_cfg();
        let out = bench_run_x_arg_cfg(
            &cfg,
            &mut [|| {}],
            RunLength::Count(10),
            None::<fn(usize)>,
            None::<fn(usize)>,
        );

        assert_eq!(out.n(), 10);
    }

    #[test]
    fn test_bench_run_with_duration() {
        let cfg = quick_cfg();

        // Use a very short timeout that should be exceeded immediately
        let out = bench_run_arg_cfg(
            &cfg,
            &mut [|| thread::sleep(Duration::from_nanos(1))],
            RunLength::Duration(Duration::from_nanos(1)),
        );
        // At least some executions should have been captured
        assert!(out.n() > 0);
    }

    #[test]
    fn test_bench_run_with_timeout() {
        let cfg = quick_cfg();

        // Use a very short timeout that should be exceeded immediately
        let out = bench_run_arg_cfg(
            &cfg,
            &mut [|| thread::sleep(Duration::from_nanos(1))],
            RunLength::CountWithTimeout(20, Duration::from_nanos(1)),
        );
        // At least some executions should have been captured
        assert!(out.n() > 0 && out.n() < 20);
    }
}
