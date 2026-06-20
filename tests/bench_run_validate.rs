#![cfg(feature = "_bench")]

//! cargo test -r --test bench_run_validate --all-features -- with_batch no_batch --nocapture --test-threads=1

use basic_stats::{dev_utils::ApproxEq, rel_approx_eq};
use bench_utils::{BenchCfg, LatencyUnit, RunLength, latency, load::BusyWork, multi::BenchOut};
use std::time::{Duration, Instant};

trait FnsSrc<const K: usize> {
    fn f1(&self) -> impl FnMut();
    fn f2(&self) -> impl FnMut();
}

fn run<const K: usize, R, Src>(
    rec_unit: LatencyUnit,
    runner: R,
    fsrc: Src,
    base_warmup_millis: u64,
    base_status_millis: u64,
    base_bench_time: Duration,
    base_target_latency: Duration,
    batch: Option<usize>,
    epsilon: f64,
) where
    Src: FnsSrc<K>,
    R: FnOnce(Duration, &BenchCfg, RunLength, Option<usize>) -> BenchOut<K>,
{
    _ = env_logger::try_init();

    assert!(1 <= K && K <= 2, "K={K} must be 1 or 2");
    let start = Instant::now();

    let mut f1 = fsrc.f1();
    let mut f2 = fsrc.f2();

    let warmup_millis = base_warmup_millis * K as u64;
    let bench_time = base_bench_time * K as u32;
    let status_millis = base_status_millis * K as u64;

    println!(
        "validate_bench_run: K={K}, rec_unit={rec_unit:?}, base_target_latency={base_target_latency:?}, warmup={warmup_millis}, bench_time={bench_time:?}, batch={batch:?}"
    );

    let cfg = BenchCfg::default()
        .with_warmup_millis(warmup_millis)
        .with_status_millis(status_millis)
        .with_recording_unit(rec_unit);
    let out = runner(
        base_target_latency,
        &cfg,
        RunLength::Time(bench_time),
        batch,
    );
    let count = out.n();

    let mut raw_latencies = Vec::<Duration>::new();
    if K >= 1 {
        raw_latencies.push(latency(|| {
            for _ in 0..count {
                f1();
            }
        }));
    }
    if K == 2 {
        raw_latencies.push(latency(|| {
            for _ in 0..count {
                f2();
            }
        }));
    }

    let raw_means = raw_latencies
        .iter()
        .map(|lat| lat.as_secs_f64() / count as f64)
        .collect::<Vec<_>>();

    let base_target_latency_secs_f64 = base_target_latency.as_secs_f64();

    for i in 0..K {
        let out_mean_secs_f64 = out[i].mean().as_f64();
        let raw_mean = raw_means[i];

        println!(
            "base_target_latency_secs_f64={base_target_latency_secs_f64:?}, out[{i}].mean().as_f64={out_mean_secs_f64:?}, rel_diff={}",
            base_target_latency_secs_f64.abs_rel_diff(out_mean_secs_f64)
        );

        println!(
            "base_target_latency_secs_f64={base_target_latency_secs_f64:?}, raw_mean[{i}]={raw_mean:?}, rel_diff={}",
            base_target_latency_secs_f64.abs_rel_diff(raw_mean)
        );

        println!(
            "raw_mean[{i}]={raw_mean:?}, out[{i}].mean_as_f64()={out_mean_secs_f64:?}, rel_diff={}",
            raw_mean.abs_rel_diff(out_mean_secs_f64)
        );

        println!("out[{i}].hist().mean()={:?}", out[i].hist().mean());

        println!("out[{i}].summary()={:?}", out[i].summary());
    }

    let aggregate_raw_mean = raw_latencies.iter().sum::<Duration>().as_secs_f64() / count as f64;
    let aggregate_out_mean = out.iter().map(|x| x.mean().0).sum();

    if K >= 2 {
        println!(
            "aggregate_raw_mean={aggregate_raw_mean:?}, aggregate_out_mean={aggregate_out_mean:?}, rel_diff={}",
            aggregate_raw_mean.abs_rel_diff(aggregate_out_mean)
        );
    }

    println!("test total elapsed time = {:?}", start.elapsed());

    // Assertions
    {
        if K >= 2 {
            rel_approx_eq!(
                aggregate_raw_mean,
                aggregate_out_mean,
                epsilon / (K as f64).sqrt()
            );
        }

        for i in 0..K {
            rel_approx_eq!(raw_means[i], out[i].mean().0, epsilon);
        }
    }
}

