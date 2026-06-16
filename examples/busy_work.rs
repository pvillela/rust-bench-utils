//! Example demonstrating [`BusyWork`] latency distribution with target latency and batching parameters.
//! This example requires the feature "load".
//!
//! To run the example:
//! ```
//! cargo run -r --example busy_work --features load
//! ```

use bench_utils::{BenchCfg, RunLength, bench_run_arg_cfg, bench_run_arg_cfg_b, load::BusyWork};
use env_logger;
use std::time::{Duration, Instant};

fn run(target_latency: Duration, warmup_millis: u64, run_length: RunLength, batch: Option<u32>) {
    let effort = BusyWork::calibrate(target_latency);
    println!(
        "\nRunning with target_latency={target_latency:?}, effort={effort}, run_length={run_length:?}, batch={batch:?}"
    );
    let f = BusyWork::fun(effort);
    let cfg = BenchCfg::default().with_warmup_millis(warmup_millis);

    let start = Instant::now();
    let out = match batch {
        None => bench_run_arg_cfg(&cfg, f, run_length),
        Some(batch) => bench_run_arg_cfg_b(&cfg, f, run_length, batch),
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

    const WARMUP_MILLIS: u64 = 100;
    const RUN_LENGTH: RunLength = RunLength::Time(Duration::from_millis(100));

    //=== Nanos
    {
        {
            let target_latency = Duration::from_nanos(1);
            let batch = Some(1_000_000);
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }

        {
            let target_latency = Duration::from_nanos(10);
            let batch = Some(100_000);
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }

        {
            let target_latency = Duration::from_nanos(100);
            let batch = Some(10_000);
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }

        {
            let target_latency = Duration::from_nanos(100);
            let batch = Some(100_000);
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }
    }

    //=== Micros
    {
        {
            let target_latency = Duration::from_micros(1);
            let batch = None;
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }

        {
            let target_latency = Duration::from_micros(1);
            let batch = Some(10);
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }

        {
            let target_latency = Duration::from_micros(1);
            let batch = Some(100);
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }

        {
            let target_latency = Duration::from_micros(1);
            let batch = Some(1000);
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }

        {
            let target_latency = Duration::from_micros(10);
            let batch = None;
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }

        {
            let target_latency = Duration::from_micros(10);
            let batch = Some(10);
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }

        {
            let target_latency = Duration::from_micros(10);
            let batch = Some(100);
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }

        {
            let target_latency = Duration::from_micros(10);
            let batch = Some(1000);
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }

        {
            let target_latency = Duration::from_micros(100);
            let batch = None;
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }

        {
            let target_latency = Duration::from_micros(100);
            let batch = Some(10);
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }

        {
            let target_latency = Duration::from_micros(100);
            let batch = Some(100);
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }

        {
            const WARMUP_MILLIS: u64 = 1000;
            const RUN_LENGTH: RunLength = RunLength::Time(Duration::from_millis(1000));
            let target_latency = Duration::from_micros(100);
            let batch = Some(1000);
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }
    }

    //=== Millis
    {
        const WARMUP_MILLIS: u64 = 1000;
        const RUN_LENGTH: RunLength = RunLength::Time(Duration::from_millis(1000));

        {
            let target_latency = Duration::from_millis(1);
            let batch = None;
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }

        {
            let target_latency = Duration::from_millis(1);
            let batch = Some(10);
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }

        {
            let target_latency = Duration::from_millis(1);
            let batch = Some(100);
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }

        {
            let target_latency = Duration::from_millis(10);
            let batch = None;
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }

        {
            let target_latency = Duration::from_millis(10);
            let batch = Some(10);
            run(target_latency, WARMUP_MILLIS, RUN_LENGTH, batch);
        }
    }
}
