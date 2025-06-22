use basic_stats::{dev_utils::ApproxEq, rel_approx_eq};
use bench_utils::{bench_run_with_status, get_bench_cfg};
use std::{thread, time::Duration};

const EPSILON: f64 = 0.005;
const TARGET_LATENCY: Duration = Duration::from_millis(60); // large enough to dominate noise

fn f() {
    thread::sleep(TARGET_LATENCY);
}

fn main() {
    let reporting_unit = get_bench_cfg().reporting_unit();
    let target_median = reporting_unit.latency_as_f64(TARGET_LATENCY);
    let exec_count = 50;
    let out = bench_run_with_status(f, exec_count, |_| println!("validate_bench_run"));
    println!(
        "target_median={target_median}, out.median()={}, rel_diff={}",
        out.median(),
        target_median.abs_rel_diff(out.median(), 0.)
    );
    rel_approx_eq!(target_median, out.median(), EPSILON);
}
