#![cfg(feature = "_bench")]

//! cargo test -r --test bench_run_validate --all-features -- with_batch no_batch --nocapture --test-threads=1

use bench_utils::{
    BenchCfg, FpSeconds, RunLength,
    dev_support::{midpoint_value, quickmedian},
    latency_n,
    load::{BusyWork, Calibration},
    multi::BenchOut,
    rel_approx_eq_fpsecs, suggested_batch_size,
    test_support::AbsRelDiffFpSecs,
};
use log::debug;
use std::{
    array,
    fmt::Debug,
    sync::LazyLock,
    time::{Duration, Instant},
};

trait FnsSrc<const K: usize>: Clone + Debug {
    fn f1(&self) -> impl FnMut();
    fn f2(&self) -> impl FnMut();
    fn batch_sz(&self) -> usize;
    fn base_tgt_ltncy(&self) -> FpSeconds;
}

fn run<const K: usize, R, Src>(
    runner: R,
    fsrc: Src,
    base_warmup_millis: u64,
    base_status_millis: u64,
    samp_size: usize,
    epsilon: f64,
) where
    Src: FnsSrc<K>,
    R: FnOnce(&BenchCfg, Src, RunLength, Option<usize>) -> BenchOut<K>,
{
    _ = env_logger::try_init();

    assert!(1 <= K && K <= 2, "K={K} must be 1 or 2");
    let start = Instant::now();

    let fsrc_cln = fsrc.clone();
    let mut f1 = fsrc_cln.f1();
    let mut f2 = fsrc_cln.f2();

    let warmup_millis = base_warmup_millis * K as u64; // not sure need to multiply by K
    let status_millis = base_status_millis * K as u64;
    let batch_sz = fsrc.batch_sz();
    let base_tgt_ltncy = fsrc.base_tgt_ltncy();
    let count = batch_sz * samp_size;

    println!(
        "validate_bench_run: K={K}, fsrc={fsrc:?}, warmup={warmup_millis}, batch_size={batch_sz}, samp_size={samp_size}"
    );

    let cfg = BenchCfg::default()
        .with_warmup_millis(warmup_millis)
        .with_status_millis(status_millis);
    let out = runner(&cfg, fsrc, RunLength::Count(count), Some(batch_sz));

    //=== The `v_*` variables are used to validate the bench_run output.

    let mut v_latencies: [Vec<FpSeconds>; K] = array::from_fn(|_| Vec::with_capacity(samp_size));
    for _ in 0..samp_size {
        if K >= 1 {
            v_latencies[0].push(FpSeconds::from_duration(latency_n(&mut f1, batch_sz)) / batch_sz);
        }
        if K == 2 {
            v_latencies[1].push(FpSeconds::from_duration(latency_n(&mut f2, batch_sz)) / batch_sz);
        }
    }

    let v_medians: Vec<FpSeconds> = v_latencies
        .iter_mut()
        .map(|v| {
            quickmedian(v);
            midpoint_value(v)
        })
        .collect();

    for i in 0..K {
        let out_median = out[i].median_r();
        let v_median = v_medians[i];

        println!(
            "base_tgt_ltncy={base_tgt_ltncy:?}, out[{i}].median()={out_median:?}, rel_diff={}",
            base_tgt_ltncy.abs_rel_diff_fpsecs(out_median)
        );

        println!(
            "base_tgt_ltncy={base_tgt_ltncy:?}, v_medians[{i}]={v_median:?}, rel_diff={}",
            base_tgt_ltncy.abs_rel_diff_fpsecs(v_median)
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
        rel_approx_eq_fpsecs!(v_medians[i], out[i].median_r(), epsilon);
    }
}

const BASE_WARMUP_MILLIS: u64 = 100;
const BASE_STATUS_MILLIS: u64 = 10;
const DEFAULT_SAMP_SIZE: usize = 200;
const DEFAULT_TOLERANCE: f64 = 0.005;

static CALIBR: LazyLock<Calibration> = LazyLock::new(|| BusyWork::calibrate());

#[derive(Clone, Copy, Debug)]
enum LatencyDriver {
    Effort(u32),
    Latency(Duration),
}

#[derive(Clone)]
struct Fns<const K: usize> {
    ltncy_driver: LatencyDriver,
    calibr: Calibration,
}

impl<const K: usize> Fns<K> {
    fn new(ltncy_driver: LatencyDriver, calibr: Calibration) -> Self {
        _ = env_logger::try_init();

        let res = Self {
            ltncy_driver,
            calibr,
        };
        debug!("Fns<{K}>::new >>> {res:?}");
        res
    }

    fn base_effort(&self) -> u32 {
        match self.ltncy_driver {
            LatencyDriver::Effort(effort) => effort,
            LatencyDriver::Latency(ltncy) => self.calibr.effort_for_latency(ltncy.into()).0,
        }
    }

    fn batch_sz(&self) -> usize {
        let overhead = self.calibr.overhead();
        let rough_ltncy = self.calibr.latency_for_effort(self.base_effort());
        suggested_batch_size(rough_ltncy, overhead, DEFAULT_TOLERANCE).ceil() as usize
    }

    fn base_tgt_ltncy(&self) -> FpSeconds {
        match self.ltncy_driver {
            LatencyDriver::Effort(effort) => self.calibr.latency_for_effort(effort),
            LatencyDriver::Latency(ltncy) => self.calibr.effort_for_latency(ltncy.into()).1,
        }
    }

    fn f1(&self) -> impl FnMut() {
        BusyWork::fun(self.base_effort())
    }
}

impl<const K: usize> Debug for Fns<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (calibr_effort, calibr_ltncy) = match self.ltncy_driver {
            LatencyDriver::Effort(effort) => (effort, self.calibr.latency_for_effort(effort)),
            LatencyDriver::Latency(ltncy) => self.calibr.effort_for_latency(ltncy.into()),
        };
        f.write_str(&format!(
            "Fns<{K}> {{ltncy_driver={:?}, calibr={:?}, calibr_effort={}, calibr_ltncy={:?}}}",
            self.ltncy_driver, self.calibr, calibr_effort, calibr_ltncy
        ))
    }
}

