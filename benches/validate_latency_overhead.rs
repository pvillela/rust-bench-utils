use basic_stats::{dev_utils::ApproxEq, rel_approx_eq};
use bench_utils::{
    RunLength, bench_run_with_status, busy_work, calibrate_busy_work, get_bench_cfg,
};
use std::time::{Duration, Instant};

/// Compares latency outputs for `n` executions of a function `f` with `n/group_size` executions of `f` grouped `group_size` times.
fn run_bench(
    bench_time: Duration,
    target_latency: Duration,
    group_size: usize,
    epsilon: f64,
) -> (f64, f64) {
    let name = "Group of ".to_owned() + &group_size.to_string();
    let effort = calibrate_busy_work(target_latency);
    let solo_f = || busy_work(effort);
    let group_f = || {
        for _ in 0..group_size {
            solo_f();
        }
    };

    let reporting_unit = get_bench_cfg().reporting_unit();
    let target_median_solo = reporting_unit.latency_as_f64(target_latency);
    let target_median_group = target_median_solo * group_size as f64;
    let exec_count_group =
        (reporting_unit.latency_as_f64(bench_time) / target_median_group) as usize;
    // Guard against integer truncation to 0 when target_median is larger than
    // BENCH_TIME. Fall back to 1 execution so the benchmark still produces
    // meaningful data and the assertion doesn't fire on an empty sample.
    let exec_count_group = exec_count_group.max(1);
    let exec_count_solo = exec_count_group * group_size;

    println!("reporting_unit={reporting_unit:?}");
    println!();

    let out_solo = bench_run_with_status(solo_f, RunLength::Count(exec_count_solo), |_| {
        println!("running solo_f: {name}");
    });
    println!("{:?}", out_solo.summary());
    println!(
        "target_median_solo={target_median_solo}, out_solo.median()={}, rel_diff={}",
        out_solo.median(),
        target_median_solo.abs_rel_diff(out_solo.median(), epsilon)
    );
    println!();

    let out_group = bench_run_with_status(group_f, RunLength::Count(exec_count_group), |_| {
        println!("running group_f: {name}");
    });
    println!("{:?}", out_group.summary());
    println!(
        "target_median_group={target_median_group}, out_group.median()={}, rel_diff={}",
        out_group.median(),
        target_median_group.abs_rel_diff(out_group.median(), epsilon)
    );
    println!();

    println!(
        "Solo vs. grouped: group_size={group_size}, out_solo.median()*group_size={}, out_group.median()={}, rel_diff={}",
        out_solo.median() * group_size as f64,
        out_group.median(),
        target_median_group.abs_rel_diff(out_group.median(), epsilon)
    );
    println!();

    (out_solo.median(), out_group.median())
}

fn main() {
    const EPSILON: f64 = 0.1;

    let start = Instant::now();
    let cfg = get_bench_cfg();
    cfg.with_warmup_millis(500).set();

    let bench_time = Duration::from_millis(500);
    let target_latency = Duration::from_micros(100);

    let (solo_median_20, group_median_20) = run_bench(bench_time, target_latency, 20, EPSILON);
    let (solo_median_100, group_median_100) = run_bench(bench_time, target_latency, 100, EPSILON);

    println!("elapsed time: {} millis", start.elapsed().as_millis());

    rel_approx_eq!(solo_median_20 * 20., group_median_20, EPSILON);
    rel_approx_eq!(solo_median_100 * 100., group_median_100, EPSILON);
}
