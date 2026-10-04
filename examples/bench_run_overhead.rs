//! Demonstrates the clock overhead per [`bench_utils::bench_run`] execution iteration.
//!
//! We define "clock overhead" as the time between the two clock reads around each timed
//! execution or batch, incurred once per recorded value whatever the batch size.
//!
//! On my current computer, the clock overhead is ~13ns.
//!
//! ```
//! cargo run -r --example bench_run_overhead --features _test_support
//! ```

use bench_utils::{
    BenchCfg, BenchOut, RunLength,
    multi::{bench_run_x, test_support::LatencySrc0},
    overhead, overhead_x,
    status::DefaultStatus,
};
use std::{io, time::Instant};

/// This function uses [`bench_utils::multi::bench_run_x`] with
/// [`bench_utils::multi::test_support::LatencySrc0`] instead of using an empty clusure.
/// This was done on purpose to demonstrate, by comparison with the runs below using
/// [`bench_utils::overhead_x`], that the two approaches yield similar values for clock
/// overhead.
fn my_bench_run_with_status_arg_cfg(cfg: &BenchCfg, exec_count: usize) -> BenchOut {
    let mut w = io::sink();
    let s = DefaultStatus::new(
        &mut w,
        "Warming up".to_owned(),
        "Executing bench_run".to_owned(),
    );

    bench_run_x(cfg, LatencySrc0, RunLength::Count(exec_count), s).into()
}

fn overhead_x_with_status(exec_count: usize, groups: usize) -> (BenchOut, BenchOut) {
    // Warmup.
    let cfg_w = BenchCfg::default().with_warmup_millis(1000);
    my_bench_run_with_status_arg_cfg(&cfg_w, exec_count);

    let cfg = BenchCfg::default().with_warmup_millis(0);
    let mut pooled_out = BenchOut::new(&cfg, None);
    let mut grouped_out = BenchOut::new(&cfg, None);
    for _ in 0..groups {
        let out = my_bench_run_with_status_arg_cfg(&cfg, exec_count);
        pooled_out.record_from_iter_with_counts(out.iter_with_counts());
        grouped_out.capture_data_with_counts((out.median_r(), 1));
    }
    (pooled_out, grouped_out)
}

fn run_overhead() {
    println!("*** overhead");
    let start = Instant::now();
    let out = overhead();
    println!("elapsed={:?}, {:?}", start.elapsed(), out.summary(),);
}

fn run_overhead_x_no_status(exec_count: usize, groups: usize) {
    println!("\n***** overhead_x (no-status): exec_count={exec_count}, groups={groups}");

    let start = Instant::now();
    let (pooled_out, grouped_out) = overhead_x(exec_count, groups);
    println!(
        "elapsed={:?}, \n== pooled={:?}, \n== grouped={:?}",
        start.elapsed(),
        pooled_out.summary(),
        grouped_out.summary()
    );
}

fn run_overhead_x_with_status(exec_count: usize, groups: usize) {
    println!("\n***** overhead_x_with_status: exec_count={exec_count}, groups={groups}");

    let start = Instant::now();
    let (pooled_out, grouped_out) = overhead_x_with_status(exec_count, groups);
    println!(
        "elapsed={:?}, \n== pooled={:?}, \n== grouped={:?}",
        start.elapsed(),
        pooled_out.summary(),
        grouped_out.summary()
    );
}

fn main() {
    _ = env_logger::try_init();

    let groups = 100;

    run_overhead();

    //=== no_status

    {
        {
            let exec_count = 1;
            run_overhead_x_no_status(exec_count, groups);
        }

        {
            let exec_count = 10;
            run_overhead_x_no_status(exec_count, groups);
        }

        {
            let exec_count = 100;
            run_overhead_x_no_status(exec_count, groups);
        }

        {
            let exec_count = 1000;
            run_overhead_x_no_status(exec_count, groups);
        }

        {
            let exec_count = 10_000;
            run_overhead_x_no_status(exec_count, groups);
        }

        {
            let exec_count = 100_000;
            run_overhead_x_no_status(exec_count, groups);
        }

        {
            let exec_count = 1_000_000;
            run_overhead_x_no_status(exec_count, groups);
        }
    }

    //=== with_status

    {
        {
            let exec_count = 1;
            run_overhead_x_with_status(exec_count, groups);
        }

        {
            let exec_count = 10;
            run_overhead_x_with_status(exec_count, groups);
        }

        {
            let exec_count = 100;
            run_overhead_x_with_status(exec_count, groups);
        }

        {
            let exec_count = 1000;
            run_overhead_x_with_status(exec_count, groups);
        }

        {
            let exec_count = 10_000;
            run_overhead_x_with_status(exec_count, groups);
        }

        {
            let exec_count = 100_000;
            run_overhead_x_with_status(exec_count, groups);
        }

        {
            let exec_count = 1_000_000;
            run_overhead_x_with_status(exec_count, groups);
        }
    }
}
