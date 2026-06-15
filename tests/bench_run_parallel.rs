#![cfg(feature = "_bench")]

//! Example of two benches running in parallel on separate threads.
//!
//! cargo test -r --test bench_run_parallel --all-features -- --nocapture --test-threads=1

use basic_stats::rel_approx_eq;
use bench_utils::{BenchCfg, BusyWork, RunLength, duo::bench_run_parallel_arg_cfg};
use std::time::{Duration, Instant};

#[test]
fn test() {
    let start_total = Instant::now();

    const WARMUP_MILLIS: u64 = 100;
    const RUN_TIME: Duration = Duration::from_millis(100);
    const TARGET_BASE_LATENCY: Duration = Duration::from_micros(100);
    const TARGET_MEDIAN_RATIO: f64 = 1.05;
    const EPSILON: f64 = 0.01;

    let cfg = BenchCfg::default().with_warmup_millis(WARMUP_MILLIS);
    let exec_run_length = RunLength::Time(RUN_TIME);
    let bw1 = BusyWork::new(TARGET_BASE_LATENCY);
    let effort1 = bw1.effort();
    let effort2 = (effort1 as f64 / TARGET_MEDIAN_RATIO).round() as u32;
    let f1 = bw1.fun();
    let f2 = BusyWork::from_effort(effort2).fun();

    let start = Instant::now();

    let out = bench_run_parallel_arg_cfg(&cfg, f1, f2, exec_run_length);

    let elapsed = start.elapsed();

    let out1 = out.out_f1();
    let out2 = out.out_f2();

    let median_ratio = out1.median().as_secs_f64() / out2.median().as_secs_f64();

    println!("median_ratio={median_ratio}");
    println!("out1.summary={:?}", out1.summary());
    println!("out2.summary={:?}", out2.summary());

    let elapsed_total = start_total.elapsed();
    println!("*** elapsed_in_threads={elapsed:?}, elapsed_total={elapsed_total:?}");

    rel_approx_eq!(TARGET_MEDIAN_RATIO, median_ratio, EPSILON);
}
