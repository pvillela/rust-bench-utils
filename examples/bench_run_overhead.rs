//! Demonstrates the measurement overhead per [`bench_utils::bench_run`] execution iteration.
//!
//! On my current computer, the overhead is ~13ns.
//!
//! ```
//! cargo run -r --example bench_run_overhead --features _test_support
//! ```

use bench_utils::{
    BenchCfg, BenchOut, FpSeconds, LatencyUnit, RunLength,
    multi::{LatencySrc, bench_run_arg_cfg, bench_run_x, test_support::LatencySrc0},
    status::DefaultStatus,
};
use std::{io, iter, time::Instant};

fn my_bench_run_with_status_arg_cfg(
    cfg: &BenchCfg,
    src: impl LatencySrc<1>,
    run_length: RunLength,
) -> BenchOut {
    let mut w = io::sink();
    let s = DefaultStatus::new(
        &mut w,
        "Warming up".to_owned(),
        "Executing bench_run".to_owned(),
    );

    bench_run_x(cfg, src, run_length, s).into()
}

#[inline(always)]
fn no_status(cfg: &BenchCfg, run_length: RunLength) -> Option<(FpSeconds, usize)> {
    let out = bench_run_arg_cfg(&cfg, LatencySrc0, run_length);
    Some((out.mean(), out.n() as usize))
}

#[inline(always)]
fn with_status(cfg: &BenchCfg, run_length: RunLength) -> Option<(FpSeconds, usize)> {
    let out = my_bench_run_with_status_arg_cfg(&cfg, LatencySrc0, run_length);
    Some((out.mean(), out.n() as usize))
}

fn run_no_status(cfg: &BenchCfg, run_length: RunLength, samp_size: usize) {
    println!("*** no_status, run_length={run_length:?}");
    let start = Instant::now();
    let target_fn = || no_status(cfg, run_length);
    let src = iter::from_fn(target_fn).take(samp_size);
    let out = BenchOut::from_iter_with_counts(&cfg, src);
    println!("elapsed={:?}, {:?}", start.elapsed(), out.summary());
}

fn run_with_status(cfg: &BenchCfg, run_length: RunLength, samp_size: usize) {
    println!("*** with_status, run_length={run_length:?}");
    let start = Instant::now();
    let target_fn = || with_status(cfg, run_length);
    let src = iter::from_fn(target_fn).take(samp_size);
    let out = BenchOut::from_iter_with_counts(&cfg, src);
    println!("elapsed={:?}, {:?}", start.elapsed(), out.summary());
}

fn main() {
    _ = env_logger::try_init();

    let cfg = BenchCfg::default()
        .with_warmup_millis(0)
        .with_recording_unit(LatencyUnit::sub_sec(12));
    let samp_size = 100;

    //=== no_status

    {
        {
            let run_length = RunLength::Count(1);
            run_no_status(&cfg, run_length, samp_size);
        }

        {
            let run_length = RunLength::Count(10);
            run_no_status(&cfg, run_length, samp_size);
        }

        {
            let run_length = RunLength::Count(100);
            run_no_status(&cfg, run_length, samp_size);
        }

        {
            let run_length = RunLength::Count(1000);
            run_no_status(&cfg, run_length, samp_size);
        }

        {
            let run_length = RunLength::Count(10_000);
            run_no_status(&cfg, run_length, samp_size);
        }

        {
            let run_length = RunLength::Count(100_000);
            run_no_status(&cfg, run_length, samp_size);
        }

        {
            let run_length = RunLength::Count(1_000_000);
            run_no_status(&cfg, run_length, samp_size);
        }
    }

    //=== with_status

    {
        {
            let run_length = RunLength::Count(1);
            run_with_status(&cfg, run_length, samp_size);
        }

        {
            let run_length = RunLength::Count(10);
            run_no_status(&cfg, run_length, samp_size);
        }

        {
            let run_length = RunLength::Count(100);
            run_with_status(&cfg, run_length, samp_size);
        }

        {
            let run_length = RunLength::Count(1000);
            run_with_status(&cfg, run_length, samp_size);
        }

        {
            let run_length = RunLength::Count(10_000);
            run_with_status(&cfg, run_length, samp_size);
        }

        {
            let run_length = RunLength::Count(100_000);
            run_with_status(&cfg, run_length, samp_size);
        }

        {
            let run_length = RunLength::Count(1_000_000);
            run_with_status(&cfg, run_length, samp_size);
        }
    }
}
