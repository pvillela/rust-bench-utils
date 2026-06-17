#![cfg(feature = "_bench")]

//! cargo test -r --test bench_run_validate --all-features -- with_batch no_batch --nocapture --test-threads=1

use bench_utils::{
    BenchCfg, LatencyUnit, RunLength, latency,
    load::BusyWork,
    multi::{
        BenchOut, LatencySrc, LatencySrc1, LatencySrc1b, LatencySrc2, LatencySrc2b,
        bench_run_arg_cfg, bench_run_with_status_arg_cfg,
    },
    rel_approx_eq_dur,
    test_support::AbsRelDiffDur,
};
use std::time::{Duration, Instant};

struct FnsLatencySrc<F0, F1, Src> {
    f0: F0,
    f1: F1,
    src: Src,
    batch: Option<u32>,
}

impl<F0, F1, Src> FnsLatencySrc<F0, F1, Src> {
    fn new(f0: F0, f1: F1, src: Src, batch: Option<u32>) -> Self {
        Self { f0, f1, src, batch }
    }
}

fn run<const K: usize, R, F0, F1, Src>(
    runner: R,
    fsrc: FnsLatencySrc<F0, F1, Src>,
    base_warmup_millis: u64,
    base_status_millis: u64,
    base_bench_time: Duration,
    base_target_latency: Duration,
    epsilon: f64,
) where
    F0: FnMut(),
    F1: FnMut(),
    Src: LatencySrc<K>,
    R: FnOnce(&BenchCfg, Src, RunLength) -> BenchOut<K>,
{
    _ = env_logger::try_init();

    assert!(1 <= K && K <= 2, "K={K} must be 1 or 2");
    let start = Instant::now();

    let FnsLatencySrc {
        mut f0,
        mut f1,
        src,
        batch,
    } = fsrc;

    let warmup_millis = base_warmup_millis * K as u64;
    let bench_time = base_bench_time * K as u32;
    let status_millis = base_status_millis * K as u64;

    let exec_count = (bench_time.as_secs_f64() / (base_target_latency * K as u32).as_secs_f64())
        .round() as usize;

    println!(
        "validate_bench_run: K={K}, base_target_latency={base_target_latency:?}, warmup={warmup_millis}, bench_time={bench_time:?}, batch={batch:?}, exec_count={exec_count}"
    );

    let cfg = BenchCfg::default()
        .with_warmup_millis(warmup_millis)
        .with_status_millis(status_millis)
        .with_recording_unit(LatencyUnit::Nano);
    let out = runner(&cfg, src, RunLength::Count(exec_count));

    let mut raw_latencies = Vec::<Duration>::new();
    if K >= 1 {
        raw_latencies.push(latency(|| {
            for _ in 0..exec_count {
                f0();
            }
        }));
    }
    if K == 2 {
        raw_latencies.push(latency(|| {
            for _ in 0..exec_count {
                f1();
            }
        }));
    }

    let raw_means = raw_latencies
        .iter()
        .map(|lat| *lat / exec_count as u32)
        .collect::<Vec<_>>();

    for i in 0..K {
        let out_mean = out[i].mean();
        let raw_mean = raw_means[i];

        println!(
            "target_mean={base_target_latency:?}, out[{i}].mean()={out_mean:?}, rel_diff={}",
            base_target_latency.abs_rel_diff(out_mean)
        );

        println!(
            "target_mean={base_target_latency:?}, raw_mean[{i}]={raw_mean:?}, rel_diff={}",
            base_target_latency.abs_rel_diff(raw_mean)
        );

        println!(
            "raw_mean[{i}]={raw_mean:?}, out[{i}].mean()={out_mean:?}, rel_diff={}",
            raw_mean.abs_rel_diff(out_mean)
        );
    }

    let aggregate_raw_mean = raw_latencies.iter().sum::<Duration>() / exec_count as u32;
    let aggregate_out_mean = out.iter().map(|x| x.mean()).sum();

    if K >= 2 {
        println!(
            "aggregate_raw_mean={aggregate_raw_mean:?}, aggregate_out_mean={aggregate_out_mean:?}, rel_diff={}",
            aggregate_raw_mean.abs_rel_diff(aggregate_out_mean)
        );
    }

    println!("test total elapsed time = {:?}", start.elapsed());

    // Assertions
    {
        if K >= 2 {
            rel_approx_eq_dur!(
                aggregate_raw_mean,
                aggregate_out_mean,
                epsilon / (K as f64).sqrt()
            );
        }

        for i in 0..K {
            rel_approx_eq_dur!(raw_means[i], out[i].mean(), epsilon);
        }
    }
}

