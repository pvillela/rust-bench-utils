use bench_utils::{
    BenchCfg, RunLength, bench_run_with_status_arg_cfg, bench_run_with_status_arg_cfg_b, latency,
    load::BusyWork, rel_approx_eq_dur, test_support::AbsRelDiffDur,
};
use std::time::Duration;

fn run_bench_with_status(
    warmup_millis: u64,
    status_millis: u64,
    bench_time: Duration,
    target_latency: Duration,
    epsilon: f64,
    batch: Option<usize>,
) {
    let name = format!(
        "target_latency={target_latency:?}, warmup={warmup_millis}, bench_time={bench_time:?}"
    );
    let exec_count = (bench_time.as_secs_f64() / target_latency.as_secs_f64()).round() as usize;

    println!("validate_bench_run: {name}");

    let effort = BusyWork::calibrate(target_latency);
    let mut f = BusyWork::fun(effort);

    let cfg = BenchCfg::default()
        .with_warmup_millis(warmup_millis)
        .with_status_millis(status_millis);
    let out = match batch {
        None => bench_run_with_status_arg_cfg(&cfg, &mut f, RunLength::Count(exec_count)),
        Some(batch) => {
            bench_run_with_status_arg_cfg_b(&cfg, &mut f, RunLength::Count(exec_count), batch)
        }
    };
    println!();

    let out_mean = out.mean();
    println!(
        "target_mean={target_latency:?}, out.mean()={out_mean:?}, rel_diff={}",
        target_latency.abs_rel_diff_dur(out_mean.as_duration())
    );

    let raw_latency = latency(|| {
        for _ in 0..exec_count {
            f();
        }
    });
    let raw_mean = raw_latency / exec_count as u32;
    println!(
        "target_mean={target_latency:?}, raw_mean()={raw_mean:?}, rel_diff={}",
        target_latency.abs_rel_diff_dur(raw_mean)
    );

    println!(
        "raw_mean={out_mean:?}, out_mean()={raw_mean:?}, rel_diff={}",
        raw_mean.abs_rel_diff_dur(out_mean.as_duration())
    );

    rel_approx_eq_dur!(raw_mean, out_mean.as_duration(), epsilon);
}

fn main() {
    {
        const EPSILON: f64 = 0.01; // 0.05
        run_bench_with_status(
            1000,
            100,
            Duration::from_millis(2000),
            Duration::from_millis(10),
            EPSILON,
            None,
        );
    }

    {
        const EPSILON: f64 = 0.01; // 0.05
        run_bench_with_status(
            100,
            10,
            Duration::from_millis(200),
            Duration::from_micros(50),
            EPSILON,
            Some(20),
        );
    }
}
