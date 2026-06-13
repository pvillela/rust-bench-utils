#![cfg(feature = "_bench")]

//! cargo test -r --test bench_run_validate --all-features -- grouped_10 ungrouped --nocapture --test-threads=1

use bench_utils::{
    BenchCfg, BusyWork, LatencyUnit, RunLength, latency,
    multi::{
        BenchOut, LatencySrc, LatencySrc1, LatencySrc1n, LatencySrc2, LatencySrc2n,
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
}

impl<F0, F1, Src> FnsLatencySrc<F0, F1, Src> {
    fn new(f0: F0, f1: F1, src: Src) -> Self {
        Self { f0, f1, src }
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
    } = fsrc;

    let warmup_millis = base_warmup_millis * K as u64;
    let bench_time = base_bench_time * K as u32;
    let status_millis = base_status_millis * K as u64;
    let group_size = src.group_size() as u64;

    let exec_count = (bench_time.as_secs_f64() / (base_target_latency * K as u32).as_secs_f64())
        .round() as usize;

    println!(
        "validate_bench_run: K={K}, base_target_latency={base_target_latency:?}, warmup={warmup_millis}, bench_time={bench_time:?}, group_size={group_size}, exec_count={exec_count}"
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

// cargo test -r --test bench_run_validate --all-features -- ungrouped --nocapture --test-threads=1
mod ungrouped {
    use super::*;

    fn fsrc1(
        base_target_latency: Duration,
    ) -> FnsLatencySrc<impl Fn() + Clone, impl Fn() + Clone, impl LatencySrc<1>> {
        let f = BusyWork::new(base_target_latency).fun();
        let src = LatencySrc1::new(f.clone());
        FnsLatencySrc::new(f, || (), src)
    }

    fn fsrc2(
        base_target_latency: Duration,
    ) -> FnsLatencySrc<impl Fn() + Clone, impl Fn() + Clone, impl LatencySrc<2>> {
        let bw0 = BusyWork::new(base_target_latency);
        let effort0 = bw0.effort();
        let effort_delta = effort0 / 10;

        let f0 = bw0.fun();
        let f1a = BusyWork::from_effort(effort0 - effort_delta).fun();
        let f1b = BusyWork::from_effort(effort_delta).fun();
        let f1 = move || {
            f1a();
            f1b()
        };

        let src = LatencySrc2::new(f0.clone(), f1.clone());
        FnsLatencySrc::new(f0, f1, src)
    }

    // cargo test -r --test bench_run_validate --all-features -- ungrouped::with_status1 --nocapture --test-threads=1
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

    // cargo test -r --test bench_run_validate --all-features -- ungrouped::without_status1 --nocapture --test-threads=1
    mod without_status1 {
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

    // cargo test -r --test bench_run_validate --all-features -- ungrouped::with_status2 --nocapture --test-threads=1
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

    // cargo test -r --test bench_run_validate --all-features -- ungrouped::without_status2 --nocapture --test-threads=1
    mod without_status2 {
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

// cargo test -r --test bench_run_validate --all-features -- grouped_10 --nocapture --test-threads=1
mod grouped_10 {
    use super::*;

    const GROUP_SIZE: usize = 10;

    fn fsrc1_grouped(
        base_target_latency: Duration,
        group_size: usize,
    ) -> FnsLatencySrc<impl Fn() + Clone, impl Fn() + Clone, impl LatencySrc<1>> {
        let f = BusyWork::new(base_target_latency).fun();
        let src = LatencySrc1n::new(f.clone(), group_size);
        FnsLatencySrc::new(f, || (), src)
    }

    fn fsrc2_grouped(
        base_target_latency: Duration,
        group_size: usize,
    ) -> FnsLatencySrc<impl Fn() + Clone, impl Fn() + Clone, impl LatencySrc<2>> {
        let bw0 = BusyWork::new(base_target_latency);
        let effort0 = bw0.effort();
        let effort_delta = effort0 / 10;

        let f0 = bw0.fun();
        let f1a = BusyWork::from_effort(effort0 - effort_delta).fun();
        let f1b = BusyWork::from_effort(effort_delta).fun();
        let f1 = move || {
            f1a();
            f1b()
        };

        let src = LatencySrc2n::new(f0.clone(), f1.clone(), group_size);
        FnsLatencySrc::new(f0, f1, src)
    }

    // cargo test -r --test bench_run_validate --all-features -- grouped_10::with_status1 --nocapture --test-threads=1
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
                fsrc1_grouped(base_target_latency, GROUP_SIZE),
                base_warmup_millis,
                base_status_millis,
                base_bench_time,
                base_target_latency,
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

    // cargo test -r --test bench_run_validate --all-features -- grouped_10::without_status1 --nocapture --test-threads=1
    mod without_status1 {
        use super::*;

        fn run_bench(
            base_warmup_millis: u64,
            base_bench_time: Duration,
            base_target_latency: Duration,
            epsilon: f64,
        ) {
            run(
                bench_run_arg_cfg,
                fsrc1_grouped(base_target_latency, GROUP_SIZE),
                base_warmup_millis,
                0,
                base_bench_time,
                base_target_latency,
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

    // cargo test -r --test bench_run_validate --all-features -- grouped_10::with_status2 --nocapture --test-threads=1
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
                fsrc2_grouped(base_target_latency, GROUP_SIZE),
                base_warmup_millis,
                base_status_millis,
                base_bench_time,
                base_target_latency,
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

    // cargo test -r --test bench_run_validate --all-features -- grouped_10::without_status2 --nocapture --test-threads=1
    mod without_status2 {
        use super::*;

        fn run_bench(
            base_warmup_millis: u64,
            base_bench_time: Duration,
            base_target_latency: Duration,
            epsilon: f64,
        ) {
            run(
                bench_run_arg_cfg,
                fsrc2_grouped(base_target_latency, GROUP_SIZE),
                base_warmup_millis,
                0,
                base_bench_time,
                base_target_latency,
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