const BASE_WARMUP_MILLIS: u64 = 100;
const BASE_STATUS_MILLIS: u64 = 10;
const BASE_BENCH_TIME: Duration = Duration::from_millis(100);

// cargo test -r --test bench_run_validate --all-features -- no_batch --nocapture --test-threads=1
mod no_batch {
    use super::*;

    fn fsrc1(
        base_target_latency: Duration,
    ) -> FnsLatencySrc<impl FnMut() + Clone, impl FnMut() + Clone, impl LatencySrc<1>> {
        let effort = BusyWork::calibrate(base_target_latency);
        let f = BusyWork::fun(effort);
        let src = LatencySrc1::new(f.clone());
        FnsLatencySrc::new(f, || (), src, None)
    }

    fn fsrc2(
        base_target_latency: Duration,
    ) -> FnsLatencySrc<impl FnMut() + Clone, impl FnMut() + Clone, impl LatencySrc<2>> {
        let effort0 = BusyWork::calibrate(base_target_latency);
        let effort_delta = effort0 / 10;

        let f0 = BusyWork::fun(effort0);
        let mut f1a = BusyWork::fun(effort0 - effort_delta);
        let mut f1b = BusyWork::fun(effort_delta);
        let f1 = move || {
            f1a();
            f1b()
        };

        let src = LatencySrc2::new(f0.clone(), f1.clone());
        FnsLatencySrc::new(f0, f1, src, None)
    }

    // cargo test -r --test bench_run_validate --all-features -- no_batch::with_status1 --nocapture --test-threads=1
    mod with_status1 {
        use super::*;

        fn run_bench(
            base_warmup_millis: u64,
            base_status_millis: u64,
            base_bench_time: Duration,
            base_target_latency: Duration,
            epsilon: f64,
        ) {
            run(
                bench_run_with_status_arg_cfg,
                fsrc1(base_target_latency),
                base_warmup_millis,
                base_status_millis,
                base_bench_time,
                base_target_latency,
                epsilon,
            );
        }

        #[test]
        fn test_micros_50() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(50);
            run_bench(
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_millis_10() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_millis(10);
            run_bench(
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }
    }

    // cargo test -r --test bench_run_validate --all-features -- no_batch::no_status1 --nocapture --test-threads=1
    mod no_status1 {
        use super::*;

        fn run_bench(
            base_warmup_millis: u64,
            base_bench_time: Duration,
            base_target_latency: Duration,
            epsilon: f64,
        ) {
            run(
                bench_run_arg_cfg,
                fsrc1(base_target_latency),
                base_warmup_millis,
                0,
                base_bench_time,
                base_target_latency,
                epsilon,
            );
        }

        #[test]
        fn test_micros_50() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(50);
            run_bench(BASE_WARMUP_MILLIS, BASE_BENCH_TIME, target_latency, EPSILON);
        }

