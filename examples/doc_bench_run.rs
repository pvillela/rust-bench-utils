use bench_utils::{RunLength, bench_run};

/// Benchmark a no-op closure for 1000 iterations with default configuration, no batching.
fn main() {
    let out = bench_run(|| {}, RunLength::Count(1000), None);
    println!("median latency: {:?}", out.median_r());
}