impl Fns<1> {
    fn f2(&self) -> impl FnMut() {
        || ()
    }
}

impl Fns<2> {
    fn effort_delta(&self) -> u32 {
        self.base_effort() / 10
    }

    fn f2(&self) -> impl FnMut() {
        let f2a = BusyWork::fun(self.base_effort() - self.effort_delta());
        let f2b = BusyWork::fun(self.effort_delta());
        move || {
            f2a();
            f2b()
        }
    }
}

impl FnsSrc<1> for Fns<1> {
    fn f1(&self) -> impl FnMut() {
        self.f1()
    }

    fn f2(&self) -> impl FnMut() {
        self.f2()
    }

    fn batch_sz(&self) -> usize {
        self.batch_sz()
    }

    fn base_tgt_ltncy(&self) -> FpSeconds {
        self.base_tgt_ltncy()
    }
}

impl FnsSrc<2> for Fns<2> {
    fn f1(&self) -> impl FnMut() {
        self.f1()
    }

    fn f2(&self) -> impl FnMut() {
        self.f2()
    }

    fn batch_sz(&self) -> usize {
        self.batch_sz()
    }

    fn base_tgt_ltncy(&self) -> FpSeconds {
        self.base_tgt_ltncy()
    }
}

// cargo test -r --test bench_run_validate --all-features -- no_status1 --nocapture --test-threads=1
mod no_status1 {
    use super::*;
    use bench_utils::bench_run_arg_cfg;

    fn runner(
        cfg: &BenchCfg,
        fns: impl FnsSrc<1>,
        run_length: RunLength,
        batch: Option<usize>,
    ) -> BenchOut<1> {
        bench_run_arg_cfg(cfg, fns.f1(), run_length, batch).into()
    }

