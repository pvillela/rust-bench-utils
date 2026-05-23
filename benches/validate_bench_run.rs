use basic_stats::{dev_utils::ApproxEq, rel_approx_eq};
use bench_utils::{BenchCfg, RunLength, bench_run_with_status_arg_cfg};
use std::{thread, time::Duration};

const EPSILON: f64 = 0.005;
const BENCH_TIME: Duration = Duration::from_secs(3);

fn sleep_fn(target_latency: Duration) {
    thread::sleep(target_latency);
}

fn run_bench(name: &'static str, warmup_millis: u64, target_latency: Duration, check: bool) {
    let reporting_unit = BenchCfg::default().reporting_unit();
    let target_median = reporting_unit.latency_as_f64(target_latency);
    let exec_count = (reporting_unit.latency_as_f64(BENCH_TIME) / target_median) as usize;
    let cfg = BenchCfg::default().with_warmup_millis(warmup_millis);
    println!("validate_bench_run: {name}");
    let out = bench_run_with_status_arg_cfg(
        &cfg,
        || sleep_fn(target_latency),
        RunLength::Count(exec_count),
    );
    println!(
        "target_median={target_median}, out.median()={}, rel_diff={}",
        out.median(),
        target_median.abs_rel_diff(out.median())
    );
    println!("{:?}", out.summary());
    println!();

    if check {
        rel_approx_eq!(target_median, out.median(), EPSILON);
    }
}

fn main() {
    // sleep long enough to dominate noise
    run_bench("sleep_60_millis", 600, Duration::from_millis(60), true);

    // short sleep, very noisy
    run_bench("sleep_60_micros", 100, Duration::from_micros(60), false);
}
