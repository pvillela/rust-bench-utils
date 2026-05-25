//! Validates that the latency-measurement overhead per function execution is acceptable.
//! Gated by feature **"_bench"**.
//!
//! The function [`validate_latency_overhead`]
//! compares solo vs. grouped execution latencies to detect overhead from the measurement harness.

use crate::{BenchCfg, BusyWork, RunLength, bench_run_with_status_arg_cfg};
use basic_stats::dev_utils::ApproxEq;
use std::time::Duration;

/// Compares latency outputs for `n` executions of a function `f` with `n/group_size` executions of `f` grouped `group_size` times.
pub fn validate_latency_overhead(
    cfg: &BenchCfg,
    bench_duration: Duration,
    target_latency: Duration,
    group_size: usize,
) -> (Duration, Duration) {
    let name = "Group of ".to_owned() + &group_size.to_string();
    let solo_f = BusyWork::new(target_latency).fun();
    let group_f = || {
        for _ in 0..group_size {
            solo_f();
        }
    };

    let recording_unit = cfg.recording_unit();
    let target_median_solo = recording_unit.latency_as_f64(target_latency);
    let target_median_group = target_median_solo * group_size as f64;
    let exec_count_group =
        (recording_unit.latency_as_f64(bench_duration) / target_median_group) as usize;
    // Guard against integer truncation to 0 when target_median_group is larger than
    // bench_duration. Fall back to 1 execution so the benchmark still produces
    // meaningful data and the assertion doesn't fire on an empty sample.
    let exec_count_group = exec_count_group.max(1);
    let exec_count_solo = exec_count_group * group_size;

    println!("recording_unit={recording_unit:?}");
    println!();

    println!("running solo_f: {name}");
    let out_solo = bench_run_with_status_arg_cfg(cfg, &solo_f, RunLength::Count(exec_count_solo));
    println!("{:?}", out_solo.summary());
    let solo_median_ns = out_solo.median().as_nanos() as f64;
    println!(
        "target_median_solo={target_median_solo}, out_solo.median()={solo_median_ns}, rel_diff={}",
        target_median_solo.abs_rel_diff(solo_median_ns)
    );
    println!();

    println!("running group_f: {name}");
    let out_group = bench_run_with_status_arg_cfg(cfg, group_f, RunLength::Count(exec_count_group));
    println!("{:?}", out_group.summary());
    let group_median_ns = out_group.median().as_nanos() as f64;
    println!(
        "target_median_group={target_median_group}, out_group.median()={group_median_ns}, rel_diff={}",
        target_median_group.abs_rel_diff(group_median_ns)
    );
    println!();

    println!(
        "Solo vs. grouped: group_size={}, out_solo.median()*group_size={}, out_group.median()={}, rel_diff={}",
        group_size,
        solo_median_ns * group_size as f64,
        group_median_ns,
        (solo_median_ns * group_size as f64).abs_rel_diff(group_median_ns)
    );
    println!();

    (out_solo.median(), out_group.median())
}
