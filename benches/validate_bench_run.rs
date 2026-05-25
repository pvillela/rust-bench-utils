use bench_utils::{
    BenchCfg, RunLength, bench_run_with_status_arg_cfg, rel_approx_eq_dur,
    test_support::AbsRelDiffDur,
};
use std::{thread, time::Duration};

const EPSILON: f64 = 0.005;
const BENCH_TIME: Duration = Duration::from_secs(3);

fn sleep_fn(target_latency: Duration) {
    thread::sleep(target_latency);
}

fn run_bench(name: &'static str, warmup_millis: u64, target_latency: Duration, check: bool) {
    let exec_count = (BENCH_TIME.as_secs_f64() / target_latency.as_secs_f64()) as usize;
    let cfg = BenchCfg::default().with_warmup_millis(warmup_millis);
    println!("validate_bench_run: {name}");
    let out = bench_run_with_status_arg_cfg(
        &cfg,
        || sleep_fn(target_latency),
        RunLength::Count(exec_count),
    );
    let out_median = out.median();
    println!(
        "target_median={target_latency:?}, out.median()={out_median:?}, rel_diff={}",
        target_latency.abs_rel_diff(out_median)
    );
    println!("{:?}", out.summary());
    println!();

    if check {
        rel_approx_eq_dur!(target_latency, out_median, EPSILON);
    }
}

fn main() {
    // sleep long enough to dominate noise
    run_bench("sleep_60_millis", 600, Duration::from_millis(60), true);

    // short sleep, very noisy
    run_bench("sleep_60_micros", 100, Duration::from_micros(60), true);
}