        #[test]
        fn test_millis_10() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_millis(10);
            run_bench(BASE_WARMUP_MILLIS, BASE_BENCH_TIME, target_latency, EPSILON);
        }
    }

    // cargo test -r --test bench_run_validate --all-features -- no_batch::with_status2 --nocapture --test-threads=1
    mod with_status2 {
        use super::*;

        fn run_bench(
            base_warmup_millis: u64,
            base_status_millis: u64,
            base_bench_time: Duration,
            base_target_latency: Duration,
            epsilon: f64,
        ) {
            run(
                bench_run_with_status_arg_cfg,
                fsrc2(base_target_latency),
                base_warmup_millis,
                base_status_millis,
                base_bench_time,
                base_target_latency,
                epsilon,
            );
        }

        #[test]
        fn test_micros_50() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(50);
            run_bench(
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_millis_10() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_millis(10);
            run_bench(
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }
    }

    // cargo test -r --test bench_run_validate --all-features -- no_batch::no_status2 --nocapture --test-threads=1
    mod no_status2 {
        use super::*;

        fn run_bench(
            base_warmup_millis: u64,
            base_bench_time: Duration,
            base_target_latency: Duration,
            epsilon: f64,
        ) {
            run(
                bench_run_arg_cfg,
                fsrc2(base_target_latency),
                base_warmup_millis,
                0,
                base_bench_time,
                base_target_latency,
                epsilon,
            );
        }

        #[test]
        fn test_micros_50() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(50);
            run_bench(BASE_WARMUP_MILLIS, BASE_BENCH_TIME, target_latency, EPSILON);
        }

        #[test]
        fn test_millis_10() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_millis(10);
            run_bench(BASE_WARMUP_MILLIS, BASE_BENCH_TIME, target_latency, EPSILON);
        }
    }
}

// cargo test -r --test bench_run_validate --all-features -- with_batch --nocapture --test-threads=1
mod with_batch {
    use super::*;

    fn fsrc1_b(
        base_target_latency: Duration,
        batch: u32,
    ) -> FnsLatencySrc<impl FnMut() + Clone, impl FnMut() + Clone, impl LatencySrc<1>> {
        let effort = BusyWork::calibrate(base_target_latency);
        let f = BusyWork::fun(effort);
        let src = LatencySrc1b::new(f.clone(), batch);
        FnsLatencySrc::new(f, || (), src, Some(batch))
    }

    fn fsrc2_b(
        base_target_latency: Duration,
        batch: u32,
    ) -> FnsLatencySrc<impl FnMut() + Clone, impl FnMut() + Clone, impl LatencySrc<2>> {
        let effort0 = BusyWork::calibrate(base_target_latency);
        let effort_delta = effort0 / 10;

        let f0 = BusyWork::fun(effort0);
        let mut f1a = BusyWork::fun(effort0 - effort_delta);
        let mut f1b = BusyWork::fun(effort_delta);
        let f1 = move || {
            f1a();
            f1b()
        };

        let src = LatencySrc2b::new(f0.clone(), f1.clone(), batch);
        FnsLatencySrc::new(f0, f1, src, Some(batch))
    }

    /// Calculates a batch size that yields `n` batches given the `target_latency` and `bench_time`.
    fn batch_n(n: u32, target_latency: Duration, bench_time: Duration) -> u32 {
        (bench_time.as_nanos() / target_latency.as_nanos()) as u32 / n
    }

    // cargo test -r --test bench_run_validate --all-features -- with_batch::with_status1 --nocapture --test-threads=1
    mod with_status1 {
        use super::*;

        fn run_bench(
            base_warmup_millis: u64,
            base_status_millis: u64,
            base_bench_time: Duration,
            base_target_latency: Duration,
            batch: u32,
            epsilon: f64,
        ) {
            run(
                bench_run_with_status_arg_cfg,
                fsrc1_b(base_target_latency, batch),
                base_warmup_millis,
                base_status_millis,
                base_bench_time,
                base_target_latency,
                epsilon,
            );
        }

