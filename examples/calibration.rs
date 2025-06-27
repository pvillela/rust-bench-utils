//! Example demonstrating [`busy_work`] and [`calibrate_busy_work`].
//! This example requires the undocumented feature "_bench".
//!
//! To run the example:
//! ```
//! cargo run -r --example calibration --features busy_work
//! ```

use bench_utils::{busy_work, calibrate_busy_work, latency};
use std::time::Duration;

fn main() {
    let target_latency = Duration::from_nanos(2000);
    let target_latency_nanos = target_latency.as_nanos() as f64;
    let target_effort = calibrate_busy_work(target_latency);

    println!("target_latency_nanos={}", target_latency.as_nanos());
    println!("target_effort={}", target_effort);

    const N: usize = 10;

    let mut sum2dev = 0.;
    for _ in 0..N {
        let latency_nanos = latency(|| busy_work(target_effort)).as_nanos() as f64;
        sum2dev += (latency_nanos - target_latency_nanos).powi(2);

        println!("latency_nanos={}", latency_nanos);
    }

    let stdev = (sum2dev / N as f64).sqrt();
    let rel_stdev = stdev / target_latency_nanos;

    println!("stdev={stdev}, rel_stdev={rel_stdev}");
}
