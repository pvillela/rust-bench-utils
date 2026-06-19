//! Validates that the latency-measurement overhead per function execution is acceptable.
//! Gated by feature **"_bench"**.
//!
//! The function [`validate_latency_overhead`]
//! compares solo vs. grouped execution latencies to detect overhead from the measurement harness.

use crate::{
    BenchCfg, FpSeconds, RunLength, bench_run_with_status_arg_cfg, load::BusyWork,
    test_support::AbsRelDiffFpSecs,
};
use std::time::Duration;

/// Compares latency outputs for `n` executions of a function `f` with `n/batch` executions of `f` grouped `batch` times.
///
/// # Panics
///
/// Panics if any of the following conditions is true:
/// - `target_latency` is zero
/// - `batch` is zero.
/// the estimated execution count.
pub fn validate_latency_overhead(
    cfg: &BenchCfg,
    bench_duration: Duration,
    target_latency: Duration,
    batch: usize,
) -> (FpSeconds, FpSeconds) {
    assert!(
        target_latency > Duration::ZERO && batch > 0,
        "`target_latency` and `batch` must both be positive"
    );
    let name = "Group of ".to_owned() + &batch.to_string();
    let effort = BusyWork::calibrate(target_latency);
    let mut solo_f = BusyWork::fun(effort);
    let mut solo_fc = solo_f.clone();
    let group_f = || {
        for _ in 0..batch {
            solo_fc();
        }
    };

    let target_group_latency = target_latency * batch as u32;
    let exec_count_group =
        (bench_duration.as_secs_f64() / target_group_latency.as_secs_f64()).round() as usize;
    let exec_count_solo = exec_count_group * batch;

    println!("running solo_f: {name}");
    let out_solo =
        bench_run_with_status_arg_cfg(cfg, &mut solo_f, RunLength::Count(exec_count_solo));
    println!("{:?}", out_solo.summary());
    let solo_median = out_solo.median();
    println!(
        "target_median_solo={target_latency:?}, out_solo.median()={solo_median:?}, rel_diff={}",
        FpSeconds::from_duration(target_latency).abs_rel_diff(solo_median)
    );
    println!();

    println!("running group_f: {name}");
    let out_group = bench_run_with_status_arg_cfg(cfg, group_f, RunLength::Count(exec_count_group));
    println!("{:?}", out_group.summary());
    let group_median = out_group.median();
    println!(
        "target_median_group={:?}, out_group.median()={group_median:?}, rel_diff={}",
        target_group_latency,
        FpSeconds::from_duration(target_group_latency).abs_rel_diff(group_median)
    );
    println!();

    println!(
        "Solo vs. grouped: batch={batch}, out_solo.median()*batch={:?}, out_group.median()={group_median:?}, rel_diff={}",
        solo_median * batch,
        (solo_median * batch).abs_rel_diff(group_median)
    );
    println!();

    (solo_median, group_median)
}
