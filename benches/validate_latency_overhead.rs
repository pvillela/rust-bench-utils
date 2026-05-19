use basic_stats::rel_approx_eq;
use bench_utils::{BenchCfg, bench_support::validate_latency_overhead};
use std::time::{Duration, Instant};

fn main() {
    const EPSILON: f64 = 0.1;

    let start = Instant::now();
    let cfg = BenchCfg::default().with_warmup_millis(500);

    let bench_time = Duration::from_millis(500);
    let target_latency = Duration::from_micros(100);

    let (solo_median_20, group_median_20) =
        validate_latency_overhead(&cfg, bench_time, target_latency, 20, EPSILON);
    let (solo_median_100, group_median_100) =
        validate_latency_overhead(&cfg, bench_time, target_latency, 100, EPSILON);

    println!("elapsed time: {} millis", start.elapsed().as_millis());

    rel_approx_eq!(solo_median_20 * 20., group_median_20, EPSILON);
    rel_approx_eq!(solo_median_100 * 100., group_median_100, EPSILON);
}
