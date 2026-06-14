//! Validates that the latency-measurement overhead per function execution is acceptable.
//! Gated by feature **"_bench"**.
//!
//! The function [`validate_latency_overhead`]
//! compares solo vs. grouped execution latencies to detect overhead from the measurement harness.

use crate::{
    BenchCfg, BusyWork, RunLength, bench_run_with_status_arg_cfg, test_support::AbsRelDiffDur,
};
use std::time::Duration;

/// Compares latency outputs for `n` executions of a function `f` with `n/grouping` executions of `f` grouped `grouping` times.
///
/// # Panics
///
/// Panics if any of the following conditions is true:
/// - `target_latency` is zero
/// - `grouping` is zero.
/// the estimated execution count.
pub fn validate_latency_overhead(
    cfg: &BenchCfg,
    bench_duration: Duration,
    target_latency: Duration,
    grouping: usize,
) -> (Duration, Duration) {
    assert!(
        target_latency > Duration::ZERO && grouping > 0,
        "`target_latency` and `grouping` must both be positive"
    );
    let name = "Group of ".to_owned() + &grouping.to_string();
    let solo_f = BusyWork::new(target_latency).fun();
    let group_f = || {
        for _ in 0..grouping {
            solo_f();
        }
    };

    let target_group_latency = target_latency * grouping as u32;
    let exec_count_group =
        (bench_duration.as_secs_f64() / target_group_latency.as_secs_f64()).round() as usize;
    let exec_count_solo = exec_count_group * grouping;

    println!("running solo_f: {name}");
    let out_solo = bench_run_with_status_arg_cfg(cfg, &solo_f, RunLength::Count(exec_count_solo));
    println!("{:?}", out_solo.summary());
    let solo_median = out_solo.median();
    println!(
        "target_median_solo={target_latency:?}, out_solo.median()={solo_median:?}, rel_diff={}",
        target_latency.abs_rel_diff(solo_median)
    );
    println!();

    println!("running group_f: {name}");
    let out_group = bench_run_with_status_arg_cfg(cfg, group_f, RunLength::Count(exec_count_group));
    println!("{:?}", out_group.summary());
    let group_median = out_group.median();
    println!(
        "target_median_group={:?}, out_group.median()={group_median:?}, rel_diff={}",
        target_group_latency,
        target_group_latency.abs_rel_diff(group_median)
    );
    println!();

    println!(
        "Solo vs. grouped: grouping={grouping}, out_solo.median()*grouping={:?}, out_group.median()={group_median:?}, rel_diff={}",
        solo_median * grouping as u32,
        (solo_median * grouping as u32).abs_rel_diff(group_median)
    );
    println!();

    (solo_median, group_median)
}
