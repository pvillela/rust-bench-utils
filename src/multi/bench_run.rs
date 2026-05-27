//! Implements functions to collect latency statistics for a closure.

use crate::{
    BenchCfg, RunLength,
    multi::BenchOut,
    status::{DefaultStatus, NoStatus, Status},
};
use std::{
    io::stderr,
    time::{Duration, Instant},
};

type BenchState<const K: usize> = BenchOut<K>;

impl<const K: usize> BenchState<K> {
    /// Executes target closures repeatedly and captures latencies.
    /// `exec_status` is invoked once for every `status_freq` invocations of the closures.
    fn execute(
        &mut self,
        latency_src: &mut impl Iterator<Item = [Duration; K]>,
        run_length: RunLength,
        status_freq: usize,
        // Used in control of the exit from the iteration loop when both `status_freq` and `exec_count` are too high
        // compared to `run_length` duration.
        est_count: usize,
        status: &mut Option<impl FnMut(usize)>,
    ) {
        assert!(status_freq > 0, "status_freq must be > 0");

        let (exec_count, run_time) = run_length.get_exec_count_and_duration();
        assert!(exec_count > 0, "exec_count must be > 0");

        let mut est_remaining_iters = est_count;
        let start = Instant::now();

        for i in 1..=exec_count {
            let iter_finished = if let Some(latencies) = latency_src.next() {
                self.capture_data(latencies);
                false
            } else {
                true
            };

            if est_remaining_iters > 0 {
                est_remaining_iters -= 1;
            }

            if i % status_freq == 0 || i == exec_count || est_remaining_iters == 0 || iter_finished
            {
                let elapsed = start.elapsed();
                let finished = i == exec_count || elapsed >= run_time || iter_finished;

                if i % status_freq == 0 || finished {
                    if let Some(exec_status) = status {
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

/// Repeatedly executes closures `fs`, collects the resulting latency data in a [`BenchOut`] object, and
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
pub fn bench_run_x<'a, const K: usize, S: Status<'a>>(
    cfg: &BenchCfg,
    mut latency_src: impl Iterator<Item = [Duration; K]>,
    exec_run_length: RunLength,
    s: &mut S,
) -> BenchOut<K> {
    let mut state = BenchOut::new(cfg);
    let execs_per_milli = cfg.latency_src_execs_per_milli(&mut latency_src);
    let status_freq = cfg.status_freq(execs_per_milli);

    let warmup_run_length = RunLength::Duration(Duration::from_millis(cfg.warmup_millis()));
    let warmup_est_dur = warmup_run_length.estimated_duration(execs_per_milli);
    let warmup_est_count = warmup_run_length.estimated_count(execs_per_milli);
    let exec_est_dur = exec_run_length.estimated_duration(execs_per_milli);
    let exec_est_count = exec_run_length.estimated_count(execs_per_milli);

    // Warm-up.
    let mut warmup_status = S::part_apply(s.warmup_status(), warmup_est_dur, warmup_est_count);
    state.execute(
        &mut latency_src,
        warmup_run_length,
        status_freq,
        warmup_est_count,
        &mut warmup_status,
    );
    state.reset();
    drop(warmup_status);

    // Execute.
    let mut exec_status = S::part_apply(s.exec_status(), exec_est_dur, exec_est_count);
    state.execute(
        &mut latency_src,
        exec_run_length,
        status_freq,
        exec_est_count,
        &mut exec_status,
    );

    state
}

/// Repeatedly executes closures `fs` and collects the resulting latency data in a [`BenchOut`] object.
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
    latency_src: impl Iterator<Item = [Duration; K]>,
    exec_run_length: RunLength,
) -> BenchOut<K> {
    let cfg = BenchCfg::default();
    bench_run_arg_cfg(&cfg, latency_src, exec_run_length)
}

/// Repeatedly executes closures `fs` and collects the resulting latency data in a [`BenchOut`] object.
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
    latency_src: impl Iterator<Item = [Duration; K]>,
    exec_run_length: RunLength,
) -> BenchOut<K> {
    bench_run_x(cfg, latency_src, exec_run_length, &mut NoStatus)
}

/// Repeatedly executes closures `fs`, collects the resulting latency data in a [`BenchOut`] object, and
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
    latency_src: impl Iterator<Item = [Duration; K]>,
    exec_run_length: RunLength,
) -> BenchOut<K> {
    let cfg = BenchCfg::default();
    bench_run_with_status_arg_cfg(&cfg, latency_src, exec_run_length)
}

/// Repeatedly executes closures `fs`, collects the resulting latency data in a [`BenchOut`] object, and
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
    latency_src: impl Iterator<Item = [Duration; K]>,
    exec_run_length: RunLength,
) -> BenchOut<K> {
    let mut w = stderr();

    // The `\n` below is to separate warmup status from exec status. Otherwise, they get mixed up due to
    // the `eprint!("{}", "\u{8}".repeat(status_len))` line in the `status` closure.
    let mut s = DefaultStatus::new(
        &mut w,
        "Warming up".to_owned(),
        "\nExecuting bench_run".to_owned(),
    );

    bench_run_x(cfg, latency_src, exec_run_length, &mut s)
}

#[cfg(test)]
#[cfg(feature = "_bench_long_test")]
// cargo test -r --package bench_utils --lib --all-features -- multi::bench_run::validate --nocapture --test-threads=1
mod validate {
    use crate::{
        BenchCfg, BusyWork, RunLength, latency,
        multi::{BenchOut, LatencySrc2},
        rel_approx_eq_dur,
        test_support::AbsRelDiffDur,
    };
    use std::time::Duration;

    fn run<R, Src, F0, F1>(
        runner: R,
        latency_src: Src,
        f0: F0,
        f1: F1,
        warmup_millis: u64,
        status_millis: u64,
        bench_time: Duration,
        target_latency: Duration,
        epsilon: f64,
    ) where
        F0: FnMut(),
        F1: FnMut(),
        Src: Iterator<Item = [Duration; 2]>,
        R: Fn(&BenchCfg, Src, RunLength) -> BenchOut<2>,
    {
        let warmup_millis2 = warmup_millis * 2;
        let bench_time2 = bench_time * 2;
        let status_millis2 = status_millis * 2;
        let exec_count = (bench_time2.as_secs_f64() / target_latency.as_secs_f64()) as usize;

        let name = format!(
            "target_latency={target_latency:?}, warmup={warmup_millis2}, bench_time={bench_time2:?}"
        );
        println!("validate_bench_run: {name}");

        let mut fs = (f0, f1);

        let cfg = BenchCfg::default()
            .with_warmup_millis(warmup_millis2)
            .with_status_millis(status_millis2);
        let out = runner(&cfg, latency_src, RunLength::Count(exec_count));

        let raw_latencies = [
            latency(|| {
                for _ in 0..exec_count {
                    fs.0();
                }
            }),
            latency(|| {
                for _ in 0..exec_count {
                    fs.1();
                }
            }),
        ];
        let raw_means = raw_latencies
            .iter()
            .map(|lat| *lat / exec_count as u32)
            .collect::<Vec<_>>();

        let aggregate_raw_mean = raw_latencies.iter().sum::<Duration>() / exec_count as u32;
        let aggregate_out_mean = out[0].mean() + out[1].mean();
        println!();
        println!(
            "aggregate_raw_mean={aggregate_raw_mean:?}, aggregate_out_mean={aggregate_out_mean:?}, rel_diff={}",
            aggregate_raw_mean.abs_rel_diff(aggregate_out_mean)
        );

        rel_approx_eq_dur!(
            aggregate_raw_mean,
            aggregate_out_mean,
            epsilon / 2.0f64.sqrt()
        );

        for i in 0..2 {
            let out_mean = out[i].mean();
            let raw_mean = raw_means[i];

            println!();
            println!(
                "target_mean={target_latency:?}, out[{i}].mean()={out_mean:?}, rel_diff={}",
                target_latency.abs_rel_diff(out_mean)
            );

            println!(
                "target_mean={target_latency:?}, raw_mean()={raw_mean:?}, rel_diff={}",
                target_latency.abs_rel_diff(raw_mean)
            );

            println!(
                "raw_mean={raw_mean:?}, out[{i}].mean()={out_mean:?}, rel_diff={}",
                raw_mean.abs_rel_diff(out_mean)
            );

            rel_approx_eq_dur!(raw_mean, out_mean, epsilon);
        }
    }

    fn fs(target_latency: Duration) -> (impl Fn() + Clone, impl Fn() + Clone) {
        let latency_delta = target_latency / 10;

        (
            move || {
                BusyWork::new(target_latency - latency_delta).fun()();
                BusyWork::new(latency_delta).fun()();
            },
            BusyWork::new(target_latency).fun(),
        )
    }

    // cargo test -r --package bench_utils --lib --all-features -- multi::bench_run::validate::with_status --nocapture --test-threads=1
    mod with_status {
        use super::*;
        use crate::multi::bench_run::bench_run_with_status_arg_cfg;

        fn run_bench(
            warmup_millis: u64,
            status_millis: u64,
            bench_time: Duration,
            target_latency: Duration,
            epsilon: f64,
        ) {
            let (f0, f1) = fs(target_latency);
            let latency_src = LatencySrc2(f0.clone(), f1.clone());
            let runner = bench_run_with_status_arg_cfg;

            run(
                runner,
                latency_src,
                f0,
                f1,
                warmup_millis,
                status_millis,
                bench_time,
                target_latency,
                epsilon,
            );
        }

        #[test]
        fn test_millis() {
            const EPSILON: f64 = 0.02;
            run_bench(
                1000,
                100,
                Duration::from_millis(2000),
                Duration::from_millis(10),
                EPSILON,
            );
        }

        #[test]
        fn test_micros() {
            const EPSILON: f64 = 0.02;
            run_bench(
                100,
                10,
                Duration::from_millis(200),
                Duration::from_micros(50),
                EPSILON,
            );
        }
    }

    // cargo test -r --package bench_utils --lib --all-features -- multi::bench_run::validate::without_status --nocapture --test-threads=1
    mod without_status {
        use super::*;
        use crate::multi::bench_run::bench_run_arg_cfg;

        fn run_bench_without_status(
            warmup_millis: u64,
            bench_time: Duration,
            target_latency: Duration,
            epsilon: f64,
        ) {
            let (f0, f1) = fs(target_latency);
            let latency_src = LatencySrc2(f0.clone(), f1.clone());
            let runner = bench_run_arg_cfg;

            run(
                runner,
                latency_src,
                f0,
                f1,
                warmup_millis,
                0,
                bench_time,
                target_latency,
                epsilon,
            );
        }

        #[test]
        fn test_millis() {
            const EPSILON: f64 = 0.02;
            run_bench_without_status(
                1000,
                Duration::from_millis(2000),
                Duration::from_millis(10),
                EPSILON,
            );
        }

        #[test]
        fn test_micros() {
            const EPSILON: f64 = 0.02;
            run_bench_without_status(
                100,
                Duration::from_millis(200),
                Duration::from_micros(50),
                EPSILON,
            );
        }
    }
}

#[cfg(test)]
#[cfg(feature = "_bench")]
// cargo test -r --package bench_utils --lib --all-features -- multi::bench_run::status --nocapture
mod status {
    use super::*;
    use crate::{
        BusyWork, LatencyUnit, RunLength, multi::LatencySrc2, status::DefaultStatus,
        test_support::StringWriter,
    };
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
        // Scale certain arguments to align with status tests in non-multi `bench_run`,
        // because we have two functions here.
        let warmup_millis2 = warmup_millis * 2;
        let exec_run_length2 = match exec_run_length {
            RunLength::Count(_) => exec_run_length,
            RunLength::Duration(duration) => RunLength::Duration(duration * 2),
            RunLength::CountWithTimeout(count, duration) => {
                RunLength::CountWithTimeout(count, duration * 2)
            }
        };
        let status_millis2 = status_millis * 2;

        println!(
            "\n***** Testing: warmup_millis={warmup_millis2}, exec_run_length={exec_run_length2:?}, status_millis={status_millis2}, target_latency={target_latency:?}, epsilon={epsilon}"
        );

        let warmup_run_length = RunLength::Duration(Duration::from_millis(warmup_millis2));

        let cfg = BenchCfg::default()
            .with_warmup_millis(warmup_millis2)
            .with_status_millis(status_millis2)
            .with_recording_unit(LatencyUnit::Nano);

        let mut w = StringWriter::new();
        let mut status = DefaultStatus::new(
            &mut w,
            "Warming up".to_owned(),
            "\nExecuting bench_run".to_owned(),
        );

        let latency_delta = target_latency / 10;
        let mut latency_src = LatencySrc2(
            || {
                BusyWork::new(target_latency - latency_delta).fun()();
                BusyWork::new(latency_delta).fun()();
            },
            BusyWork::new(target_latency).fun(),
        );

        let execs_per_milli = cfg.latency_src_execs_per_milli(&mut latency_src);

        let out = bench_run_x(&cfg, &mut latency_src, exec_run_length2, &mut status);

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
            assert_eq!(caps[1], warmup_millis2.to_string());
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
                exec_run_length2
                    .estimated_duration(execs_per_milli)
                    .as_millis() as f64,
                epsilon
            );
            let exec_last = usize::from_str_radix(&caps[5], 10).unwrap();
            let exec_est_count = usize::from_str_radix(&caps[6], 10).unwrap();
            rel_approx_eq!(
                exec_est_count as f64,
                exec_run_length2.estimated_count(execs_per_milli) as f64,
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
    use crate::{LatencyUnit, RunLength, multi::LatencySrc1};
    use std::{thread, time::Duration};

    /// Helper to get a clean config with minimal warmup/calibration for fast tests.
    fn quick_cfg() -> BenchCfg {
        BenchCfg::default()
            .with_warmup_millis(0)
            .with_status_millis(1)
            .with_recording_unit(LatencyUnit::Nano)
    }

    #[test]
    fn test_bench_run_with_count() {
        let cfg = quick_cfg();
        let out = bench_run_arg_cfg(&cfg, &mut LatencySrc1(|| ()), RunLength::Count(5));
        // With 5 count and no timeout, we should have exactly 5 iterations
        assert_eq!(out.n(), 5);
    }

    #[test]
    fn test_bench_run_with_duration() {
        let cfg = quick_cfg();

        // Use a very short timeout that should be exceeded immediately
        let out = bench_run_arg_cfg(
            &cfg,
            &mut LatencySrc1(|| thread::sleep(Duration::from_nanos(1))),
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
            &mut LatencySrc1(|| thread::sleep(Duration::from_nanos(1))),
            RunLength::CountWithTimeout(20, Duration::from_nanos(1)),
        );
        // At least some executions should have been captured
        assert!(out.n() > 0 && out.n() < 20);
    }
}