const BASE_WARMUP_MILLIS: u64 = 100;
const BASE_STATUS_MILLIS: u64 = 10;
const BASE_BENCH_TIME: Duration = Duration::from_millis(100);
const DEFAULT_REC_UNIT: LatencyUnit = LatencyUnit::Nano;
const DEFAULT_N_BATCHES: usize = 50;

struct Fns1 {
    effort: u32,
}

impl Fns1 {
    fn new(base_target_latency: Duration) -> Self {
        Self {
            effort: BusyWork::calibrate(base_target_latency),
        }
    }
}

impl FnsSrc<1> for Fns1 {
    fn f1(&self) -> impl FnMut() {
        BusyWork::fun(self.effort)
    }

    fn f2(&self) -> impl FnMut() {
        || ()
    }
}

struct Fns2 {
    effort0: u32,
    effort_delta: u32,
}

impl Fns2 {
    fn new(base_target_latency: Duration) -> Self {
        let effort0 = BusyWork::calibrate(base_target_latency);
        Self {
            effort0,
            effort_delta: effort0 / 10,
        }
    }
}

impl FnsSrc<2> for Fns2 {
    fn f1(&self) -> impl FnMut() {
        BusyWork::fun(self.effort0)
    }

    fn f2(&self) -> impl FnMut() {
        let mut f2a = BusyWork::fun(self.effort0 - self.effort_delta);
        let mut f2b = BusyWork::fun(self.effort_delta);
        move || {
            f2a();
            f2b()
        }
    }
}

/// Calculates a batch size that yields `n` batches given the `target_latency` and `bench_time`.
fn batch_n(n: usize, target_latency: Duration, bench_time: Duration) -> Option<usize> {
    Some((bench_time.as_nanos() / target_latency.as_nanos()) as usize / n)
}

// cargo test -r --test bench_run_validate --all-features -- no_status1 --nocapture --test-threads=1
mod no_status1 {
    use super::*;
    use bench_utils::bench_run_arg_cfg_o;

    fn runner(
        base_target_latency: Duration,
        cfg: &BenchCfg,
        run_length: RunLength,
        batch: Option<usize>,
    ) -> BenchOut<1> {
        let fns = Fns1::new(base_target_latency);
        bench_run_arg_cfg_o(cfg, fns.f1(), run_length, batch).into()
    }