    fn run_bench(
        base_warmup_millis: u64,
        ltncy_driver: LatencyDriver,
        samp_size: usize,
        epsilon: f64,
    ) {
        run(
            runner,
            Fns::<1>::new(ltncy_driver, *CALIBR),
            base_warmup_millis,
            0,
            samp_size,
            epsilon,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_effort_1 --exact --nocapture --test-threads=1
    #[test]
    fn test_effort_1() {
        const EPSILON: f64 = 0.03;
        let ltncy_driver = LatencyDriver::Effort(1);
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, DEFAULT_SAMP_SIZE, EPSILON);
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_effort_10 --exact --nocapture --test-threads=1
    #[test]
    fn test_effort_10() {
        const EPSILON: f64 = 0.03;
        let ltncy_driver = LatencyDriver::Effort(10);
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, DEFAULT_SAMP_SIZE, EPSILON);
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_effort_100 --exact --nocapture --test-threads=1
    #[test]
    fn test_effort_100() {
        const EPSILON: f64 = 0.03;
        let ltncy_driver = LatencyDriver::Effort(100);
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, DEFAULT_SAMP_SIZE, EPSILON);
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_nanos_100 --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_100() {
        const EPSILON: f64 = 0.03;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_nanos(100));
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, DEFAULT_SAMP_SIZE, EPSILON);
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_micros_1 --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_1() {
        const EPSILON: f64 = 0.03;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_micros(1));
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, DEFAULT_SAMP_SIZE, EPSILON);
    }

    #[test]
    fn test_micros_10() {
        const EPSILON: f64 = 0.03;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_micros(10));
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, DEFAULT_SAMP_SIZE, EPSILON);
    }

    #[test]
    fn test_micros_100() {
        const EPSILON: f64 = 0.03;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_micros(100));
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, DEFAULT_SAMP_SIZE, EPSILON);
    }

    #[test]
    fn test_millis_1() {
        const EPSILON: f64 = 0.03;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_millis(1));
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, DEFAULT_SAMP_SIZE, EPSILON);
    }

    #[test]
    fn test_millis_10() {
        const EPSILON: f64 = 0.03;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_millis(10));
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, DEFAULT_SAMP_SIZE, EPSILON);
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status1::test_millis_50 --nocapture --test-threads=1
    #[test]
    fn test_millis_50() {
        const EPSILON: f64 = 0.03;
        const SAMP_SIZE: usize = 50;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_millis(50));
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, SAMP_SIZE, EPSILON);
    }
}

// cargo test -r --test bench_run_validate --all-features -- with_status1 --nocapture --test-threads=1
mod with_status1 {
    use super::*;
    use bench_utils::bench_run_with_status_arg_cfg;

    fn runner(
        cfg: &BenchCfg,
        fns: impl FnsSrc<1>,
        run_length: RunLength,
        batch: Option<usize>,
    ) -> BenchOut<1> {
        bench_run_with_status_arg_cfg(cfg, fns.f1(), run_length, batch).into()
    }

