//! Example of two benches running in parallel on separate threads.
//! This example requires the feature "load".
//!
//! To run the example:
//! ```
//! cargo run -r --example parallel --features load
//! ```

use bench_utils::{BenchCfg, RunLength, duo::bench_run_parallel_arg_cfg, load::BusyWork};
use std::time::{Duration, Instant};

fn main() {
    let start_total = Instant::now();

    const WARMUP_MILLIS: u64 = 100;
    const RUN_TIME: Duration = Duration::from_millis(100);
    const TARGET_BASE_LATENCY: Duration = Duration::from_micros(100);
    const TARGET_MEDIAN_RATIO: f64 = 1.05;

    let cfg = BenchCfg::default().with_warmup_millis(WARMUP_MILLIS);
    let exec_run_length = RunLength::Time(RUN_TIME);
    let effort1 = BusyWork::calibrate(TARGET_BASE_LATENCY);
    let effort2 = (effort1 as f64 / TARGET_MEDIAN_RATIO).round() as u32;
    let f1 = BusyWork::fun(effort1);
    let f2 = BusyWork::fun(effort2);

    let start = Instant::now();

    let out = bench_run_parallel_arg_cfg(&cfg, f1, f2, exec_run_length);

    let elapsed = start.elapsed();

    let out1 = out.out_f1();
    let out2 = out.out_f2();

    let median_ratio = out1.median().as_f64() / out2.median().as_f64();

    println!("median_ratio={median_ratio}");
    println!("out1.summary={:?}", out1.summary());
    println!("out2.summary={:?}", out2.summary());

    let elapsed_total = start_total.elapsed();
    println!("*** elapsed_in_threads={elapsed:?}, elapsed_total={elapsed_total:?}");
}