    fn run_bench(
        rec_unit: LatencyUnit,
        base_warmup_millis: u64,
        base_bench_time: Duration,
        base_target_latency: Duration,
        batch: Option<usize>,
        epsilon: f64,
    ) {
        run(
            rec_unit,
            runner,
            Fns1::new(base_target_latency),
            base_warmup_millis,
            0,
            base_bench_time,
            base_target_latency,
            batch,
            epsilon,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_nanos_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_1b() {
        const EPSILON: f64 = 0.02;
        let rec_unit = LatencyUnit::SubSec(12);
        let target_latency = Duration::from_nanos(1);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_nanos_10b --nocapture --test-threads=1
    #[test]
    fn test_nanos_10b() {
        const EPSILON: f64 = 0.10;
        let rec_unit = LatencyUnit::SubSec(11);
        let target_latency = Duration::from_nanos(10);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_nanos_50 --nocapture --test-threads=1
    #[test]
    fn test_nanos_50() {
        const EPSILON: f64 = 0.20;
        let rec_unit = LatencyUnit::SubSec(11);
        let target_latency = Duration::from_nanos(50);
        let batch = None;
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_nanos_50b --nocapture --test-threads=1
    #[test]
    fn test_nanos_50b() {
        const EPSILON: f64 = 0.05;
        let rec_unit = LatencyUnit::SubSec(11);
        let target_latency = Duration::from_nanos(50);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_micros_1 --nocapture --test-threads=1
    #[test]
    fn test_micros_1() {
        const EPSILON: f64 = 0.03;
        let target_latency = Duration::from_micros(1);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_micros_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_1b() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_micros(1);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_micros_50() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_micros(50);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_micros_50b --nocapture --test-threads=1
    #[test]
    fn test_micros_50b() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_micros(50);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_1() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(1);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_millis_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_millis_1b() {
        const EPSILON: f64 = 0.01;
        const BASE_WARMUP_MILLIS: u64 = 500;
        const BASE_BENCH_TIME: Duration = Duration::from_millis(500);

        let target_latency = Duration::from_millis(1);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_10() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(10);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_millis_10b --nocapture --test-threads=1
    #[test]
    fn test_millis_10b() {
        const EPSILON: f64 = 0.02;
        const BASE_WARMUP_MILLIS: u64 = 500;
        const BASE_BENCH_TIME: Duration = Duration::from_millis(500);

        let target_latency = Duration::from_millis(10);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }
}

// cargo test -r --test bench_run_validate --all-features -- with_status1 --nocapture --test-threads=1
mod with_status1 {
    use super::*;
    use bench_utils::bench_run_with_status_arg_cfg_o;

    fn runner(
        base_target_latency: Duration,
        cfg: &BenchCfg,
        run_length: RunLength,
        batch: Option<usize>,
    ) -> BenchOut<1> {
        let fns = Fns1::new(base_target_latency);
        bench_run_with_status_arg_cfg_o(cfg, fns.f1(), run_length, batch).into()
    }

    fn run_bench(
        rec_unit: LatencyUnit,
        base_warmup_millis: u64,
        base_status_millis: u64,
        base_bench_time: Duration,
        base_target_latency: Duration,
        batch: Option<usize>,
        epsilon: f64,
    ) {
        run(
            rec_unit,
            runner,
            Fns1::new(base_target_latency),
            base_warmup_millis,
            base_status_millis,
            base_bench_time,
            base_target_latency,
            batch,
            epsilon,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_nanos_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_1b() {
        const EPSILON: f64 = 0.02;
        let rec_unit = LatencyUnit::SubSec(12);
        let target_latency = Duration::from_nanos(1);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_nanos_10b --nocapture --test-threads=1
    #[test]
    fn test_nanos_10b() {
        const EPSILON: f64 = 0.10;
        let rec_unit = LatencyUnit::SubSec(11);
        let target_latency = Duration::from_nanos(10);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_nanos_50 --nocapture --test-threads=1
    #[test]
    fn test_nanos_50() {
        const EPSILON: f64 = 0.20;
        let rec_unit = LatencyUnit::SubSec(11);
        let target_latency = Duration::from_nanos(50);
        let batch = None;
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_nanos_50b --nocapture --test-threads=1
    #[test]
    fn test_nanos_50b() {
        const EPSILON: f64 = 0.05;
        let rec_unit = LatencyUnit::SubSec(11);
        let target_latency = Duration::from_nanos(50);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_micros_1 --nocapture --test-threads=1
    #[test]
    fn test_micros_1() {
        const EPSILON: f64 = 0.03;
        let target_latency = Duration::from_micros(1);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_micros_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_1b() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_micros(1);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_micros_50() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_micros(50);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_micros_50b --nocapture --test-threads=1
    #[test]
    fn test_micros_50b() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_micros(50);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_1() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(1);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_millis_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_millis_1b() {
        const EPSILON: f64 = 0.01;
        const BASE_WARMUP_MILLIS: u64 = 500;
        const BASE_BENCH_TIME: Duration = Duration::from_millis(500);

        let target_latency = Duration::from_millis(1);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_10() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(10);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_millis_10b --nocapture --test-threads=1
    #[test]
    fn test_millis_10b() {
        const EPSILON: f64 = 0.02;
        const BASE_WARMUP_MILLIS: u64 = 500;
        const BASE_BENCH_TIME: Duration = Duration::from_millis(500);

        let target_latency = Duration::from_millis(10);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }
}

// cargo test -r --test bench_run_validate --all-features -- no_status2 --nocapture --test-threads=1
mod no_status2 {
    use super::*;
    use bench_utils::duo::bench_run_arg_cfg_o;

    fn runner(
        base_target_latency: Duration,
        cfg: &BenchCfg,
        run_length: RunLength,
        batch: Option<usize>,
    ) -> BenchOut<2> {
        let fns = Fns2::new(base_target_latency);
        bench_run_arg_cfg_o(cfg, fns.f1(), fns.f2(), run_length, batch)
    }

    fn run_bench(
        rec_unit: LatencyUnit,
        base_warmup_millis: u64,
        base_bench_time: Duration,
        base_target_latency: Duration,
        batch: Option<usize>,
        epsilon: f64,
    ) {
        run(
            rec_unit,
            runner,
            Fns2::new(base_target_latency),
            base_warmup_millis,
            0,
            base_bench_time,
            base_target_latency,
            batch,
            epsilon,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_nanos_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_1b() {
        const EPSILON: f64 = 0.02;
        let rec_unit = LatencyUnit::SubSec(12);
        let target_latency = Duration::from_nanos(1);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_nanos_10b --nocapture --test-threads=1
    #[test]
    fn test_nanos_10b() {
        const EPSILON: f64 = 0.10;
        let rec_unit = LatencyUnit::SubSec(11);
        let target_latency = Duration::from_nanos(10);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_nanos_50 --nocapture --test-threads=1
    #[test]
    fn test_nanos_50() {
        const EPSILON: f64 = 0.20;
        let rec_unit = LatencyUnit::SubSec(11);
        let target_latency = Duration::from_nanos(50);
        let batch = None;
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_nanos_50b --nocapture --test-threads=1
    #[test]
    fn test_nanos_50b() {
        const EPSILON: f64 = 0.05;
        let rec_unit = LatencyUnit::SubSec(11);
        let target_latency = Duration::from_nanos(50);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_micros_1 --nocapture --test-threads=1
    #[test]
    fn test_micros_1() {
        const EPSILON: f64 = 0.03;
        let target_latency = Duration::from_micros(1);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_micros_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_1b() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_micros(1);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_micros_50() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_micros(50);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_micros_50b --nocapture --test-threads=1
    #[test]
    fn test_micros_50b() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_micros(50);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_1() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(1);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_millis_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_millis_1b() {
        const EPSILON: f64 = 0.01;
        const BASE_WARMUP_MILLIS: u64 = 500;
        const BASE_BENCH_TIME: Duration = Duration::from_millis(500);

        let target_latency = Duration::from_millis(1);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_10() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(10);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_millis_10b --nocapture --test-threads=1
    #[test]
    fn test_millis_10b() {
        const EPSILON: f64 = 0.02;
        const BASE_WARMUP_MILLIS: u64 = 500;
        const BASE_BENCH_TIME: Duration = Duration::from_millis(500);

        let target_latency = Duration::from_millis(10);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }
}

// cargo test -r --test bench_run_validate --all-features -- with_status2 --nocapture --test-threads=1
mod with_status2 {
    use super::*;
    use bench_utils::duo::bench_run_with_status_arg_cfg_o;

    fn runner(
        base_target_latency: Duration,
        cfg: &BenchCfg,
        run_length: RunLength,
        batch: Option<usize>,
    ) -> BenchOut<2> {
        let fns = Fns2::new(base_target_latency);
        bench_run_with_status_arg_cfg_o(cfg, fns.f1(), fns.f2(), run_length, batch).into()
    }

    fn run_bench(
        rec_unit: LatencyUnit,
        base_warmup_millis: u64,
        base_status_millis: u64,
        base_bench_time: Duration,
        base_target_latency: Duration,
        batch: Option<usize>,
        epsilon: f64,
    ) {
        run(
            rec_unit,
            runner,
            Fns2::new(base_target_latency),
            base_warmup_millis,
            base_status_millis,
            base_bench_time,
            base_target_latency,
            batch,
            epsilon,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_nanos_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_1b() {
        const EPSILON: f64 = 0.02;
        let rec_unit = LatencyUnit::SubSec(12);
        let target_latency = Duration::from_nanos(1);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_nanos_10b --nocapture --test-threads=1
    #[test]
    fn test_nanos_10b() {
        const EPSILON: f64 = 0.10;
        let rec_unit = LatencyUnit::SubSec(11);
        let target_latency = Duration::from_nanos(10);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_nanos_50 --nocapture --test-threads=1
    #[test]
    fn test_nanos_50() {
        const EPSILON: f64 = 0.20;
        let rec_unit = LatencyUnit::SubSec(11);
        let target_latency = Duration::from_nanos(50);
        let batch = None;
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_nanos_50b --nocapture --test-threads=1
    #[test]
    fn test_nanos_50b() {
        const EPSILON: f64 = 0.05;
        let rec_unit = LatencyUnit::SubSec(11);
        let target_latency = Duration::from_nanos(50);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_micros_1 --nocapture --test-threads=1
    #[test]
    fn test_micros_1() {
        const EPSILON: f64 = 0.03;
        let target_latency = Duration::from_micros(1);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_micros_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_1b() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_micros(1);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_micros_50() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_micros(50);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_micros_50b --nocapture --test-threads=1
    #[test]
    fn test_micros_50b() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_micros(50);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_1() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(1);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_millis_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_millis_1b() {
        const EPSILON: f64 = 0.01;
        const BASE_WARMUP_MILLIS: u64 = 500;
        const BASE_BENCH_TIME: Duration = Duration::from_millis(500);

        let target_latency = Duration::from_millis(1);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_10() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(10);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_millis_10b --nocapture --test-threads=1
    #[test]
    fn test_millis_10b() {
        const EPSILON: f64 = 0.02;
        const BASE_WARMUP_MILLIS: u64 = 500;
        const BASE_BENCH_TIME: Duration = Duration::from_millis(500);

        let target_latency = Duration::from_millis(10);
        let batch = batch_n(DEFAULT_N_BATCHES, target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }
}
