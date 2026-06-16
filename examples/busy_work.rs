//! Example demonstrating [`BusyWork`].
//! This example requires the feature "load".
//!
//! To run the example:
//! ```
//! cargo run -r --example busy_work --features load
//! ```

use bench_utils::{RunLength, bench_run, bench_run_b, load::BusyWork};
use env_logger;
use std::time::{Duration, Instant};

fn run(target_latency: Duration, run_length: RunLength, batch: Option<u32>) {
    println!(
        "\nRunning with target_latency={target_latency:?}, run_length={run_length:?}, batch={batch:?}"
    );
    let effort = BusyWork::calibrate(target_latency);
    let f = BusyWork::fun(effort);
    let start = Instant::now();
    let out = match batch {
        None => bench_run(f, run_length),
        Some(batch) => bench_run_b(f, run_length, batch),
    };
    let elapsed = start.elapsed();
    println!("elaped time={elapsed:?}");
    println!(
        "target_latency/median_latency={}",
        target_latency.as_secs_f64() / out.median().as_secs_f64()
    );
    println!("{:?}", out.summary());
}

fn main() {
    _ = env_logger::try_init();
    // const RUN_LENGTH: RunLength = RunLength::Time(Duration::from_millis(100));
    const RUN_LENGTH: RunLength = RunLength::Count(100_000);

    {
        let target_latency = Duration::from_nanos(10);
        let batch = Some(100_000);
        run(target_latency, RUN_LENGTH, batch);
    }

    {
        let target_latency = Duration::from_nanos(100);
        let batch = Some(10_000);
        run(target_latency, RUN_LENGTH, batch);
    }

    {
        let target_latency = Duration::from_micros(1);
        let batch = None;
        run(target_latency, RUN_LENGTH, batch);
    }

    {
        let target_latency = Duration::from_micros(1);
        let batch = Some(10);
        run(target_latency, RUN_LENGTH, batch);
    }

    {
        let target_latency = Duration::from_micros(1);
        let batch = Some(100);
        run(target_latency, RUN_LENGTH, batch);
    }

    {
        let target_latency = Duration::from_micros(1);
        let batch = Some(1000);
        run(target_latency, RUN_LENGTH, batch);
    }
}
