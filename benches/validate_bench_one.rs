use basic_stats::dev_utils::ApproxEq;
use bench_utils::{LatencyUnit, bench_one_with_status};
use std::{thread, time::Duration};

const EPSILON: f64 = 0.005;
const TARGET_LATENCY: Duration = Duration::from_millis(60); // large enough to dominate noise

fn f() {
    thread::sleep(TARGET_LATENCY);
}

fn main() {
    let unit = LatencyUnit::Micro;
    let target_median = unit.latency_as_f64(TARGET_LATENCY);
    let exec_count = 50;
    let out = bench_one_with_status(unit, f, exec_count, |_, _| println!("validate_bench_one"));
    println!(
        "target_median={target_median}, out.median()={}, rel_diff={}",
        out.median(),
        target_median.rel_diff(out.median())
    );
    assert!(target_median.rel_approx_eq(out.median(), EPSILON));
}
