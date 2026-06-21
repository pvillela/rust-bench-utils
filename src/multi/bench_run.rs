//! Implements functions to collect latency statistics for a closure.

use crate::{
    BenchCfg, FpSeconds, RunLength,
    multi::{BenchOut, LatencySrc},
    status::{DefaultStatus, NoStatus, Status},
};
use log::{debug, trace};
use std::{
    io::stderr,
    time::{Duration, Instant},
};

type BenchState<const K: usize> = BenchOut<K>;

impl<const K: usize> BenchState<K> {
    /// Executes target closures repeatedly and captures latencies.
    /// `exec_status` is invoked once for every `status_count` invocations of the closures.
    fn execute(
        &mut self,
        src: &mut impl LatencySrc<K>,
        run_length: RunLength,
        status_count: usize,
        mut status: Option<impl FnMut(usize)>,
    ) {
        assert!(status_count > 0, "status_count must be > 0");

        let (exec_count, run_time) = run_length.exec_count_and_duration();
        debug!("execute >>> exec_count={exec_count}, run_time={run_time:?}");
        assert!(exec_count > 0, "exec_count must be > 0");

        let mut acc_latency = FpSeconds::ZERO; // enables testing with synthetic latency sources
        let start = Instant::now();

        for i in 1..=exec_count {
            let src_finished = if let Some(batch_latencies) = src.next() {
                acc_latency +=
                    batch_latencies.0.iter().cloned().sum::<FpSeconds>() * batch_latencies.1;
                trace!(
                    "execute >>> i={i}, batch_latencies={batch_latencies:?}, acc_latency={acc_latency:?}"
                );
                self.capture_data(batch_latencies);
                false
            } else {
                true
            };

            let elapsed = start.elapsed();
            trace!("execute >>> i={i}, elapsed={elapsed:?}");

            if i == exec_count
                || elapsed >= run_time
                || i.is_multiple_of(status_count)
                || acc_latency.as_duration() >= run_time
                || src_finished
            {
                let finished = i == exec_count
                    || elapsed >= run_time
                    || acc_latency.as_duration() >= run_time
                    || src_finished;

                if (i % status_count == 0 || finished)
                    && let Some(exec_status) = &mut status
                {
                    exec_status(i);
                }

                if finished {
                    debug!(
                        "execute >>> i={i}, elapsed={elapsed:?}, acc_latency={acc_latency:?}, src_finished={src_finished}"
                    );
                    break;
                }
            }
        }
    }
}

/// Repeatedly invokes `src.next()`, collects the resulting latency data in a
/// [`BenchOut`] object, and *optionally* reports progress status during benchmark
/// execution.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly invoking
/// `src.next()` for [`BenchCfg::warmup_millis`] milliseconds.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `src` - iterator yielding arrays of measured latencies.
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
/// - `s` - status handler for reporting warm-up and execution progress.
pub fn bench_run_x<'a, const K: usize, S: Status<'a>>(
    cfg: &BenchCfg,
    mut src: impl LatencySrc<K>,
    run_length: RunLength,
    mut s: S,
) -> BenchOut<K> {
    debug!("bench_run_x >>> run_length={run_length:?}");
    let mut state = BenchOut::new(cfg);
    let execs_per_second = cfg.execs_per_sec(&mut src, run_length);
    debug!("bench_run_x >>> execs_per_second={execs_per_second}");

    let warmup_run_length = RunLength::Time(Duration::from_millis(cfg.warmup_millis()));
    let warmup_est_time = warmup_run_length.estimated_time(execs_per_second);
    let warmup_est_count = warmup_run_length.estimated_count(execs_per_second);
    let exec_est_time = run_length.estimated_time(execs_per_second);
    let exec_est_count = run_length.estimated_count(execs_per_second);

    // Warm-up.
    let warmup_status = S::part_apply(s.warmup_status(), warmup_est_time, warmup_est_count);
    let warmup_status_count = if warmup_status.is_some() {
        cfg.status_count(execs_per_second)
    } else {
        usize::MAX
    };
    debug!("bench_run_x >>> warmup_status_count={warmup_status_count}");
    state.execute(
        &mut src,
        warmup_run_length,
        warmup_status_count,
        warmup_status,
    );
    if let Some(end_warmup_status) = s.end_warmup_status() {
        end_warmup_status();
    }
    state.reset();

    // Execute.
    let exec_status = S::part_apply(s.exec_status(), exec_est_time, exec_est_count);
    let exec_status_count = if exec_status.is_some() {
        cfg.status_count(execs_per_second)
    } else {
        usize::MAX
    };
    debug!("bench_run_x >>> exec_status_count={exec_status_count}");
    state.execute(&mut src, run_length, exec_status_count, exec_status);
    if let Some(end_exec_status) = s.end_exec_status() {
        end_exec_status();
    }

    state
}

