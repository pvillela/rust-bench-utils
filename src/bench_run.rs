//! Implements functions to collect latency statistics for a closure.

use crate::{
    BenchCfg, BenchOut, RunLength,
    multi::{self, LatencySrc1},
    status::Status,
};

/// Repeatedly executes closure `f`, collects the resulting latency data in a [`BenchOut`] object, and
/// *optionally* reports progress status during benchmark execution.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// `cfg.warmup_millis` milliseconds.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f` - benchmark target closure.
/// - `exec_run_length` - target run length (iteration count and/or duration) for data collection.
/// - `s` - status handler for reporting warm-up and execution progress.
pub fn bench_run_x<'a, S: Status<'a>>(
    cfg: &BenchCfg,
    f: impl FnMut(),
    exec_run_length: RunLength,
    s: &mut S,
) -> BenchOut {
    multi::bench_run_x(cfg, LatencySrc1(f), exec_run_length, s).into()
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
pub fn bench_run(f: impl FnMut(), exec_run_length: RunLength) -> BenchOut {
    multi::bench_run(LatencySrc1(f), exec_run_length).into()
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
pub fn bench_run_arg_cfg(cfg: &BenchCfg, f: impl FnMut(), exec_run_length: RunLength) -> BenchOut {
    multi::bench_run_arg_cfg(cfg, LatencySrc1(f), exec_run_length).into()
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
pub fn bench_run_with_status(f: impl FnMut(), exec_run_length: RunLength) -> BenchOut {
    multi::bench_run_with_status(LatencySrc1(f), exec_run_length).into()
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
pub fn bench_run_with_status_arg_cfg(
    cfg: &BenchCfg,
    f: impl FnMut(),
    exec_run_length: RunLength,
) -> BenchOut {
    multi::bench_run_with_status_arg_cfg(cfg, LatencySrc1(f), exec_run_length).into()
}

#[cfg(test)]
#[cfg(feature = "_bench_long_test")]
// cargo test -r --package bench_utils --lib --all-features -- bench_run::long --nocapture --test-threads=1 --skip multi
mod long {
    // cargo test -r --package bench_utils --lib --all-features -- bench_run::long::validate --nocapture --test-threads=1 --skip multi
    mod validate {
        use crate::{
            BenchCfg, BenchOut, BusyWork, RunLength, latency, rel_approx_eq_dur,
            test_support::AbsRelDiffDur,
        };
        use std::time::{Duration, Instant};

        fn run<R, Fa>(
            runner: R,
            mut f: Fa,
            warmup_millis: u64,
            status_millis: u64,
            bench_time: Duration,
            target_latency: Duration,
            epsilon: f64,
        ) where
            Fa: FnMut() + Clone,
            R: Fn(&BenchCfg, Fa, RunLength) -> BenchOut,
        {
            let start = Instant::now();

            let name = format!(
                "target_latency={target_latency:?}, warmup={warmup_millis}, bench_time={bench_time:?}"
            );
            let exec_count = (bench_time.as_secs_f64() / target_latency.as_secs_f64()) as u64;

            println!("validate_bench_run: {name}");

            let cfg = BenchCfg::default()
                .with_warmup_millis(warmup_millis)
                .with_status_millis(status_millis);
            let out = runner(&cfg, f.clone(), RunLength::Count(exec_count));

            let raw_latency = latency(|| {
                for _ in 0..exec_count {
                    f();
                }
            });

            println!();

            let out_mean = out.mean();
            println!(
                "target_mean={target_latency:?}, out.mean()={out_mean:?}, rel_diff={}",
                target_latency.abs_rel_diff(out_mean)
            );
            let raw_mean = raw_latency / exec_count as u32;
            println!(
                "target_mean={target_latency:?}, raw_mean()={raw_mean:?}, rel_diff={}",
                target_latency.abs_rel_diff(raw_mean)
            );

            println!(
                "raw_mean={raw_mean:?}, out_mean()={out_mean:?}, rel_diff={}",
                raw_mean.abs_rel_diff(out_mean)
            );

            println!("test total elapsed time = {:?}", start.elapsed());

            rel_approx_eq_dur!(raw_mean, out_mean, epsilon);
        }

        // cargo test -r --package bench_utils --lib --all-features -- bench_run::validate::with_status --nocapture --test-threads=1 --skip multi
        mod with_status {
            use super::*;
            use crate::bench_run::bench_run_with_status_arg_cfg;

            fn run_bench(
                warmup_millis: u64,
                status_millis: u64,
                bench_time: Duration,
                target_latency: Duration,
                epsilon: f64,
            ) {
                let f = BusyWork::new(target_latency).fun();
                let runner = bench_run_with_status_arg_cfg;
                run(
                    runner,
                    f,
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

        // cargo test -r --package bench_utils --lib --all-features -- bench_run::long::validate::without_status --nocapture --test-threads=1 --skip multi
        mod without_status {
            use super::*;
            use crate::bench_run::bench_run_arg_cfg;

            fn run_bench(
                warmup_millis: u64,
                bench_time: Duration,
                target_latency: Duration,
                epsilon: f64,
            ) {
                let f = BusyWork::new(target_latency).fun();
                let runner = bench_run_arg_cfg;
                run(
                    runner,
                    f,
                    warmup_millis,
                    u64::MAX, // this value makes no difference as it is overridden in `bench_run_arg_cfg`
                    // 0,
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
                    Duration::from_millis(200),
                    Duration::from_micros(50),
                    EPSILON,
                );
            }
        }
    }
}

#[cfg(test)]
#[cfg(feature = "_bench")]
// cargo test -r --package bench_utils --lib --all-features -- bench_run::status --nocapture--test-threads=1 --skip multi
mod status {
    use super::*;
    use crate::{
        BusyWork, LatencyUnit, RunLength, status::DefaultStatus, test_support::StringWriter,
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
        println!(
            "\n***** Testing: warmup_millis={warmup_millis}, exec_run_length={exec_run_length:?}, status_millis={status_millis}, target_latency={target_latency:?}, epsilon={epsilon}"
        );

        let warmup_run_length = RunLength::Duration(Duration::from_millis(warmup_millis));

        let cfg = BenchCfg::default()
            .with_warmup_millis(warmup_millis)
            .with_status_millis(status_millis)
            .with_recording_unit(LatencyUnit::Nano);

        let mut w = StringWriter::new();
        let mut status = DefaultStatus::new(
            &mut w,
            "Warming up".to_owned(),
            "\nExecuting bench_run".to_owned(),
        );

        let f = BusyWork::new(target_latency).fun();

        let execs_per_milli = cfg.fn_execs_per_milli(&f, exec_run_length);

        let out = bench_run_x(&cfg, f, exec_run_length, &mut status);

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
            let warmup_last = caps[2].parse::<u64>().unwrap();
            let warmup_est_count = caps[3].parse::<u64>().unwrap();
            rel_approx_eq!(
                warmup_est_count as f64,
                warmup_run_length.estimated_count(execs_per_milli) as f64,
                epsilon
            );
            rel_approx_eq!(warmup_last as f64, warmup_est_count as f64, epsilon);
        }

        {
            rel_approx_eq!(
                caps[4].parse::<u64>().unwrap() as f64,
                exec_run_length
                    .estimated_duration(execs_per_milli)
                    .as_millis() as f64,
                epsilon
            );
            let exec_last = caps[5].parse::<u64>().unwrap();
            let exec_est_count = caps[6].parse::<u64>().unwrap();
            rel_approx_eq!(
                exec_est_count as f64,
                exec_run_length.estimated_count(execs_per_milli) as f64,
                epsilon
            );
            rel_approx_eq!(exec_last as f64, exec_est_count as f64, epsilon);
            assert_eq!(out.n(), exec_last);
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
    }

    #[test]
    fn test_bench_run_with_count() {
        let cfg = quick_cfg();
        let out = bench_run_arg_cfg(&cfg, || (), RunLength::Count(5));
        // With 5 count and no timeout, we should have exactly 5 iterations
        assert_eq!(out.n(), 5);
    }

    #[test]
    fn test_bench_run_with_duration() {
        let cfg = quick_cfg();

        // Use a very short timeout that should be exceeded immediately
        let out = bench_run_arg_cfg(
            &cfg,
            || thread::sleep(Duration::from_nanos(1)),
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
            || thread::sleep(Duration::from_nanos(1)),
            RunLength::CountWithTimeout(20, Duration::from_nanos(1)),
        );
        // At least some executions should have been captured
        assert!(out.n() > 0 && out.n() < 20);
    }
}
