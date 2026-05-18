mod support;

use bench_utils::BusyWork;
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
    let base_busy_work = BusyWork::new(base_latency);
    let base_effort = base_busy_work.effort();

    eprintln!("base_latency={base_latency:?}");
    eprintln!("base_effort={}", base_effort);

    let effort1 = (base_effort as f64 * target_ratio) as u32;
    let mut f1 = BusyWork::from_effort(effort1).fun();

    let effort2 = base_effort;
    let mut f2 = BusyWork::from_effort(effort2).fun();

    for i in 1..=nrepeats {
        let name1 = format!("f1={target_ratio}@novar[{i}/{nrepeats}]");
        let name2 = format!("f2=1@novar[{i}/{nrepeats}]");

        c.bench_function(&name1, |b| b.iter(&mut f1));
        c.bench_function(&name2, |b| b.iter(&mut f2));
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
