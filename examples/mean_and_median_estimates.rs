//! Demonstrates demonstrates mean and median estimates for different target latencies, batch sizes, and
//! sample sizes with [`BusyWork`].
//!
//! ```
//! cargo run -r --example mean_and_median_estimates --features load,_test
//! ```

use bench_utils::{
    BenchCfg, LatencyUnit, RunLength, bench_run_arg_cfg,
    load::{BusyWork, Calibration},
    test_support::AbsRelDiffFpSecs,
};
use std::time::{Duration, Instant};

fn main() {
    let calibration = BusyWork::calibrate();

    {
        let target_latency = Duration::from_nanos(1);
        println!("\n*** target_latency={target_latency:?}");

        {
            let batch = Some(100_000);
            let samp_size = 100;
            run_and_display(target_latency, calibration, batch, samp_size);
        }
    }

    {
        let target_latency = Duration::from_nanos(10);
        println!("\n*** target_latency={target_latency:?}");

        {
            let batch = Some(10_000);
            let samp_size = 100;
            run_and_display(target_latency, calibration, batch, samp_size);
        }
    }

    {
        let target_latency = Duration::from_nanos(100);
        println!("\n*** target_latency={target_latency:?}");

        {
            let batch = Some(1_000);
            let samp_size = 100;
            run_and_display(target_latency, calibration, batch, samp_size);
        }
    }

    {
        let target_latency = Duration::from_micros(1);
        println!("\n*** target_latency={target_latency:?}");

        {
            let batch = Some(100);
            let samp_size = 100;
            run_and_display(target_latency, calibration, batch, samp_size);
        }
    }

    {
        let target_latency = Duration::from_micros(10);
        println!("\n*** target_latency={target_latency:?}");

        {
            let batch = Some(10);
            let samp_size = 100;
            run_and_display(target_latency, calibration, batch, samp_size);
        }
    }

    {
        let target_latency = Duration::from_micros(100);
        println!("\n*** target_latency={target_latency:?}");

        {
            let batch = None;
            let samp_size = 1000;
            run_and_display(target_latency, calibration, batch, samp_size);
        }

        {
            let batch = Some(10);
            let samp_size = 100;
            run_and_display(target_latency, calibration, batch, samp_size);
        }
    }

    {
        let target_latency = Duration::from_millis(1);
        println!("\n*** target_latency={target_latency:?}");

        {
            let batch = None;
            let samp_size = 500;
            run_and_display(target_latency, calibration, batch, samp_size);
        }

        {
            let batch = Some(10);
            let samp_size = 50;
            run_and_display(target_latency, calibration, batch, samp_size);
        }
    }

    {
        let target_latency = Duration::from_millis(10);
        println!("\n*** target_latency={target_latency:?}");

        {
            let batch = None;
            let samp_size = 200;
            run_and_display(target_latency, calibration, batch, samp_size);
        }

        {
            let batch = Some(10);
            let samp_size = 20;
            run_and_display(target_latency, calibration, batch, samp_size);
        }
    }
}

fn run_and_display(
    target_latency: Duration,
    calibration: Calibration,
    batch: Option<usize>,
    samp_size: usize,
) {
    let start = Instant::now();

    let bsz = batch.unwrap_or(1).max(1);
    let run_length = RunLength::Count(bsz * samp_size);
    let (effort, calibr_ltncy) = calibration.effort_for_latency(target_latency.into());
    let f = BusyWork::fun(effort);
    let cfg = BenchCfg::default()
        .with_recording_unit(LatencyUnit::sub_sec(12))
        .with_warmup_millis(100);
    let out = bench_run_arg_cfg(&cfg, f, run_length, batch);

    let mean = out.mean();
    let stdev = out.stdev();
    let cv = stdev / mean;
    let stdev_r = out.stdev_r();
    let rc_q_ns = out.rousseeuw_croux_q_ns();
    let mean_ln_r = out.mean_ln_r();
    let stdev_ln_r = out.stdev_ln_r();
    let rc_q_ls = out.rousseeuw_croux_q_ls();

    let median_r = out.median_r();
    let median_log_space = out.median_log_space_estimator();
    let median_rmom = out.median_rmom_estimator();
    let median_rob = out.median_rob();

    let mean_rel_diff = calibr_ltncy.abs_rel_diff_fpsecs(mean);
    let median_r_rel_diff = calibr_ltncy.abs_rel_diff_fpsecs(median_r);
    let median_rob_rel_diff = calibr_ltncy.abs_rel_diff_fpsecs(median_rob);

    let elapsed = start.elapsed();

    println!();
    println!(
        "target_latency={target_latency:?}, calibr_ltncy={calibr_ltncy:?}, effort={effort}, batch={batch:?}, samp_size={samp_size}"
    );
    println!(
        "mean={mean:?}, stdev={stdev:?}, CV={cv:?}, stdev_r={stdev_r:?}, rc_q_ns={rc_q_ns:?}, mean_ln_r={mean_ln_r:.3e}, stdev_ln_r={stdev_ln_r:.3e}, rc_q_ls={rc_q_ls:.3e}"
    );
    println!(
        "median_r={median_r:?}, median_log_space={median_log_space:?}, median_rmom={median_rmom:?}, median_rob={median_rob:?}"
    );
    println!(
        "mean_rel_diff={mean_rel_diff:?}, median_r_rel_diff={median_r_rel_diff:?}, median_rob_rel_diff={median_rob_rel_diff:?}"
    );
    println!("{:?}", out.summary());
    println!("elapsed_time={elapsed:?}");
}