        #[test]
        fn test_micros_50_() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(50);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        mod millis {
            use super::*;

            const BASE_WARMUP_MILLIS: u64 = 1000;
            const BASE_BENCH_TIME: Duration = Duration::from_millis(1000);

            #[test]
            fn test_millis_10() {
                const EPSILON: f64 = 0.02;
                let target_latency = Duration::from_millis(10);
                let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
                run_bench(
                    BASE_WARMUP_MILLIS,
                    BASE_STATUS_MILLIS,
                    BASE_BENCH_TIME,
                    target_latency,
                    batch,
                    EPSILON,
                );
            }
        }
    }

    // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status1 --nocapture --test-threads=1
    mod no_status1 {
        use super::*;

        fn run_bench(
            base_warmup_millis: u64,
            base_bench_time: Duration,
            base_target_latency: Duration,
            batch: u32,
            epsilon: f64,
        ) {
            run(
                bench_run_arg_cfg,
                fsrc1_b(base_target_latency, batch),
                base_warmup_millis,
                0,
                base_bench_time,
                base_target_latency,
                epsilon,
            );
        }

        #[test]
        fn test_micros_50_() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(50);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        mod millis {
            use super::*;

            const BASE_WARMUP_MILLIS: u64 = 1000;
            const BASE_BENCH_TIME: Duration = Duration::from_millis(1000);

            #[test]
            fn test_millis_10() {
                const EPSILON: f64 = 0.02;
                let target_latency = Duration::from_millis(10);
                let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
                run_bench(
                    BASE_WARMUP_MILLIS,
                    BASE_BENCH_TIME,
                    target_latency,
                    batch,
                    EPSILON,
                );
            }
        }
    }

    // cargo test -r --test bench_run_validate --all-features -- with_batch::with_status2 --nocapture --test-threads=1
    mod with_status2 {
        use super::*;

        fn run_bench(
            base_warmup_millis: u64,
            base_status_millis: u64,
            base_bench_time: Duration,
            base_target_latency: Duration,
            batch: u32,
            epsilon: f64,
        ) {
            run(
                bench_run_with_status_arg_cfg,
                fsrc2_b(base_target_latency, batch),
                base_warmup_millis,
                base_status_millis,
                base_bench_time,
                base_target_latency,
                epsilon,
            );
        }

        #[test]
        fn test_micros_50_() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(50);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        mod millis {
            use super::*;

            const BASE_WARMUP_MILLIS: u64 = 1000;
            const BASE_BENCH_TIME: Duration = Duration::from_millis(1000);

            #[test]
            fn test_millis_10() {
                const EPSILON: f64 = 0.02;
                let target_latency = Duration::from_millis(10);
                let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
                run_bench(
                    BASE_WARMUP_MILLIS,
                    BASE_STATUS_MILLIS,
                    BASE_BENCH_TIME,
                    target_latency,
                    batch,
                    EPSILON,
                );
            }
        }
    }

    // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status2 --nocapture --test-threads=1
    mod no_status2 {
        use super::*;

        fn run_bench(
            base_warmup_millis: u64,
            base_bench_time: Duration,
            base_target_latency: Duration,
            batch: u32,
            epsilon: f64,
        ) {
            run(
                bench_run_arg_cfg,
                fsrc2_b(base_target_latency, batch),
                base_warmup_millis,
                0,
                base_bench_time,
                base_target_latency,
                epsilon,
            );
        }

        #[test]
        fn test_micros_50_() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(50);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        mod millis {
            use super::*;

            const BASE_WARMUP_MILLIS: u64 = 1000;
            const BASE_BENCH_TIME: Duration = Duration::from_millis(1000);

            #[test]
            fn test_millis_10() {
                const EPSILON: f64 = 0.02;
                let target_latency = Duration::from_millis(10);
                let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
                run_bench(
                    BASE_WARMUP_MILLIS,
                    BASE_BENCH_TIME,
                    target_latency,
                    batch,
                    EPSILON,
                );
            }
        }
    }
}
