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
) -> (f64, f64) {
    let name = "Group of ".to_owned() + &group_size.to_string();
    let solo_f = BusyWork::new(target_latency).fun();
    let group_f = || {
        for _ in 0..group_size {
            solo_f();
        }
    };

    let reporting_unit = cfg.reporting_unit();
    let target_median_solo = reporting_unit.latency_as_f64(target_latency);
    let target_median_group = target_median_solo * group_size as f64;
    let exec_count_group =
        (reporting_unit.latency_as_f64(bench_duration) / target_median_group) as usize;
    // Guard against integer truncation to 0 when target_median_group is larger than
    // bench_duration. Fall back to 1 execution so the benchmark still produces
    // meaningful data and the assertion doesn't fire on an empty sample.
    let exec_count_group = exec_count_group.max(1);
    let exec_count_solo = exec_count_group * group_size;

    println!("reporting_unit={reporting_unit:?}");
    println!();

    println!("running solo_f: {name}");
    let out_solo = bench_run_with_status_arg_cfg(cfg, &solo_f, RunLength::Count(exec_count_solo));
    println!("{:?}", out_solo.summary());
    println!(
        "target_median_solo={target_median_solo}, out_solo.median()={}, rel_diff={}",
        out_solo.median(),
        target_median_solo.abs_rel_diff(out_solo.median())
    );
    println!();

    println!("running group_f: {name}");
    let out_group = bench_run_with_status_arg_cfg(cfg, group_f, RunLength::Count(exec_count_group));
    println!("{:?}", out_group.summary());
    println!(
        "target_median_group={target_median_group}, out_group.median()={}, rel_diff={}",
        out_group.median(),
        target_median_group.abs_rel_diff(out_group.median())
    );
    println!();

    println!(
        "Solo vs. grouped: group_size={group_size}, out_solo.median()*group_size={}, out_group.median()={}, rel_diff={}",
        out_solo.median() * group_size as f64,
        out_group.median(),
        (out_solo.median() * group_size as f64).abs_rel_diff(out_group.median())
    );
    println!();

    (out_solo.median(), out_group.median())
}
