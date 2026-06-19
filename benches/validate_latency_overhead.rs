use bench_utils::{
    BenchCfg, bench_support::validate_latency_overhead, rel_approx_eq_fpsecs,
    test_support::AbsRelDiffFpSecs,
};
use std::time::{Duration, Instant};

fn main() {
    const EPSILON: f64 = 0.05;

    let start = Instant::now();
    let cfg = BenchCfg::default().with_warmup_millis(100);

    let bench_duration = Duration::from_millis(100);
    let target_latency = Duration::from_micros(50);

    let (solo_median_20, group_median_20) =
        validate_latency_overhead(&cfg, bench_duration, target_latency, 20);
    let (solo_median_100, group_median_100) =
        validate_latency_overhead(&cfg, bench_duration, target_latency, 100);

    println!("elapsed time: {} millis", start.elapsed().as_millis());

    println!(
        "solo_median_20 * 20 = {:?}, group_median_20 = {:?}, abs_rel_diff = {}",
        solo_median_20 * 20,
        group_median_20,
        (solo_median_20 * 20).abs_rel_diff(group_median_20)
    );
    println!(
        "solo_median_100 * 100 = {:?}, group_median_100 = {:?}, abs_rel_diff = {}",
        solo_median_100 * 100,
        group_median_100,
        (solo_median_100 * 100).abs_rel_diff(group_median_100)
    );

    rel_approx_eq_fpsecs!(solo_median_20 * 20, group_median_20, EPSILON);
    rel_approx_eq_fpsecs!(solo_median_100 * 100, group_median_100, EPSILON);
}
