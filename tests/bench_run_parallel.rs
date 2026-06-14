#![cfg(feature = "_bench")]

//! Example of two benches running in parallel on separate threads.
//!
//! cargo test -r --test bench_run_parallel --all-features -- --nocapture --test-threads=1

use bench_utils::{BusyWork, RunLength, bench_run};
use std::{thread, time::Duration};

#[test]
fn test() {
    const RUN_TIME: Duration = Duration::from_millis(100);
    const TARGET_BASE_LATENCY: Duration = Duration::from_micros(100);
    const TARGET_MEDIAN_RATIO: f64 = 1.05;
    const EPSILON: f64 = 0.01;

    let exec_run_length = RunLength::Time(RUN_TIME);
    let bw1 = BusyWork::new(TARGET_BASE_LATENCY);
    let effort1 = bw1.effort();
    let effort2 = (effort1 as f64 / TARGET_MEDIAN_RATIO).round() as u32;
    let f1 = bw1.fun();
    let f2 = BusyWork::from_effort(effort2).fun();

    let h1 = thread::spawn(move || {
        println!("running bench on thread={:?}", thread::current());
        bench_run(f1, exec_run_length)
    });
    let h2 = thread::spawn(move || {
        println!("running bench on thread={:?}", thread::current());
        bench_run(f2, exec_run_length)
    });

    let out1 = h1.join().unwrap();
    let out2 = h2.join().unwrap();
    let median_ratio = out1.median().as_secs_f64() / out2.median().as_secs_f64();

    println!("median_ratio={median_ratio}");
    println!("out1.summary={:?}", out1.summary());
    println!("out2.summary={:?}", out2.summary());
}
