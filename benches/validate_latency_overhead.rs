use basic_stats::{dev_utils::ApproxEq, rel_approx_eq};
use bench_utils::{
    RunLength, bench_run_with_status, busy_work, calibrate_busy_work, get_bench_cfg,
};
use std::time::Duration;

const EPSILON: f64 = 0.1;
const BENCH_TIME: Duration = Duration::from_secs(3);

/// Compares latency outputs for `n` executions of a function `f` with `n/group_size` executions of `f` grouped `group_size` times.
fn run_bench(name: &'static str, target_latency: Duration, group_size: usize, check: bool) {
    let effort = calibrate_busy_work(target_latency);
    let solo_f = || busy_work(effort);
    let group_f = || {
        for i in 0..group_size {
            solo_f();
        }
    };

    let reporting_unit = get_bench_cfg().reporting_unit();
    let target_median_solo = reporting_unit.latency_as_f64(target_latency);
    let target_median_group = target_median_solo * group_size as f64;
    let exec_count_group =
        (reporting_unit.latency_as_f64(BENCH_TIME) / target_median_group) as usize;
    // Guard against integer truncation to 0 when target_median is larger than
    // BENCH_TIME. Fall back to 1 execution so the benchmark still produces
    // meaningful data and the assertion doesn't fire on an empty sample.
    let exec_count_group = exec_count_group.max(1);
    let exec_count_solo = exec_count_group * group_size;

    let out_solo = bench_run_with_status(solo_f, RunLength::Count(exec_count_solo), |_| {
        println!("running solo_f: {name}");
    });
    println!(
        "target_median={target_median_solo}, out.median()={}, rel_diff={}",
        out_solo.median(),
        target_median_solo.abs_rel_diff(out_solo.median(), 0.)
    );
    println!("{:?}", out_solo.summary());
    println!();

    let out_group = bench_run_with_status(group_f, RunLength::Count(exec_count_group), |_| {
        println!("running group_f: {name}");
    });
    println!(
        "target_median={target_median_group}, out.median()={}, rel_diff={}",
        out_group.median(),
        target_median_group.abs_rel_diff(out_group.median(), 0.)
    );
    println!("{:?}", out_group.summary());
    println!();

    if check {
        rel_approx_eq!(target_median_solo, out_solo.median(), EPSILON);
        rel_approx_eq!(target_median_group, out_group.median(), EPSILON);
        rel_approx_eq!(
            out_solo.median() * group_size as f64,
            out_group.median(),
            EPSILON
        );
    }
}

fn main() {
    let target_latency = Duration::from_millis(60);

    run_bench("Group of 20", target_latency, 20, true);
    run_bench("Group of 100", target_latency, 100, true);
}