/// Repeatedly invokes `src.next()`, collects the resulting latency data in a
/// [`BenchOut`] object, and *optionally* reports progress status during benchmark
/// execution.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
/// This function calls [`bench_run_arg_cfg`] with the default bench configuration.
///
/// Arguments:
/// - `f` - benchmark target.
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run<const K: usize>(src: impl LatencySrc<K>, run_length: RunLength) -> BenchOut<K> {
    let cfg = BenchCfg::default();
    bench_run_arg_cfg(&cfg, src, run_length)
}

/// Repeatedly invokes `src.next()`, collects the resulting latency data in a
/// [`BenchOut`] object, and *optionally* reports progress status during benchmark
/// execution.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
/// This function calls [`bench_run_x`] with a no-op progress status handler.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f` - benchmark target.
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run_arg_cfg<const K: usize>(
    cfg: &BenchCfg,
    src: impl LatencySrc<K>,
    run_length: RunLength,
) -> BenchOut<K> {
    bench_run_x(cfg, src, run_length, NoStatus)
}

/// Repeatedly invokes `src.next()`, collects the resulting latency data in a
/// [`BenchOut`] object, and *optionally* reports progress status during benchmark
/// execution.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
/// This function calls [`bench_run_with_status_arg_cfg`] with the default bench configuration.
///
/// Arguments:
/// - `f` - benchmark target.
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run_with_status<const K: usize>(
    src: impl LatencySrc<K>,
    run_length: RunLength,
) -> BenchOut<K> {
    let cfg = BenchCfg::default();
    bench_run_with_status_arg_cfg(&cfg, src, run_length)
}

/// Repeatedly invokes `src.next()`, collects the resulting latency data in a
/// [`BenchOut`] object, and *optionally* reports progress status during benchmark
/// execution.
///
/// Prior to data collection, the benchmark is "warmed-up" by repeatedly executing `f` for
/// [`BenchCfg::warmup_millis`] milliseconds.
/// This function calls [`bench_run_x`] with a pre-defined status handler that outputs
/// benchmark status to `stderr`.
///
/// Arguments:
/// - `cfg` - bench configuration used to run the benchmark.
/// - `f` - benchmark target.
/// - `run_length` - target run length (iteration count and/or duration) for data collection.
pub fn bench_run_with_status_arg_cfg<const K: usize>(
    cfg: &BenchCfg,
    src: impl LatencySrc<K>,
    run_length: RunLength,
) -> BenchOut<K> {
    let mut w = stderr();

    // The `\n` below is to separate warmup status from exec status. Otherwise, they get mixed up due to
    // the `eprint!("{}", "\u{8}".repeat(status_len))` line in the `status` closure.
    let s = DefaultStatus::new(
        &mut w,
        "Warming up".to_owned(),
        "Executing bench_run".to_owned(),
    );

    bench_run_x(cfg, src, run_length, s)
}

#[cfg(test)]
#[cfg(feature = "_test")]
// cargo test -r --package bench_utils --lib --all-features -- multi::bench_run::status --nocapture --test-threads=1
mod status {
    use super::*;
    use crate::{
        LatencyUnit, RunLength, multi::test_support::ConstLatencySrc, status::DefaultStatus,
        test_support::StringWriter,
    };
    use basic_stats::rel_approx_eq;
    use regex::Regex;
    use std::time::Duration;

    fn run<const K: usize, Src>(
        mut src: Src,
        base_warmup_millis: u64,
        base_run_length: RunLength,
        base_status_millis: u64,
        base_target_latency: Duration,
        epsilon: f64,
    ) where
        Src: LatencySrc<K>,
    {
        _ = env_logger::try_init();

        assert!(1 <= K && K <= 2, "K={K} must be 1 or 2");

        // Scale certain arguments to align with status tests between K = 1 and 2.
        let warmup_millis = base_warmup_millis * K as u64;
        let status_millis = base_status_millis * K as u64;
        let run_length = match base_run_length {
            RunLength::Count(_) => base_run_length,
            RunLength::Time(duration) => RunLength::Time(duration * 2),
            RunLength::CountWithTimeout(count, duration) => {
                RunLength::CountWithTimeout(count, duration * 2)
            }
        };

        println!(
            "\n***** Testing: warmup_millis={warmup_millis}, run_length={run_length:?}, status_millis={status_millis}, target_latency={base_target_latency:?}, epsilon={epsilon}"
        );

        let warmup_run_length = RunLength::Time(Duration::from_millis(warmup_millis));

        let cfg = BenchCfg::default()
            .with_warmup_millis(warmup_millis)
            .with_status_millis(status_millis)
            .with_recording_unit(LatencyUnit::Nano);

        let mut w = StringWriter::new();
        let status = DefaultStatus::new(
            &mut w,
            "Warming up".to_owned(),
            "Executing bench_run".to_owned(),
        );

        let execs_per_second = cfg.execs_per_sec(&mut src, run_length);

        let out = bench_run_x(&cfg, src, run_length, status);

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
            let warmup_est_count = caps[3].parse::<u64>().unwrap();
            rel_approx_eq!(
                warmup_est_count as f64,
                warmup_run_length.estimated_count(execs_per_second) as f64,
                epsilon
            );
        }

