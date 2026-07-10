use bench_utils::duo::bench_run_parallel_arg_cfg;
use bench_utils::{BenchCfg, BenchOut, RunLength};
use std::time::Duration;

/// Benchmark two closures in parallel for 1 second with custom configuration and batching.
fn main() {
    let cfg = BenchCfg::default().with_warmup_millis(500);
    let batch = Some(10);
    let out = bench_run_parallel_arg_cfg(
        &cfg,
        || std::thread::sleep(Duration::from_micros(10)),
        || std::thread::sleep(Duration::from_micros(20)),
        RunLength::Time(Duration::from_secs(1)),
        batch,
    );
    println!(
        "groups={}, batch={:?}, executions={}, mean_latencies={:?}",
        out.n_r(),
        batch,
        out.n(),
        out.map(BenchOut::mean)
    );
}
