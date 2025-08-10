mod support;

use bench_utils::{busy_work, calibrate_busy_work};
use criterion::{Criterion, criterion_group, criterion_main};
use support::bench_basic_naive::{Args, get_args};

fn criterion_benchmark(c: &mut Criterion) {
    let args = get_args();
    eprintln!("args={args:?}");

    let Args {
        target_ratio,
        latency_unit,
        base_median,
        nrepeats,
    } = args;

    let base_latency = latency_unit.latency_from_f64(base_median);
    let base_effort = calibrate_busy_work(base_latency);

    eprintln!("base_latency={base_latency:?}");
    eprintln!("base_effort={}", base_effort);

    let effort1 = (base_effort as f64 * target_ratio) as u32;
    let f1 = || busy_work(effort1);

    let effort2 = base_effort;
    let f2 = || busy_work(effort2);

    for i in 1..=nrepeats {
        let name1 = format!("f1={target_ratio}@novar[{i}/{nrepeats}]");
        let name2 = format!("f2={target_ratio}@novar[{i}/{nrepeats}]");

        c.bench_function(&name1, |b| b.iter(f1));
        c.bench_function(&name2, |b| b.iter(f2));
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
