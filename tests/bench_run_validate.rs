#![cfg(feature = "_bench")]

//! cargo test -r --test bench_run_validate --all-features -- with_batch no_batch --nocapture --test-threads=1

use basic_stats::{dev_utils::ApproxEq, rel_approx_eq};
use bench_utils::{
    BenchCfg, LatencyUnit, RunLength, latency,
    load::BusyWork,
    multi::{
        BenchOut, LatencySrc, LatencySrc1, LatencySrc1b, LatencySrc2, LatencySrc2b,
        bench_run_arg_cfg, bench_run_with_status_arg_cfg,
    },
};
use std::time::{Duration, Instant};

struct FnsLatencySrc<F1, F2, Src> {
    f1: F1,
    f2: F2,
    src: Src,
    batch: Option<usize>,
}

impl<F1, F2, Src> FnsLatencySrc<F1, F2, Src> {
    fn new(f1: F1, f2: F2, src: Src, batch: Option<usize>) -> Self {
        Self { f1, f2, src, batch }
    }
}

fn run<const K: usize, R, F1, F2, Src>(
    rec_unit: LatencyUnit,
    runner: R,
    fsrc: FnsLatencySrc<F1, F2, Src>,
    base_warmup_millis: u64,
    base_status_millis: u64,
    base_bench_time: Duration,
    base_target_latency: Duration,
    epsilon: f64,
) where
    F1: FnMut(),
    F2: FnMut(),
    Src: LatencySrc<K>,
    R: FnOnce(&BenchCfg, Src, RunLength) -> BenchOut<K>,
{
    _ = env_logger::try_init();

    assert!(1 <= K && K <= 2, "K={K} must be 1 or 2");
    let start = Instant::now();

    let FnsLatencySrc {
        mut f1,
        mut f2,
        src,
        batch,
    } = fsrc;

    let warmup_millis = base_warmup_millis * K as u64;
    let bench_time = base_bench_time * K as u32;
    let status_millis = base_status_millis * K as u64;

    println!(
        "validate_bench_run: K={K}, rec_unit={rec_unit:?}, base_target_latency={base_target_latency:?}, warmup={warmup_millis}, bench_time={bench_time:?}, batch={batch:?}"
    );

    let cfg = BenchCfg::default()
        .with_warmup_millis(warmup_millis)
        .with_status_millis(status_millis)
        .with_recording_unit(rec_unit);
    let out = runner(&cfg, src, RunLength::Time(bench_time));
    let count = out.n();

    let mut raw_latencies = Vec::<Duration>::new();
    if K >= 1 {
        raw_latencies.push(latency(|| {
            for _ in 0..count {
                f1();
            }
        }));
    }
    if K == 2 {
        raw_latencies.push(latency(|| {
            for _ in 0..count {
                f2();
            }
        }));
    }

    let raw_means = raw_latencies
        .iter()
        .map(|lat| lat.as_secs_f64() / count as f64)
        .collect::<Vec<_>>();

    let base_target_latency_secs_f64 = base_target_latency.as_secs_f64();

    for i in 0..K {
        let out_mean_secs_f64 = out[i].mean().as_f64();
        let raw_mean = raw_means[i];

        println!(
            "base_target_latency_secs_f64={base_target_latency_secs_f64:?}, out[{i}].mean().as_f64={out_mean_secs_f64:?}, rel_diff={}",
            base_target_latency_secs_f64.abs_rel_diff(out_mean_secs_f64)
        );

        println!(
            "base_target_latency_secs_f64={base_target_latency_secs_f64:?}, raw_mean[{i}]={raw_mean:?}, rel_diff={}",
            base_target_latency_secs_f64.abs_rel_diff(raw_mean)
        );

        println!(
            "raw_mean[{i}]={raw_mean:?}, out[{i}].mean_as_f64()={out_mean_secs_f64:?}, rel_diff={}",
            raw_mean.abs_rel_diff(out_mean_secs_f64)
        );

        println!("out[{i}].hist().mean()={:?}", out[i].hist().mean());

        println!("out[{i}].summary()={:?}", out[i].summary());
    }

    let aggregate_raw_mean = raw_latencies.iter().sum::<Duration>().as_secs_f64() / count as f64;
    let aggregate_out_mean = out.iter().map(|x| x.mean().0).sum();

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
            rel_approx_eq!(
                aggregate_raw_mean,
                aggregate_out_mean,
                epsilon / (K as f64).sqrt()
            );
        }

        for i in 0..K {
            rel_approx_eq!(raw_means[i], out[i].mean().0, epsilon);
        }
    }
}