    fn run_bench(
        base_warmup_millis: u64,
        base_status_millis: u64,
        ltncy_driver: LatencyDriver,
        samp_size: usize,
        epsilon: f64,
    ) {
        run(
            runner,
            Fns::<1>::new(ltncy_driver, *CALIBR),
            base_warmup_millis,
            base_status_millis,
            samp_size,
            epsilon,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_effort_1 --exact --nocapture --test-threads=1
    #[test]
    fn test_effort_1() {
        const EPSILON: f64 = 0.03;
        let ltncy_driver = LatencyDriver::Effort(1);
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            DEFAULT_SAMP_SIZE,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_effort_10 --exact --nocapture --test-threads=1
    #[test]
    fn test_effort_10() {
        const EPSILON: f64 = 0.03;
        let ltncy_driver = LatencyDriver::Effort(10);
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            DEFAULT_SAMP_SIZE,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_effort_100 --exact --nocapture --test-threads=1
    #[test]
    fn test_effort_100() {
        const EPSILON: f64 = 0.03;
        let ltncy_driver = LatencyDriver::Effort(100);
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            DEFAULT_SAMP_SIZE,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_nanos_100 --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_100() {
        const EPSILON: f64 = 0.03;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_nanos(100));
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            DEFAULT_SAMP_SIZE,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_micros_1 --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_1() {
        const EPSILON: f64 = 0.03;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_micros(1));
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            DEFAULT_SAMP_SIZE,
            EPSILON,
        );
    }

    #[test]
    fn test_micros_10() {
        const EPSILON: f64 = 0.03;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_micros(10));
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            DEFAULT_SAMP_SIZE,
            EPSILON,
        );
    }

    #[test]
    fn test_micros_100() {
        const EPSILON: f64 = 0.03;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_micros(100));
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            DEFAULT_SAMP_SIZE,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_1() {
        const EPSILON: f64 = 0.03;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_millis(1));
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            DEFAULT_SAMP_SIZE,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_10() {
        const EPSILON: f64 = 0.03;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_millis(10));
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            DEFAULT_SAMP_SIZE,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status1::test_millis_50 --nocapture --test-threads=1
    #[test]
    fn test_millis_50() {
        const EPSILON: f64 = 0.03;
        const SAMP_SIZE: usize = 50;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_millis(50));
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            SAMP_SIZE,
            EPSILON,
        );
    }
}

// cargo test -r --test bench_run_validate --all-features -- no_status2 --nocapture --test-threads=1
mod no_status2 {
    use super::*;
    use bench_utils::duo::bench_run_arg_cfg;

    fn runner(
        cfg: &BenchCfg,
        fns: impl FnsSrc<2>,
        run_length: RunLength,
        batch: Option<usize>,
    ) -> BenchOut<2> {
        bench_run_arg_cfg(cfg, fns.f1(), fns.f2(), run_length, batch)
    }

    fn run_bench(
        base_warmup_millis: u64,
        ltncy_driver: LatencyDriver,
        samp_size: usize,
        epsilon: f64,
    ) {
        run(
            runner,
            Fns::<2>::new(ltncy_driver, *CALIBR),
            base_warmup_millis,
            0,
            samp_size,
            epsilon,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_effort_1 --exact --nocapture --test-threads=1
    #[test]
    fn test_effort_1() {
        const EPSILON: f64 = 0.04;
        let ltncy_driver = LatencyDriver::Effort(1);
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, DEFAULT_SAMP_SIZE, EPSILON);
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_effort_10 --exact --nocapture --test-threads=1
    #[test]
    fn test_effort_10() {
        const EPSILON: f64 = 0.04;
        let ltncy_driver = LatencyDriver::Effort(10);
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, DEFAULT_SAMP_SIZE, EPSILON);
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_effort_100 --exact --nocapture --test-threads=1
    #[test]
    fn test_effort_100() {
        const EPSILON: f64 = 0.04;
        let ltncy_driver = LatencyDriver::Effort(100);
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, DEFAULT_SAMP_SIZE, EPSILON);
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_nanos_100 --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_100() {
        const EPSILON: f64 = 0.04;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_nanos(100));
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, DEFAULT_SAMP_SIZE, EPSILON);
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_micros_1 --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_1() {
        const EPSILON: f64 = 0.04;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_micros(1));
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, DEFAULT_SAMP_SIZE, EPSILON);
    }

    #[test]
    fn test_micros_10() {
        const EPSILON: f64 = 0.04;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_micros(10));
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, DEFAULT_SAMP_SIZE, EPSILON);
    }

    #[test]
    fn test_micros_100() {
        const EPSILON: f64 = 0.04;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_micros(100));
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, DEFAULT_SAMP_SIZE, EPSILON);
    }

    #[test]
    fn test_millis_1() {
        const EPSILON: f64 = 0.04;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_millis(1));
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, DEFAULT_SAMP_SIZE, EPSILON);
    }

    #[test]
    fn test_millis_10() {
        const EPSILON: f64 = 0.04;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_millis(10));
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, DEFAULT_SAMP_SIZE, EPSILON);
    }

    // cargo test -r --test bench_run_validate --all-features -- no_status2::test_millis_50 --nocapture --test-threads=1
    #[test]
    fn test_millis_50() {
        const EPSILON: f64 = 0.04;
        const SAMP_SIZE: usize = 50;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_millis(50));
        run_bench(BASE_WARMUP_MILLIS, ltncy_driver, SAMP_SIZE, EPSILON);
    }
}

