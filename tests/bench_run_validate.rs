#![cfg(feature = "_bench")]

//! cargo test -r --test bench_run_validate --all-features -- with_batch no_batch --nocapture --test-threads=1

use bench_utils::{
    BenchCfg, FpSeconds, LatencyUnit, RunLength,
    dev_support::{midpoint_value, quickmedian},
    latency_n,
    load::BusyWork,
    multi::BenchOut,
    rel_approx_eq_fpsecs,
    test_support::{AbsRelDiffFpSecs, count_for_acc_ltncy},
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
    base_target_latency: Duration,
    samp_size: usize,
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
    let status_millis = base_status_millis * K as u64;
    let count = batch.unwrap_or(1) * samp_size;

    println!(
        "validate_bench_run: K={K}, rec_unit={rec_unit:?}, base_target_latency={base_target_latency:?}, base_effort={base_effort}, warmup={warmup_millis}, batch={batch:?}, samp_size={samp_size}"
    );

    let cfg = BenchCfg::default()
        .with_warmup_millis(warmup_millis)
        .with_status_millis(status_millis)
        .with_recording_unit(rec_unit);
    let out = runner(&cfg, fsrc, RunLength::Count(count), batch);

    //=== The `v_*` variables are used to validate the bench_run output.

    let v_batch = count / samp_size;
    let mut v_latencies: [Vec<FpSeconds>; K] = array::from_fn(|_| Vec::with_capacity(samp_size));
    for _ in 0..samp_size {
        if K >= 1 {
            v_latencies[0].push(FpSeconds::from_duration(latency_n(&mut f1, v_batch)) / v_batch);
        }
        if K == 2 {
            v_latencies[1].push(FpSeconds::from_duration(latency_n(&mut f2, v_batch)) / v_batch);
        }
    }
    let v_medians: Vec<FpSeconds> = v_latencies
        .iter_mut()
        .map(|v| {
            quickmedian(v);
            midpoint_value(v)
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
const DEFAULT_REC_UNIT: LatencyUnit = LatencyUnit::NANO;
const DEFAULT_ACC_LTNCY: Duration = Duration::from_millis(1);
const DEFAULT_RUN_TIME: Duration = Duration::from_millis(100);

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

fn batch_opt_for_acc_ltncy(tgt_ltncy: Duration, acc_ltncy: Duration) -> Option<usize> {
    Some(count_for_acc_ltncy(tgt_ltncy, acc_ltncy))
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
        base_target_latency: Duration,
        samp_size: usize,
        batch: Option<usize>,
        epsilon: f64,
    ) {
        run(
            rec_unit,
            runner,
            Fns1::new(base_target_latency),
            base_warmup_millis,
            0,
            base_target_latency,
            samp_size,
            batch,
            epsilon,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_nanos_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_1b() {
        const EPSILON: f64 = 0.02;
        const SAMP_SIZE: usize = 100;
        let rec_unit = LatencyUnit::sub_sec(12);
        let target_latency = Duration::from_nanos(1);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_nanos_10 --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_10() {
        const EPSILON: f64 = 0.10;
        let rec_unit = LatencyUnit::sub_sec(12);
        let target_latency = Duration::from_nanos(10);
        let samp_size = 100_000; // samp_size >= 1_000_000 makes test's validation stage extremely slow
        let batch = None;
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_nanos_10b --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_10b() {
        const EPSILON: f64 = 0.02;
        const SAMP_SIZE: usize = 100;
        let rec_unit = LatencyUnit::sub_sec(11);
        let target_latency = Duration::from_nanos(10);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_nanos_100 --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_100() {
        const EPSILON: f64 = 0.02;
        let rec_unit = LatencyUnit::sub_sec(11);
        let target_latency = Duration::from_nanos(100);
        let samp_size = 100_000; // samp_size >= 1_000_000 makes test's validation stage extremely slow
        let batch = None;
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_nanos_100b --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_100b() {
        const EPSILON: f64 = 0.02;
        const SAMP_SIZE: usize = 100;
        let rec_unit = LatencyUnit::sub_sec(10);
        let target_latency = Duration::from_nanos(50);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_micros_1 --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_1() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_micros(1);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_micros_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_1b() {
        const EPSILON: f64 = 0.02;
        const SAMP_SIZE: usize = 100;
        let target_latency = Duration::from_micros(1);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_micros_50() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_micros(50);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_micros_50b --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_50b() {
        const EPSILON: f64 = 0.02;
        const SAMP_SIZE: usize = 20;
        let target_latency = Duration::from_micros(50);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_1() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(1);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_millis_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_millis_1b() {
        const EPSILON: f64 = 0.02;
        const SAMP_SIZE: usize = 20;
        let target_latency = Duration::from_millis(1);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_10() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(10);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_millis_50 --nocapture --test-threads=1
    #[test]
    fn test_millis_50() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(50);
        let samp_size = 10;
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            target_latency,
            samp_size,
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
        base_target_latency: Duration,
        samp_size: usize,
        batch: Option<usize>,
        epsilon: f64,
    ) {
        run(
            rec_unit,
            runner,
            Fns1::new(base_target_latency),
            base_warmup_millis,
            base_status_millis,
            base_target_latency,
            samp_size,
            batch,
            epsilon,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_nanos_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_1b() {
        const EPSILON: f64 = 0.15;
        const SAMP_SIZE: usize = 100;
        let rec_unit = LatencyUnit::sub_sec(12);
        let target_latency = Duration::from_nanos(1);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_nanos_10 --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_10() {
        const EPSILON: f64 = 0.25;
        let rec_unit = LatencyUnit::sub_sec(12);
        let target_latency = Duration::from_nanos(10);
        let samp_size = 100_000; // samp_size >= 1_000_000 makes test's validation stage extremely slow
        let batch = None;
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_nanos_10b --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_10b() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 100;
        let rec_unit = LatencyUnit::sub_sec(11);
        let target_latency = Duration::from_nanos(10);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_nanos_100 --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_100() {
        const EPSILON: f64 = 0.02;
        let rec_unit = LatencyUnit::sub_sec(11);
        let target_latency = Duration::from_nanos(100);
        let samp_size = 100_000; // samp_size >= 1_000_000 makes test's validation stage extremely slow
        let batch = None;
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_nanos_100b --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_100b() {
        const EPSILON: f64 = 0.02;
        const SAMP_SIZE: usize = 100;
        let rec_unit = LatencyUnit::sub_sec(10);
        let target_latency = Duration::from_nanos(50);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_micros_1 --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_1() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_micros(1);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_micros_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_1b() {
        const EPSILON: f64 = 0.02;
        const SAMP_SIZE: usize = 100;
        let target_latency = Duration::from_micros(1);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_micros_50() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_micros(50);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_micros_50b --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_50b() {
        const EPSILON: f64 = 0.02;
        const SAMP_SIZE: usize = 100;
        let target_latency = Duration::from_micros(50);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_1() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(1);
        let samp_size = 200;
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_millis_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_millis_1b() {
        const EPSILON: f64 = 0.02;
        const SAMP_SIZE: usize = 20;
        let target_latency = Duration::from_millis(1);
        let batch = Some(10);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_10() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(10);
        let samp_size = 20;
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_millis_50 --nocapture --test-threads=1
    #[test]
    fn test_millis_50() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(50);
        let samp_size = 10;
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            samp_size,
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
        base_target_latency: Duration,
        samp_size: usize,
        batch: Option<usize>,
        epsilon: f64,
    ) {
        run(
            rec_unit,
            runner,
            Fns2::new(base_target_latency),
            base_warmup_millis,
            0,
            base_target_latency,
            samp_size,
            batch,
            epsilon,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_nanos_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_1b() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 100;
        let rec_unit = LatencyUnit::sub_sec(12);
        let target_latency = Duration::from_nanos(1);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_nanos_10 --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_10() {
        const EPSILON: f64 = 0.50;
        let rec_unit = LatencyUnit::sub_sec(12);
        let target_latency = Duration::from_nanos(10);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_nanos_10b --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_10b() {
        const EPSILON: f64 = 0.03;
        const SAMP_SIZE: usize = 100;
        let rec_unit = LatencyUnit::sub_sec(11);
        let target_latency = Duration::from_nanos(10);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_nanos_100 --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_100() {
        const EPSILON: f64 = 0.30;
        let rec_unit = LatencyUnit::sub_sec(11);
        let target_latency = Duration::from_nanos(100);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_nanos_100b --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_100b() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 100;
        let rec_unit = LatencyUnit::sub_sec(10);
        let target_latency = Duration::from_nanos(50);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_micros_1 --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_1() {
        const EPSILON: f64 = 0.20;
        let target_latency = Duration::from_micros(1);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_micros_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_1b() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 100;
        let target_latency = Duration::from_micros(1);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_micros_50() {
        const EPSILON: f64 = 0.15;
        let target_latency = Duration::from_micros(50);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_micros_50b --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_50b() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 20;
        let target_latency = Duration::from_micros(50);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_1() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(1);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_millis_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_millis_1b() {
        const EPSILON: f64 = 0.02;
        const SAMP_SIZE: usize = 100;
        let target_latency = Duration::from_millis(1);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_10() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(10);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_millis_50 --nocapture --test-threads=1
    #[test]
    fn test_millis_50() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(50);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            target_latency,
            samp_size,
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
        base_target_latency: Duration,
        samp_size: usize,
        batch: Option<usize>,
        epsilon: f64,
    ) {
        run(
            rec_unit,
            runner,
            Fns2::new(base_target_latency),
            base_warmup_millis,
            base_status_millis,
            base_target_latency,
            samp_size,
            batch,
            epsilon,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_nanos_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_1b() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 100;
        let rec_unit = LatencyUnit::sub_sec(12);
        let target_latency = Duration::from_nanos(1);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_nanos_10 --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_10() {
        const EPSILON: f64 = 0.50;
        let rec_unit = LatencyUnit::sub_sec(12);
        let target_latency = Duration::from_nanos(10);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_nanos_10b --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_10b() {
        const EPSILON: f64 = 0.03;
        const SAMP_SIZE: usize = 100;
        let rec_unit = LatencyUnit::sub_sec(11);
        let target_latency = Duration::from_nanos(10);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_nanos_100 --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_100() {
        const EPSILON: f64 = 0.30;
        let rec_unit = LatencyUnit::sub_sec(11);
        let target_latency = Duration::from_nanos(100);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_nanos_100b --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_100b() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 100;
        let rec_unit = LatencyUnit::sub_sec(10);
        let target_latency = Duration::from_nanos(50);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            rec_unit,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_micros_1 --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_1() {
        const EPSILON: f64 = 0.20;
        let target_latency = Duration::from_micros(1);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_micros_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_1b() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 100;
        let target_latency = Duration::from_micros(1);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_micros_50() {
        const EPSILON: f64 = 0.15;
        let target_latency = Duration::from_micros(50);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_micros_50b --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_50b() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 100;
        let target_latency = Duration::from_micros(50);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_1() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(1);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_millis_1b --exact --nocapture --test-threads=1
    #[test]
    fn test_millis_1b() {
        const EPSILON: f64 = 0.02;
        const SAMP_SIZE: usize = 100;
        let target_latency = Duration::from_millis(1);
        let batch = batch_opt_for_acc_ltncy(target_latency, DEFAULT_ACC_LTNCY);
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            SAMP_SIZE,
            batch,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_10() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(10);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_millis_50 --nocapture --test-threads=1
    #[test]
    fn test_millis_50() {
        const EPSILON: f64 = 0.02;
        let target_latency = Duration::from_millis(50);
        let samp_size = count_for_acc_ltncy(target_latency, DEFAULT_RUN_TIME);
        let batch = None;
        run_bench(
            DEFAULT_REC_UNIT,
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            target_latency,
            samp_size,
            batch,
            EPSILON,
        );
    }
}
