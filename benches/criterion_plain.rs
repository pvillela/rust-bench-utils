use criterion::{Criterion, criterion_group, criterion_main};
use std::{thread, time::Duration};

fn sleep_60_micros(c: &mut Criterion) {
    c.bench_function("sleep_60_micros", |b| {
        b.iter(|| thread::sleep(Duration::from_micros(60)))
    });
}

fn sleep_60_millis(c: &mut Criterion) {
    c.bench_function("sleep_60_millis", |b| {
        b.iter(|| thread::sleep(Duration::from_millis(60)))
    });
}

criterion_group! {
    name = crit_bench;
    config = Criterion::default();
    targets = sleep_60_millis,sleep_60_micros
}

criterion_main!(crit_bench);
