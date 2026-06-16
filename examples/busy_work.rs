//! Example demonstrating [`BusyWork`].
//! This example requires the feature "load".
//!
//! To run the example:
//! ```
//! cargo run -r --example busy_work --features load
//! ```

use bench_utils::{latency, load::BusyWork};
use std::time::Duration;

fn main() {
    const N: u64 = 10;

    let target_latency = Duration::from_nanos(2000);
    let target_latency_nanos = target_latency.as_nanos() as f64;
    println!("target_latency_nanos={}", target_latency.as_nanos());

    let effort = BusyWork::calibrate(target_latency);
    let f = BusyWork::fun(effort);

    let mut sum2dev = 0.;
    for _ in 0..N {
        let latency_nanos = latency(&f).as_nanos() as f64;
        sum2dev += (latency_nanos - target_latency_nanos).powi(2);

        println!("latency_nanos={}", latency_nanos);
    }

    let stdev = (sum2dev / N as f64).sqrt();
    let rel_stdev = stdev / target_latency_nanos;

    println!("stdev={stdev}, rel_stdev={rel_stdev}");
}