        {
            rel_approx_eq!(
                caps[4].parse::<u64>().unwrap() as f64,
                run_length.estimated_time(execs_per_second).as_millis() as f64,
                epsilon
            );
            let exec_last = caps[5].parse::<u64>().unwrap();
            let exec_est_count = caps[6].parse::<u64>().unwrap();
            rel_approx_eq!(
                exec_est_count as f64,
                run_length.estimated_count(execs_per_second) as f64,
                epsilon
            );
            assert_eq!(out.n(), exec_last);
        }
    }

    // Use `ConstLatencySrc` to allow accurate checking of status output.

    fn src1(base_target_latency: Duration) -> impl LatencySrc<1> {
        ConstLatencySrc::new(1, [base_target_latency.into()])
    }

    fn src2(base_target_latency: Duration) -> impl LatencySrc<2> {
        let delta = base_target_latency / 10;
        ConstLatencySrc::new(
            1,
            [
                (base_target_latency + delta).into(),
                (base_target_latency - delta).into(),
            ],
        )
    }

    mod status1 {
        use super::*;

        fn run_test(
            base_warmup_millis: u64,
            base_run_length: RunLength,
            base_status_millis: u64,
            base_target_latency: Duration,
            epsilon: f64,
        ) {
            run(
                src1(base_target_latency),
                base_warmup_millis,
                base_run_length,
                base_status_millis,
                base_target_latency,
                epsilon,
            )
        }

        #[test]
        fn test_0_10_0_200() {
            const EPSILON: f64 = 0.001;

            let warmup_millis: u64 = 0;
            let target_latency = Duration::from_micros(10);
            let status_millis: u64 = 0;
            let run_length = RunLength::Count(200);

            run_test(
                warmup_millis,
                run_length,
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
            let run_length = RunLength::Count(0);

            // exec_count must be > 0 but run_length makes it 0
            let result = {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_test(
                        warmup_millis,
                        run_length,
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
            const EPSILON: f64 = 0.001;

            let warmup_millis: u64 = 0;
            let target_latency = Duration::from_micros(10);
            let status_millis: u64 = 1;
            let run_length = RunLength::Count(200);

            run_test(
                warmup_millis,
                run_length,
                status_millis,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_1_10_0_200() {
            const EPSILON: f64 = 0.001;

            let warmup_millis: u64 = 1;
            let target_latency = Duration::from_micros(10);
            let status_millis: u64 = 0;
            let run_length = RunLength::Count(200);

            run_test(
                warmup_millis,
                run_length,
                status_millis,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_1_10_1_200() {
            const EPSILON: f64 = 0.001;

            let warmup_millis: u64 = 1;
            let target_latency = Duration::from_micros(10);
            let status_millis: u64 = 1;
            let run_length = RunLength::Count(200);

            run_test(
                warmup_millis,
                run_length,
                status_millis,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_1_10_1_300() {
            const EPSILON: f64 = 0.001;

            let warmup_millis: u64 = 1;
            let target_latency = Duration::from_micros(10);
            let status_millis: u64 = 1;
            let run_length = RunLength::Count(300);

            run_test(
                warmup_millis,
                run_length,
                status_millis,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_1_10_1_2000() {
            const EPSILON: f64 = 0.001;

            let warmup_millis: u64 = 1;
            let target_latency = Duration::from_micros(10);
            let status_millis: u64 = 1;
            let run_length = RunLength::CountWithTimeout(300, Duration::from_micros(2000));

            run_test(
                warmup_millis,
                run_length,
                status_millis,
                target_latency,
                EPSILON,
            );
        }
    }

    mod status2 {
        use super::*;

        fn run_test(
            base_warmup_millis: u64,
            base_run_length: RunLength,
            base_status_millis: u64,
            base_target_latency: Duration,
            epsilon: f64,
        ) {
            run(
                src2(base_target_latency),
                base_warmup_millis,
                base_run_length,
                base_status_millis,
                base_target_latency,
                epsilon,
            )
        }

        #[test]
        fn test_0_10_0_200() {
            const EPSILON: f64 = 0.001;

            let warmup_millis: u64 = 0;
            let target_latency = Duration::from_micros(10);
            let status_millis: u64 = 0;
            let run_length = RunLength::Count(200);

            run_test(
                warmup_millis,
                run_length,
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
            let run_length = RunLength::Count(0);

            // exec_count must be > 0 but run_length makes it 0
            let result = {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_test(
                        warmup_millis,
                        run_length,
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
            const EPSILON: f64 = 0.001;

            let warmup_millis: u64 = 0;
            let target_latency = Duration::from_micros(10);
            let status_millis: u64 = 1;
            let run_length = RunLength::Count(200);

            run_test(
                warmup_millis,
                run_length,
                status_millis,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_1_10_0_200() {
            const EPSILON: f64 = 0.001;

            let warmup_millis: u64 = 1;
            let target_latency = Duration::from_micros(10);
            let status_millis: u64 = 0;
            let run_length = RunLength::Count(200);

            run_test(
                warmup_millis,
                run_length,
                status_millis,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_1_10_1_200() {
            const EPSILON: f64 = 0.001;

            let warmup_millis: u64 = 1;
            let target_latency = Duration::from_micros(10);
            let status_millis: u64 = 1;
            let run_length = RunLength::Count(200);

            run_test(
                warmup_millis,
                run_length,
                status_millis,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_1_10_1_300() {
            const EPSILON: f64 = 0.001;

            let warmup_millis: u64 = 1;
            let target_latency = Duration::from_micros(10);
            let status_millis: u64 = 1;
            let run_length = RunLength::Count(300);

            run_test(
                warmup_millis,
                run_length,
                status_millis,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_1_10_1_2000() {
            const EPSILON: f64 = 0.001;

            let warmup_millis: u64 = 1;
            let target_latency = Duration::from_micros(10);
            let status_millis: u64 = 1;
            let run_length = RunLength::CountWithTimeout(300, Duration::from_micros(2000));

            run_test(
                warmup_millis,
                run_length,
                status_millis,
                target_latency,
                EPSILON,
            );
        }
    }
}

#[cfg(test)]
#[cfg(feature = "_test")]
/// Tests created by Claude Code, improved a bit by me.
mod simple_tests {
    use super::*;
    use crate::multi::test_support::LognormalLatencySrc;
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
        let out = bench_run_arg_cfg(&cfg, LatencySrc1::new(|| ()), RunLength::Count(5));
        // With 5 count and no timeout, we should have exactly 5 iterations
        assert_eq!(out.n(), 5);
    }

    #[test]
    fn test_bench_run_with_time() {
        let cfg = quick_cfg();

        // Use a very short timeout that should be exceeded immediately
        let out = bench_run_arg_cfg(
            &cfg,
            LatencySrc1::new(|| thread::sleep(Duration::from_nanos(1))),
            RunLength::Time(Duration::from_nanos(1)),
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
            LatencySrc1::new(|| thread::sleep(Duration::from_nanos(1))),
            RunLength::CountWithTimeout(20, Duration::from_nanos(1)),
        );
        // At least some executions should have been captured
        assert!(out.n() > 0 && out.n() < 20);
    }

    #[test]
    fn test_bench_run_time_with_synthetic_source() {
        let src =
            LognormalLatencySrc::<1>::new_with_default_sigmas(1, [FpSeconds::from_millis(10)]);
        let out = bench_run_arg_cfg(
            &quick_cfg(),
            src,
            RunLength::Time(Duration::from_millis(500)),
        );
        // ~10ms per iteration → ~50 iterations expected
        assert!(out.n() >= 30 && out.n() <= 70);
    }

    #[test]
    fn test_bench_run_count_with_timeout_synthetic() {
        let src = LognormalLatencySrc::<1>::new_with_default_sigmas(1, [FpSeconds::from_millis(5)]);
        let out = bench_run_arg_cfg(
            &quick_cfg(),
            src,
            RunLength::CountWithTimeout(200, Duration::from_millis(500)),
        );
        // 5ms per iteration, 500ms timeout → stops well before 200
        assert!(out.n() > 50 && out.n() < 200);
    }
}
