use bench_utils::criterion::{CriterionOut, enable_measurement, set_measurement_type};
use criterion::{Criterion, criterion_group, criterion_main};
use std::{thread, time::Duration};

fn sleep_60_micros(c: &mut Criterion<CriterionOut>) {
    let out = enable_measurement(c);

    c.bench_function("sleep_60_micros", |b| {
        b.iter(|| thread::sleep(Duration::from_micros(60)))
    });

    let summary = out.borrow().summary();
    println!("{summary:?}")
}

fn sleep_60_millis(c: &mut Criterion<CriterionOut>) {
    let out = enable_measurement(c);

    c.bench_function("sleep_60_millis", |b| {
        b.iter(|| thread::sleep(Duration::from_millis(60)))
    });

    let summary = out.borrow().summary();
    println!("{summary:?}")
}

criterion_group! {
    name = crit_bench;
    config = set_measurement_type();
    targets = sleep_60_millis,sleep_60_micros
}

criterion_main!(crit_bench);
