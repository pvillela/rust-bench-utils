use basic_stats::{dev_utils::ApproxEq, rel_approx_eq};
use bench_utils::{bench_run_with_status, get_bench_cfg};
use std::{thread, time::Duration};

const EPSILON: f64 = 0.005;
const BENCH_TIME: Duration = Duration::from_secs(3);

fn sleep_fn(target_latency: Duration) {
    thread::sleep(target_latency);
}

fn run_bench(name: &'static str, target_latency: Duration, check: bool) {
    let reporting_unit = get_bench_cfg().reporting_unit();
    let target_median = reporting_unit.latency_as_f64(target_latency);
    let exec_count = (reporting_unit.latency_as_f64(BENCH_TIME) / target_median) as usize;
    let out = bench_run_with_status(
        || sleep_fn(target_latency),
        exec_count,
        |_| {
            println!("validate_bench_run: {name}");
        },
    );
    println!(
        "target_median={target_median}, out.median()={}, rel_diff={}",
        out.median(),
        target_median.abs_rel_diff(out.median(), 0.)
    );
    println!("{:?}", out.summary());
    println!();

    if check {
        rel_approx_eq!(target_median, out.median(), EPSILON);
    }
}

fn main() {
    // sleep long enough to dominate noise
    run_bench("sleep_60_millis", Duration::from_millis(60), true);

    // short sleep, very noisy
    run_bench("sleep_60_micros", Duration::from_micros(60), false);
}
