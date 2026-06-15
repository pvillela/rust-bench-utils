mod support;

use bench_utils::BusyWork;
use criterion::{Criterion, criterion_group, criterion_main};
use support::bench_basic_naive::{Args, get_args};

fn criterion_benchmark(c: &mut Criterion) {
    let args = get_args();
    eprintln!("args={args:?}");

    let Args {
        target_ratio: _target_ratio,
        latency_unit,
        base_median,
        nrepeats,
    } = args;

    let base_latency = latency_unit.latency_from_f64(base_median);

    eprintln!("base_latency={base_latency:?}");

    let effort = BusyWork::calibrate(base_latency);
    let mut f = BusyWork::new(effort).fun();

    for i in 1..=nrepeats {
        let name = format!("latency={base_latency:?}[{i}/{nrepeats}]");

        c.bench_function(&name, |b| b.iter(&mut f));
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
