use basic_stats::{dev_utils::ApproxEq, rel_approx_eq};
use bench_utils::{BenchCfg, RunLength, bench_run_with_status_arg_cfg};
use std::{thread, time::Duration};

const EPSILON: f64 = 0.005;
const BENCH_TIME: Duration = Duration::from_secs(3);

fn sleep_fn(target_latency: Duration) {
    thread::sleep(target_latency);
}

fn run_bench(name: &'static str, warmup_millis: u64, target_latency: Duration, check: bool) {
    let recording_unit = BenchCfg::default().recording_unit();
    let target_median = recording_unit.latency_as_f64(target_latency);
    let exec_count = (recording_unit.latency_as_f64(BENCH_TIME) / target_median) as usize;
    let cfg = BenchCfg::default().with_warmup_millis(warmup_millis);
    println!("validate_bench_run: {name}");
    let out = bench_run_with_status_arg_cfg(
        &cfg,
        || sleep_fn(target_latency),
        RunLength::Count(exec_count),
    );
    let out_median_ns = out.median().as_nanos() as f64;
    println!(
        "target_median={target_median}, out.median()={out_median_ns}, rel_diff={}",
        target_median.abs_rel_diff(out_median_ns)
    );
    println!("{:?}", out.summary());
    println!();

    if check {
        rel_approx_eq!(target_median, out_median_ns, EPSILON);
    }
}

fn main() {
    // sleep long enough to dominate noise
    run_bench("sleep_60_millis", 600, Duration::from_millis(60), true);

    // short sleep, very noisy
    run_bench("sleep_60_micros", 100, Duration::from_micros(60), false);
}