// cargo test -r --test bench_run_validate --all-features -- with_status2 --nocapture --test-threads=1
mod with_status2 {
    use super::*;
    use bench_utils::duo::bench_run_with_status_arg_cfg;

    fn runner(
        cfg: &BenchCfg,
        fns: impl FnsSrc<2>,
        run_length: RunLength,
        batch: Option<usize>,
    ) -> BenchOut<2> {
        bench_run_with_status_arg_cfg(cfg, fns.f1(), fns.f2(), run_length, batch)
    }

    fn run_bench(
        base_warmup_millis: u64,
        base_status_millis: u64,
        ltncy_driver: LatencyDriver,
        samp_size: usize,
        epsilon: f64,
    ) {
        run(
            runner,
            Fns::<2>::new(ltncy_driver, *CALIBR),
            base_warmup_millis,
            base_status_millis,
            samp_size,
            epsilon,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_effort_1 --exact --nocapture --test-threads=1
    #[test]
    fn test_effort_1() {
        const EPSILON: f64 = 0.04;
        let ltncy_driver = LatencyDriver::Effort(1);
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            DEFAULT_SAMP_SIZE,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_effort_10 --exact --nocapture --test-threads=1
    #[test]
    fn test_effort_10() {
        const EPSILON: f64 = 0.04;
        let ltncy_driver = LatencyDriver::Effort(10);
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            DEFAULT_SAMP_SIZE,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_effort_100 --exact --nocapture --test-threads=1
    #[test]
    fn test_effort_100() {
        const EPSILON: f64 = 0.04;
        let ltncy_driver = LatencyDriver::Effort(100);
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            DEFAULT_SAMP_SIZE,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_nanos_100 --exact --nocapture --test-threads=1
    #[test]
    fn test_nanos_100() {
        const EPSILON: f64 = 0.04;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_nanos(100));
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            DEFAULT_SAMP_SIZE,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_micros_1 --exact --nocapture --test-threads=1
    #[test]
    fn test_micros_1() {
        const EPSILON: f64 = 0.04;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_micros(1));
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            DEFAULT_SAMP_SIZE,
            EPSILON,
        );
    }

    #[test]
    fn test_micros_10() {
        const EPSILON: f64 = 0.04;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_micros(10));
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            DEFAULT_SAMP_SIZE,
            EPSILON,
        );
    }

    #[test]
    fn test_micros_100() {
        const EPSILON: f64 = 0.04;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_micros(100));
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            DEFAULT_SAMP_SIZE,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_1() {
        const EPSILON: f64 = 0.04;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_millis(1));
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            DEFAULT_SAMP_SIZE,
            EPSILON,
        );
    }

    #[test]
    fn test_millis_10() {
        const EPSILON: f64 = 0.04;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_millis(10));
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            DEFAULT_SAMP_SIZE,
            EPSILON,
        );
    }

    // cargo test -r --test bench_run_validate --all-features -- with_status2::test_millis_50 --nocapture --test-threads=1
    #[test]
    fn test_millis_50() {
        const EPSILON: f64 = 0.04;
        const SAMP_SIZE: usize = 50;
        let ltncy_driver = LatencyDriver::Latency(Duration::from_millis(50));
        run_bench(
            BASE_WARMUP_MILLIS,
            BASE_STATUS_MILLIS,
            ltncy_driver,
            SAMP_SIZE,
            EPSILON,
        );
    }
}
