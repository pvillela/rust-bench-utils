#![cfg(feature = "_bench")]

//! cargo test -r --test bench_run_validate --all-features -- with_batch no_batch --nocapture --test-threads=1

use bench_utils::{
    BenchCfg, FpSeconds, LatencyUnit, RunLength, latency_n,
    load::BusyWork,
    multi::BenchOut,
    rel_approx_eq_fpsecs,
    test_support::{AbsRelDiffFpSecs, midpoint_value, quickmedian},
};
use log::debug;
use std::{
    array,
    time::{Duration, Instant},
};

trait FnsSrc<const K: usize>: Clone {
    fn f1(&self) -> impl FnMut();
    fn f2(&self) -> impl FnMut();
    fn base_effort(&self) -> u32;
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
    R: FnOnce(&BenchCfg, Src, RunLength, Option<usize>) -> BenchOut<K>,
{
    _ = env_logger::try_init();

    assert!(1 <= K && K <= 2, "K={K} must be 1 or 2");
    let start = Instant::now();

    let fsrc1 = fsrc.clone();
    let mut f1 = fsrc1.f1();
    let mut f2 = fsrc1.f2();
    let base_effort = fsrc1.base_effort();

    let warmup_millis = base_warmup_millis * K as u64;
    let bench_time = base_bench_time * K as u32;
    let status_millis = base_status_millis * K as u64;

    println!(
        "validate_bench_run: K={K}, rec_unit={rec_unit:?}, base_target_latency={base_target_latency:?}, base_effort={base_effort}, warmup={warmup_millis}, bench_time={bench_time:?}, batch={batch:?}"
    );

    let cfg = BenchCfg::default()
        .with_warmup_millis(warmup_millis)
        .with_status_millis(status_millis)
        .with_recording_unit(rec_unit);
    let out = runner(&cfg, fsrc, RunLength::Time(bench_time), batch);

    //=== The `v_*` variables are used to validate the bench_run output.

    let v_batch = (out.n() as f64).sqrt().round() as usize;
    let v_n_batches = v_batch;
    let mut v_latencies: [Vec<FpSeconds>; K] =
        array::from_fn(|_| Vec::<FpSeconds>::with_capacity(v_batch));

    if K >= 1 {
        for _ in 0..v_n_batches {
            v_latencies[0].push(FpSeconds::from_duration(latency_n(&mut f1, v_batch)) / v_batch);
        }
    }
    if K == 2 {
        for _ in 0..v_n_batches {
            v_latencies[1].push(FpSeconds::from_duration(latency_n(&mut f2, v_batch)) / v_batch);
        }
    }

    let v_medians: Vec<_> = v_latencies
        .iter_mut()
        .map(|v_lat| {
            quickmedian(v_lat);
            midpoint_value(v_lat)
        })
        .collect();

    let base_target_fpsecs = FpSeconds::from_duration(base_target_latency);

    for i in 0..K {
        let out_median = out[i].median();
        let v_median = v_medians[i];

        println!(
            "base_target_fpsecs={base_target_fpsecs:?}, out[{i}].median()={out_median:?}, rel_diff={}",
            base_target_fpsecs.abs_rel_diff_fpsecs(out_median)
        );

        println!(
            "base_target_fpsecs={base_target_fpsecs:?}, v_medians[{i}]={v_median:?}, rel_diff={}",
            base_target_fpsecs.abs_rel_diff_fpsecs(v_median)
        );

        println!(
            "v_medians[{i}]={v_median:?}, out[{i}].median()={out_median:?}, rel_diff={}",
            v_median.abs_rel_diff_fpsecs(out_median)
        );

        println!(
            "out[{i}].hist().median()={:?}",
            out[i].hist().value_at_quantile(0.5)
        );

        println!("out[{i}].summary()={:?}", out[i].summary());
    }

    println!("test total elapsed time = {:?}", start.elapsed());

    // Assertions
    for i in 0..K {
        rel_approx_eq_fpsecs!(v_medians[i], out[i].median(), epsilon);
    }
}

const BASE_WARMUP_MILLIS: u64 = 100;
const BASE_STATUS_MILLIS: u64 = 10;
const BASE_BENCH_TIME: Duration = Duration::from_millis(100);
const DEFAULT_REC_UNIT: LatencyUnit = LatencyUnit::Nano;
const DEFAULT_N_BATCHES: usize = 50;

#[derive(Clone)]
struct Fns1 {
    effort: u32,
}

impl Fns1 {
    fn new(base_target_latency: Duration) -> Self {
        _ = env_logger::try_init();
        let effort = BusyWork::calibrate(base_target_latency);
        debug!("Fns1::new >>> effort={effort}");
        Self { effort }
    }
}

impl FnsSrc<1> for Fns1 {
    fn f1(&self) -> impl FnMut() {
        BusyWork::fun(self.effort)
    }