const BASE_WARMUP_MILLIS: u64 = 100;
const BASE_STATUS_MILLIS: u64 = 10;
const BASE_BENCH_TIME: Duration = Duration::from_millis(100);
const DEFAULT_REC_UNIT: LatencyUnit = LatencyUnit::Nano;

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

        let f1 = BusyWork::fun(effort0);
        let mut f2a = BusyWork::fun(effort0 - effort_delta);
        let mut f2b = BusyWork::fun(effort_delta);
        let f2 = move || {
            f2a();
            f2b()
        };

        let src = LatencySrc2::new(f1.clone(), f2.clone());
        FnsLatencySrc::new(f1, f2, src, None)
    }

    // cargo test -r --test bench_run_validate --all-features -- no_batch::no_status1 --nocapture --test-threads=1
    mod no_status1 {
        use super::*;

        fn run_bench(
            rec_unit: LatencyUnit,
            base_warmup_millis: u64,
            base_bench_time: Duration,
            base_target_latency: Duration,
            epsilon: f64,
        ) {
            run(
                rec_unit,
                bench_run_arg_cfg,
                fsrc1(base_target_latency),
                base_warmup_millis,
                0,
                base_bench_time,
                base_target_latency,
                epsilon,
            );
        }

        // #[test]
        // fn test_nanos_1() {
        //     const EPSILON: f64 = 0.02;
        //     let rec_unit = LatencyUnit::SubSec(11);
        //     let target_latency = Duration::from_nanos(1);
        //     run_bench(
        //         rec_unit,
        //         BASE_WARMUP_MILLIS,
        //         BASE_BENCH_TIME,
        //         target_latency,
        //         EPSILON,
        //     );
        // }

        // cargo test -r --test bench_run_validate --all-features -- no_batch::no_status1::test_nanos_50 --nocapture --test-threads=1
        #[test]
        fn test_nanos_50() {
            const EPSILON: f64 = 0.20;
            let rec_unit = LatencyUnit::SubSec(11);
            let target_latency = Duration::from_nanos(50);
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }

        // cargo test -r --test bench_run_validate --all-features -- no_batch::no_status1::test_micros_1 --nocapture --test-threads=1
        #[test]
        fn test_micros_1() {
            const EPSILON: f64 = 0.03;
            let target_latency = Duration::from_micros(1);
            run_bench(
                DEFAULT_REC_UNIT,
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_micros_50() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(50);
            run_bench(
                DEFAULT_REC_UNIT,
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_millis_1() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_millis(1);
            run_bench(
                DEFAULT_REC_UNIT,
                BASE_WARMUP_MILLIS,
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
                DEFAULT_REC_UNIT,
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }
    }

    // cargo test -r --test bench_run_validate --all-features -- no_batch::with_status1 --nocapture --test-threads=1
    mod with_status1 {
        use super::*;

        fn run_bench(
            rec_unit: LatencyUnit,
            base_warmup_millis: u64,
            base_status_millis: u64,
            base_bench_time: Duration,
            base_target_latency: Duration,
            epsilon: f64,
        ) {
            run(
                rec_unit,
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
        fn test_nanos_1() {
            const EPSILON: f64 = 0.02;
            let rec_unit = LatencyUnit::SubSec(11);
            let target_latency = Duration::from_nanos(1);
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_nanos_50() {
            const EPSILON: f64 = 0.02;
            let rec_unit = LatencyUnit::SubSec(11);
            let target_latency = Duration::from_nanos(50);
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_micros_1() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(1);
            run_bench(
                DEFAULT_REC_UNIT,
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_micros_50() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(50);
            run_bench(
                DEFAULT_REC_UNIT,
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_millis_1() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_millis(1);
            run_bench(
                DEFAULT_REC_UNIT,
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
                DEFAULT_REC_UNIT,
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
            rec_unit: LatencyUnit,
            base_warmup_millis: u64,
            base_bench_time: Duration,
            base_target_latency: Duration,
            epsilon: f64,
        ) {
            run(
                rec_unit,
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
        fn test_nanos_1() {
            const EPSILON: f64 = 0.02;
            let rec_unit = LatencyUnit::SubSec(11);
            let target_latency = Duration::from_nanos(1);
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_nanos_50() {
            const EPSILON: f64 = 0.02;
            let rec_unit = LatencyUnit::SubSec(11);
            let target_latency = Duration::from_nanos(50);
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_micros_1() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(1);
            run_bench(
                DEFAULT_REC_UNIT,
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_micros_50() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(50);
            run_bench(
                DEFAULT_REC_UNIT,
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_millis_1() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_millis(1);
            run_bench(
                DEFAULT_REC_UNIT,
                BASE_WARMUP_MILLIS,
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
                DEFAULT_REC_UNIT,
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }
    }

    // cargo test -r --test bench_run_validate --all-features -- no_batch::with_status2 --nocapture --test-threads=1
    mod with_status2 {
        use super::*;

        fn run_bench(
            rec_unit: LatencyUnit,
            base_warmup_millis: u64,
            base_status_millis: u64,
            base_bench_time: Duration,
            base_target_latency: Duration,
            epsilon: f64,
        ) {
            run(
                rec_unit,
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
        fn test_nanos_1() {
            const EPSILON: f64 = 0.02;
            let rec_unit = LatencyUnit::SubSec(11);
            let target_latency = Duration::from_nanos(1);
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_nanos_50() {
            const EPSILON: f64 = 0.02;
            let rec_unit = LatencyUnit::SubSec(11);
            let target_latency = Duration::from_nanos(50);
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_micros_1() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(1);
            run_bench(
                DEFAULT_REC_UNIT,
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_micros_50() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(50);
            run_bench(
                DEFAULT_REC_UNIT,
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }

        #[test]
        fn test_millis_1() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_millis(1);
            run_bench(
                DEFAULT_REC_UNIT,
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
                DEFAULT_REC_UNIT,
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                EPSILON,
            );
        }
    }
}

// cargo test -r --test bench_run_validate --all-features -- with_batch --nocapture --test-threads=1
mod with_batch {
    use super::*;

    fn fsrc1_b(
        base_target_latency: Duration,
        batch: usize,
    ) -> FnsLatencySrc<impl FnMut() + Clone, impl FnMut() + Clone, impl LatencySrc<1>> {
        let effort = BusyWork::calibrate(base_target_latency);
        let f = BusyWork::fun(effort);
        let src = LatencySrc1b::new(f.clone(), batch);
        FnsLatencySrc::new(f, || (), src, Some(batch))
    }

    fn fsrc2_b(
        base_target_latency: Duration,
        batch: usize,
    ) -> FnsLatencySrc<impl FnMut() + Clone, impl FnMut() + Clone, impl LatencySrc<2>> {
        let effort0 = BusyWork::calibrate(base_target_latency);
        let effort_delta = effort0 / 10;

        let f1 = BusyWork::fun(effort0);
        let mut f2a = BusyWork::fun(effort0 - effort_delta);
        let mut f2b = BusyWork::fun(effort_delta);
        let f2 = move || {
            f2a();
            f2b()
        };

        let src = LatencySrc2b::new(f1.clone(), f2.clone(), batch);
        FnsLatencySrc::new(f1, f2, src, Some(batch))
    }

    /// Calculates a batch size that yields `n` batches given the `target_latency` and `bench_time`.
    fn batch_n(n: usize, target_latency: Duration, bench_time: Duration) -> usize {
        (bench_time.as_nanos() / target_latency.as_nanos()) as usize / n
    }

    // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status1 --nocapture --test-threads=1
    mod no_status1 {
        use super::*;

        fn run_bench(
            rec_unit: LatencyUnit,
            base_warmup_millis: u64,
            base_bench_time: Duration,
            base_target_latency: Duration,
            batch: usize,
            epsilon: f64,
        ) {
            run(
                rec_unit,
                bench_run_arg_cfg,
                fsrc1_b(base_target_latency, batch),
                base_warmup_millis,
                0,
                base_bench_time,
                base_target_latency,
                epsilon,
            );
        }

        //=== nanos(1) is unstable
        // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status1::test_nanos_01 --nocapture --test-threads=1
        #[test]
        fn test_nanos_01() {
            const EPSILON: f64 = 0.10;
            let rec_unit = LatencyUnit::SubSec(11);
            // let rec_unit = LatencyUnit::Nano;
            let target_latency = Duration::from_nanos(1);
            let batch = 10_000_000;
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status1::test_nanos_10 --nocapture --test-threads=1
        #[test]
        fn test_nanos_10() {
            const EPSILON: f64 = 0.10;
            let rec_unit = LatencyUnit::SubSec(11);
            let target_latency = Duration::from_nanos(10);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status1::test_nanos_50 --nocapture --test-threads=1
        #[test]
        fn test_nanos_50() {
            const EPSILON: f64 = 0.05;
            let rec_unit = LatencyUnit::SubSec(11);
            let target_latency = Duration::from_nanos(50);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status1::test_micros_1 --exact --nocapture --test-threads=1
        #[test]
        fn test_micros_1() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(1);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                DEFAULT_REC_UNIT,
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status1::test_micros_50_ --nocapture --test-threads=1
        #[test]
        fn test_micros_50_() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(50);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                DEFAULT_REC_UNIT,
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        mod millis {
            use super::*;

            const BASE_WARMUP_MILLIS: u64 = 500;
            const BASE_BENCH_TIME: Duration = Duration::from_millis(500);

            // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status1::millis::test_millis_1 --exact --nocapture --test-threads=1
            #[test]
            fn test_millis_1() {
                const EPSILON: f64 = 0.01;
                let target_latency = Duration::from_millis(1);
                let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
                run_bench(
                    DEFAULT_REC_UNIT,
                    BASE_WARMUP_MILLIS,
                    BASE_BENCH_TIME,
                    target_latency,
                    batch,
                    EPSILON,
                );
            }

            // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status1::millis::test_millis_10 --nocapture --test-threads=1
            #[test]
            fn test_millis_10() {
                const EPSILON: f64 = 0.02;
                let target_latency = Duration::from_millis(10);
                let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
                run_bench(
                    DEFAULT_REC_UNIT,
                    BASE_WARMUP_MILLIS,
                    BASE_BENCH_TIME,
                    target_latency,
                    batch,
                    EPSILON,
                );
            }
        }
    }

    // cargo test -r --test bench_run_validate --all-features -- with_batch::with_status1 --nocapture --test-threads=1
    mod with_status1 {
        use super::*;

        fn run_bench(
            rec_unit: LatencyUnit,
            base_warmup_millis: u64,
            base_status_millis: u64,
            base_bench_time: Duration,
            base_target_latency: Duration,
            batch: usize,
            epsilon: f64,
        ) {
            run(
                rec_unit,
                bench_run_with_status_arg_cfg,
                fsrc1_b(base_target_latency, batch),
                base_warmup_millis,
                base_status_millis,
                base_bench_time,
                base_target_latency,
                epsilon,
            );
        }

        //=== nanos(1) is unstable
        #[test]
        fn test_nanos_1() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_nanos(1);
            let rec_unit = LatencyUnit::SubSec(11);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status1::test_nanos_10 --nocapture --test-threads=1
        #[test]
        fn test_nanos_10() {
            const EPSILON: f64 = 0.10;
            let rec_unit = LatencyUnit::SubSec(11);
            let target_latency = Duration::from_nanos(10);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status1::test_nanos_50 --nocapture --test-threads=1
        #[test]
        fn test_nanos_50() {
            const EPSILON: f64 = 0.05;
            let rec_unit = LatencyUnit::SubSec(11);
            let target_latency = Duration::from_nanos(50);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status1::test_micros_1 --nocapture --test-threads=1
        #[test]
        fn test_micros_1() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(1);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                DEFAULT_REC_UNIT,
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status1::test_micros_50_ --nocapture --test-threads=1
        #[test]
        fn test_micros_50_() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(50);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                DEFAULT_REC_UNIT,
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

            const BASE_WARMUP_MILLIS: u64 = 500;
            const BASE_BENCH_TIME: Duration = Duration::from_millis(500);

            // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status1::millis::test_millis_1 --exact  --nocapture --test-threads=1
            #[test]
            fn test_millis_1() {
                const EPSILON: f64 = 0.05;
                let target_latency = Duration::from_millis(1);
                let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
                run_bench(
                    DEFAULT_REC_UNIT,
                    BASE_WARMUP_MILLIS,
                    BASE_STATUS_MILLIS,
                    BASE_BENCH_TIME,
                    target_latency,
                    batch,
                    EPSILON,
                );
            }

            // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status1::millis::test_millis_10 --nocapture --test-threads=1
            #[test]
            fn test_millis_10() {
                const EPSILON: f64 = 0.05;
                let target_latency = Duration::from_millis(10);
                let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
                run_bench(
                    DEFAULT_REC_UNIT,
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
            rec_unit: LatencyUnit,
            base_warmup_millis: u64,
            base_bench_time: Duration,
            base_target_latency: Duration,
            batch: usize,
            epsilon: f64,
        ) {
            run(
                rec_unit,
                bench_run_arg_cfg,
                fsrc2_b(base_target_latency, batch),
                base_warmup_millis,
                0,
                base_bench_time,
                base_target_latency,
                epsilon,
            );
        }

        //=== nanos(1) is unstable
        #[test]
        fn test_nanos_1() {
            const EPSILON: f64 = 0.02;
            let rec_unit = LatencyUnit::SubSec(11);
            let target_latency = Duration::from_nanos(1);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status2::test_nanos_10 --nocapture --test-threads=1
        #[test]
        fn test_nanos_10() {
            const EPSILON: f64 = 0.10;
            let rec_unit = LatencyUnit::SubSec(11);
            let target_latency = Duration::from_nanos(10);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status2::test_nanos_50 --nocapture --test-threads=1
        #[test]
        fn test_nanos_50() {
            const EPSILON: f64 = 0.05;
            let rec_unit = LatencyUnit::SubSec(11);
            let target_latency = Duration::from_nanos(50);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status2::test_micros_1 --nocapture --test-threads=1
        #[test]
        fn test_micros_1() {
            const EPSILON: f64 = 0.01;
            let target_latency = Duration::from_micros(1);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                DEFAULT_REC_UNIT,
                BASE_WARMUP_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status2::test_micros_50_ --nocapture --test-threads=1
        #[test]
        fn test_micros_50_() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(50);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                DEFAULT_REC_UNIT,
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

            // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status2::millis::test_millis_1 --exact --nocapture --test-threads=1
            #[test]
            fn test_millis_1() {
                const EPSILON: f64 = 0.01;
                let target_latency = Duration::from_millis(1);
                let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
                run_bench(
                    DEFAULT_REC_UNIT,
                    BASE_WARMUP_MILLIS,
                    BASE_BENCH_TIME,
                    target_latency,
                    batch,
                    EPSILON,
                );
            }

            // cargo test -r --test bench_run_validate --all-features -- with_batch::no_status2::millis::test_millis_10 --nocapture --test-threads=1
            #[test]
            fn test_millis_10() {
                const EPSILON: f64 = 0.02;
                let target_latency = Duration::from_millis(10);
                let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
                run_bench(
                    DEFAULT_REC_UNIT,
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
            rec_unit: LatencyUnit,
            base_warmup_millis: u64,
            base_status_millis: u64,
            base_bench_time: Duration,
            base_target_latency: Duration,
            batch: usize,
            epsilon: f64,
        ) {
            run(
                rec_unit,
                bench_run_with_status_arg_cfg,
                fsrc2_b(base_target_latency, batch),
                base_warmup_millis,
                base_status_millis,
                base_bench_time,
                base_target_latency,
                epsilon,
            );
        }

        //=== nanos(1) is unstable
        #[test]
        fn test_nanos_1() {
            const EPSILON: f64 = 0.02;
            let rec_unit = LatencyUnit::SubSec(11);
            let target_latency = Duration::from_nanos(1);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        // cargo test -r --test bench_run_validate --all-features -- with_batch::with_status2::test_nanos_10 --nocapture --test-threads=1
        #[test]
        fn test_nanos_10() {
            const EPSILON: f64 = 0.10;
            let rec_unit = LatencyUnit::SubSec(11);
            let target_latency = Duration::from_nanos(10);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        // cargo test -r --test bench_run_validate --all-features -- with_batch::with_status2::test_nanos_50 --nocapture --test-threads=1
        #[test]
        fn test_nanos_50() {
            const EPSILON: f64 = 0.05;
            let rec_unit = LatencyUnit::SubSec(11);
            let target_latency = Duration::from_nanos(50);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                rec_unit,
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        // cargo test -r --test bench_run_validate --all-features -- with_batch::with_status2::test_micros_1 --nocapture --test-threads=1
        #[test]
        fn test_micros_1() {
            const EPSILON: f64 = 0.05;
            let target_latency = Duration::from_micros(1);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                DEFAULT_REC_UNIT,
                BASE_WARMUP_MILLIS,
                BASE_STATUS_MILLIS,
                BASE_BENCH_TIME,
                target_latency,
                batch,
                EPSILON,
            );
        }

        // cargo test -r --test bench_run_validate --all-features -- with_batch::with_status2::test_micros_50_ --nocapture --test-threads=1
        #[test]
        fn test_micros_50_() {
            const EPSILON: f64 = 0.02;
            let target_latency = Duration::from_micros(50);
            let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
            run_bench(
                DEFAULT_REC_UNIT,
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

            // cargo test -r --test bench_run_validate --all-features -- with_batch::with_status2::millis::test_millis_1 --exact --nocapture --test-threads=1
            #[test]
            fn test_millis_1() {
                const EPSILON: f64 = 0.05;
                let target_latency = Duration::from_millis(1);
                let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
                run_bench(
                    DEFAULT_REC_UNIT,
                    BASE_WARMUP_MILLIS,
                    BASE_STATUS_MILLIS,
                    BASE_BENCH_TIME,
                    target_latency,
                    batch,
                    EPSILON,
                );
            }

            // cargo test -r --test bench_run_validate --all-features -- with_batch::with_status2::millis::test_millis_10 --nocapture --test-threads=1
            #[test]
            fn test_millis_10() {
                const EPSILON: f64 = 0.02;
                let target_latency = Duration::from_millis(10);
                let batch = batch_n(50, target_latency, BASE_BENCH_TIME);
                run_bench(
                    DEFAULT_REC_UNIT,
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
}