    fn f2(&self) -> impl FnMut() {
        || ()
    }

    fn base_effort(&self) -> u32 {
        self.effort
    }
}

#[derive(Clone)]
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

    fn base_effort(&self) -> u32 {
        self.effort0
    }
}

/// Calculates a batch size that yields approximately 1 batch execution given the `target_latency` and `bench_time`.
fn high_batch(target_latency: Duration, bench_time: Duration) -> Option<usize> {
    let max_batch = (bench_time.as_secs_f64() / target_latency.as_secs_f64()).ceil() as usize;
    Some(max_batch)
}

/// Calculates a batch size that yields a number of batch execution approximately equal to the batch size,
/// given the `target_latency` and `bench_time`.
fn mid_batch(target_latency: Duration, bench_time: Duration) -> Option<usize> {
    let mid_batch = (bench_time.as_secs_f64() / target_latency.as_secs_f64())
        .sqrt()
        .round() as usize;
    Some(mid_batch)
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
        cfg: &BenchCfg,
        fns: impl FnsSrc<1>,
        run_length: RunLength,
        batch: Option<usize>,
    ) -> BenchOut<1> {
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

    //=== nanos_1 is unstable
    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_nanos_1 --exact --nocapture --test-threads=1
    // #[test]
    // fn test_nanos_1() {
    //     const EPSILON: f64 = 0.05;
    //     let rec_unit = LatencyUnit::SubSec(12);
    //     let target_latency = Duration::from_nanos(1);
    //     let batch = None;
    //     run_bench(
    //         rec_unit,
    //         BASE_WARMUP_MILLIS,
    //         BASE_BENCH_TIME,
    //         target_latency,
    //         batch,
    //         EPSILON,
    //     );
    // }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_nanos_1mb --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_1mb() {
        const EPSILON: f64 = 0.05;
        let rec_unit = LatencyUnit::SubSec(12);
        let target_latency = Duration::from_nanos(1);
        let batch = mid_batch(target_latency, BASE_BENCH_TIME);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_nanos_1hb --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_1hb() {
        const EPSILON: f64 = 0.15;
        let rec_unit = LatencyUnit::SubSec(12);
        let target_latency = Duration::from_nanos(1);
        let batch = high_batch(target_latency, BASE_BENCH_TIME);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_nanos_10 --nocapture --test-threads=1
    #[test]
    fn test_nanos_10() {
        const EPSILON: f64 = 0.50;
        let rec_unit = LatencyUnit::SubSec(12);
        let target_latency = Duration::from_nanos(10);
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

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_nanos_10mb --nocapture --test-threads=1
    #[test]
    fn test_nanos_10mb() {
        const EPSILON: f64 = 0.03;
        let rec_unit = LatencyUnit::SubSec(11);
        let target_latency = Duration::from_nanos(10);
        let batch = mid_batch(target_latency, BASE_BENCH_TIME);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_nanos_10hb --nocapture --test-threads=1
    #[test]
    fn test_nanos_10hb() {
        const EPSILON: f64 = 0.05;
        let rec_unit = LatencyUnit::SubSec(11);
        let target_latency = Duration::from_nanos(10);
        let batch = high_batch(target_latency, BASE_BENCH_TIME);
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
        const EPSILON: f64 = 0.30;
        let rec_unit = LatencyUnit::SubSec(12);
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

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_nanos_50mb --nocapture --test-threads=1
    #[test]
    fn test_nanos_50mb() {
        const EPSILON: f64 = 0.05;
        let rec_unit = LatencyUnit::SubSec(11);
        let target_latency = Duration::from_nanos(50);
        let batch = mid_batch(target_latency, BASE_BENCH_TIME);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_nanos_50hb --nocapture --test-threads=1
    #[test]
    fn test_nanos_50hb() {
        const EPSILON: f64 = 0.25;
        let rec_unit = LatencyUnit::SubSec(11);
        let target_latency = Duration::from_nanos(50);
        let batch = high_batch(target_latency, BASE_BENCH_TIME);
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
        const EPSILON: f64 = 0.30;
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

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_micros_1hb --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_1hb() {
        const EPSILON: f64 = 0.35;
        let target_latency = Duration::from_micros(1);
        let batch = high_batch(target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_micros_1mb --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_1mb() {
        const EPSILON: f64 = 0.05;
        let target_latency = Duration::from_micros(1);
        let batch = high_batch(target_latency, BASE_BENCH_TIME);
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
        const EPSILON: f64 = 0.15;
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

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_micros_50hb --nocapture --test-threads=1
    #[test]
    fn test_micros_50hb() {
        const EPSILON: f64 = 0.30;
        let target_latency = Duration::from_micros(50);
        let batch = high_batch(target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_micros_50mb --nocapture --test-threads=1
    #[test]
    fn test_micros_50mb() {
        const EPSILON: f64 = 0.05;
        let target_latency = Duration::from_micros(50);
        let batch = mid_batch(target_latency, BASE_BENCH_TIME);
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

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_millis_1mb --exact --nocapture --test-threads=1
    #[test]
    fn test_millis_1mb() {
        const EPSILON: f64 = 0.02;
        // const BASE_WARMUP_MILLIS: u64 = 500;
        // const BASE_BENCH_TIME: Duration = Duration::from_millis(500);

        let target_latency = Duration::from_millis(1);
        let batch = mid_batch(target_latency, BASE_BENCH_TIME);
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

    #[test]
    fn test_millis_10a() {
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

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_millis_10mb --nocapture --test-threads=1
    #[test]
    fn test_millis_10mb() {
        const EPSILON: f64 = 0.02;
        const BASE_WARMUP_MILLIS: u64 = 500;
        const BASE_BENCH_TIME: Duration = Duration::from_millis(500);

        let target_latency = Duration::from_millis(10);
        let batch = mid_batch(target_latency, BASE_BENCH_TIME);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_BENCH_TIME,
            target_latency,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_millis_50 --nocapture --test-threads=1
    #[test]
    fn test_millis_50() {
        const EPSILON: f64 = 0.02;
        const BASE_WARMUP_MILLIS: u64 = 500;
        const BASE_BENCH_TIME: Duration = Duration::from_millis(500);

        let target_latency = Duration::from_millis(50);
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
}

// cargo test -r --test bench_run_validate --all-features -- with_status1 --nocapture --test-threads=1
mod with_status1 {
    use super::*;
    use bench_utils::bench_run_with_status_arg_cfg_o;

    fn runner(
        cfg: &BenchCfg,
        fns: impl FnsSrc<1>,
        run_length: RunLength,
        batch: Option<usize>,
    ) -> BenchOut<1> {
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
        cfg: &BenchCfg,
        fns: impl FnsSrc<2>,
        run_length: RunLength,
        batch: Option<usize>,
    ) -> BenchOut<2> {
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
        cfg: &BenchCfg,
        fns: impl FnsSrc<2>,
        run_length: RunLength,
        batch: Option<usize>,
    ) -> BenchOut<2> {
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
