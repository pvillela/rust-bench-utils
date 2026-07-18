use crate::{FpSeconds, RunLength, latency};
use sha2::{Digest, Sha256};
use std::{hint::black_box, time::Duration};

/// Output from [`BusyWork`] calibration functions.
pub struct Calibration {
    unit_ltncy: FpSeconds,
}

impl Calibration {
    /// Returns the calibrated latency corresponding to the `effort` argument.
    pub fn latency_for_effort(&self, effort: u32) -> FpSeconds {
        self.unit_ltncy * effort as f64
    }

    /// Returns a pair whose second component is the closest achievable mean latency and whose first component
    /// is the effort corresponding to the second component.
    pub fn effort_for_latency(&self, latency: FpSeconds) -> (u32, FpSeconds) {
        let ratio = latency / self.unit_ltncy;
        let effort = ratio.round().max(1.0);
        let calibr_ltncy = self.unit_ltncy * effort;
        (effort as u32, calibr_ltncy)
    }
}

#[derive(Clone, Copy)]
/// Produces a closure which does a significant amount of computation, useful as a synthetic workload to support
/// the validation of benchmarking frameworks.
/// Gated by feature **"load"**.
///
/// The closure executes a work function whose latency is controlled by an `effort` value that is obtained by
/// running a calibration associated function.
pub struct BusyWork;

impl BusyWork {
    /// Closure which does a significant amount of computation to support validation of benchmarking frameworks.
    ///
    /// See [`calibrate`](Self::calibrate) and [`calibrate_with_budget`](Self::calibrate_with_budget) for how to
    /// determine the `effort` argument to achieve a desired target latency.
    #[inline(always)]
    pub fn fun(effort: u32) -> impl Fn() + Clone + use<> {
        move || Self::work(effort)
    }

    #[inline(always)]
    /// Does a significant amount of computation, based on SHA-256 (using the 'sha2' crate).
    /// Its latency is proportional to `effort`.
    fn work(effort: u32) {
        for _ in 0..black_box(effort) {
            let buf = [0u8; 64];
            let mut hasher = Sha256::new();
            hasher.update(black_box(buf));
            black_box(hasher);
        }
    }

    /// Returns a [`Calibration`] object that can be used to estimate the effort required to achieve
    /// a desired mean latency.
    ///
    /// Calls [`calibrate_with_budget`](Self::calibrate_with_budget) using a default budget.
    pub fn calibrate() -> Calibration {
        let budget: RunLength = RunLength::Time(Duration::from_millis(1));
        Self::calibrate_with_budget(budget)
    }

    /// Returns a [`Calibration`] object that can be used to estimate the effort required to achieve
    /// a desired mean latency, using an iterative process.
    ///
    /// `calibration_budget` limits the length of the iterative process by time and/or count
    /// (= accumulated calibration effort).
    /// The total calibration takes longer than `run_length` because a warm-up period is added.
    pub fn calibrate_with_budget(budget: RunLength) -> Calibration {
        Self::warmup(budget);
        Self::calibrate_internal(budget)
    }

    /// Does the warm-up for [`Self::calibrate_with_budget`].
    fn warmup(budget: RunLength) {
        Self::calibrate_internal(budget);
    }

    /// Core estimation of the mean latency of one unit of `effort`,
    /// using an iterative process. Used by both [`Self::warmup`] and [`Self::calibrate_with_budget`].
    /// `calibration_budget` limits the length of the iterative process by time and/or count
    /// (= accumulated calibration effort).
    fn calibrate_internal(budget: RunLength) -> Calibration {
        let (budget_count, budget_dur) = budget.count_and_time();
        let budget_fps = FpSeconds::from_duration(budget_dur);

        let mut acc_latency = FpSeconds::ZERO;
        let mut acc_effort_fp: f64 = 0.0;

        for i in 1.. {
            let iter_effort = 2u32.pow(i - 1);
            let iter_latency: FpSeconds = latency(|| Self::work(iter_effort)).into();

            acc_latency += iter_latency;
            acc_effort_fp += iter_effort as f64;

            if iter_latency >= budget_fps / 3
                || acc_latency >= budget_fps * 2 / 3
                || acc_effort_fp >= budget_count as f64 * (2.0 / 3.0)
            {
                // Estimate of unit latency based on latest iteration.
                let iter_unit_ltncy = iter_latency / iter_effort as f64;

                // Estimate of unit latency based on accumulated effort and latency for all iterations.
                let acc_unit_ltncy = acc_latency / acc_effort_fp;

                // The last iteration should have been the most efficient due to previous warming;
                // if that's not the case, return the accumulated estimate.
                let unit_ltncy = iter_unit_ltncy.min(acc_unit_ltncy.as_f64()).into();
                return Calibration { unit_ltncy };
            }
        }

        unreachable!("above loop must return at some point")
    }
}

#[cfg(test)]
#[cfg(feature = "_bench")]
/// cargo test -r --lib --all-features -- load::busy_work_sha::validate_latency --nocapture --test-threads=1
mod validate_latency {
    use super::*;
    use crate::{
        BenchCfg, FpSeconds, LatencyUnit, bench_run_arg_cfg, rel_approx_eq_fpsecs,
        test_support::{AbsRelDiffFpSecs, count_for_acc_ltncy},
    };
    use std::time::Instant;

    fn run(tgt: Duration, batch: usize, samp_size: usize) -> (FpSeconds, FpSeconds) {
        _ = env_logger::try_init();

        let start = Instant::now();
        let (effort, tgt_fpsecs) = BusyWork::calibrate().effort_for_latency(tgt.into());
        let f = BusyWork::fun(effort);

        let cfg = BenchCfg::default()
            .with_recording_unit(LatencyUnit::sub_sec(12))
            .with_warmup_millis(100);
        let out = bench_run_arg_cfg(&cfg, f, RunLength::Count(batch * samp_size), Some(batch));
        let latency_fpsecs = out.median_r();
        let rel_diff = tgt_fpsecs.abs_rel_diff_fpsecs(latency_fpsecs);

        let elapsed = start.elapsed();
        println!(
            "tgt={:?}, effort={}, tgt_fpsecs={:?}, latency_fpsecs={:?}, rel_diff={}, elapsed_time={:?}",
            tgt, effort, tgt_fpsecs, latency_fpsecs, rel_diff, elapsed
        );
        (tgt_fpsecs, latency_fpsecs)
    }

    // const ACC_LTNCY: Duration = Duration::from_micros(50);
    const ACC_LTNCY: Duration = Duration::from_millis(1);

    #[test]
    fn test_busy_work_ltncy_zero() {
        let tgt = Duration::ZERO;
        const SAMP_SIZE: usize = 100;
        let batch = count_for_acc_ltncy(Duration::from_nanos(1), ACC_LTNCY);
        let (_, latency_fpsecs) = run(tgt, batch, SAMP_SIZE);
        assert!(
            latency_fpsecs > FpSeconds::ZERO,
            "zero-target calibration must produce non-zero work"
        );
    }

    //=== below 100 nano: too small for proper calibration

    #[test]
    fn test_busy_work_ltncy_100_nano() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 200;
        let tgt = Duration::from_nanos(100);
        let batch = count_for_acc_ltncy(tgt, ACC_LTNCY);
        let (tgt_fpsecs, latency_fpsecs) = run(tgt, batch, SAMP_SIZE);
        rel_approx_eq_fpsecs!(tgt_fpsecs, latency_fpsecs, EPSILON);
    }

    #[test]
    fn test_busy_work_ltncy_1_micro() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 200;
        let tgt = Duration::from_micros(1);
        let batch = count_for_acc_ltncy(tgt, ACC_LTNCY);
        let (tgt_fpsecs, latency_fpsecs) = run(tgt, batch, SAMP_SIZE);
        rel_approx_eq_fpsecs!(tgt_fpsecs, latency_fpsecs, EPSILON);
    }

    // cargo test -r --lib --all-features -- load::busy_work_sha::validate_latency::test_busy_work_ltncy_1_milli --nocapture --test-threads=1
    #[test]
    fn test_busy_work_ltncy_1_milli() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 200;
        let tgt = Duration::from_millis(1);
        let batch = 10;
        let (tgt_fpsecs, latency_fpsecs) = run(tgt, batch, SAMP_SIZE);
        rel_approx_eq_fpsecs!(tgt_fpsecs, latency_fpsecs, EPSILON);
    }

    // cargo test -r --lib --all-features -- load::busy_work_sha::validate_latency::test_busy_work_ltncy_10_milli --nocapture --test-threads=1
    #[test]
    fn test_busy_work_ltncy_10_milli() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 50;
        let tgt = Duration::from_millis(10);
        let batch = 10;
        let (tgt_fpsecs, latency_fpsecs) = run(tgt, batch, SAMP_SIZE);
        rel_approx_eq_fpsecs!(tgt_fpsecs, latency_fpsecs, EPSILON);
    }

    // cargo test -r --lib --all-features -- load::busy_work_sha::validate_latency::test_busy_work_ltncy_50_milli --nocapture --test-threads=1
    #[test]
    fn test_busy_work_ltncy_50_milli() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 20;
        let tgt = Duration::from_millis(50);
        let batch = 10;
        let (tgt_fpsecs, latency_fpsecs) = run(tgt, batch, SAMP_SIZE);
        rel_approx_eq_fpsecs!(tgt_fpsecs, latency_fpsecs, EPSILON);
    }
}

#[cfg(test)]
#[cfg(feature = "_bench")]
// cargo test -r --lib --all-features -- load::busy_work_sha::validate_ratio --nocapture --test-threads=1
//
/// Test whether two busy work functions produce latencies that are proportional to the ratio of their
/// `effort` attributes. Checking is based on the cumulative latencies over a number of `repeats`.
mod validate_ratio {
    use super::*;
    use crate::{BenchCfg, LatencyUnit, duo, test_support::count_for_acc_ltncy};
    use basic_stats::{dev_utils::ApproxEq, rel_approx_eq};
    use std::time::Instant;

    // const ACC_LTNCY: Duration = Duration::from_micros(50);
    const ACC_LTNCY: Duration = Duration::from_micros(10);

    fn run(tgt1: Duration, ratio: f64, batch: usize, samp_size: usize) -> (f64, f64) {
        _ = env_logger::try_init();

        let start = Instant::now();
        let (effort2, _) = BusyWork::calibrate().effort_for_latency(tgt1.into());
        let effort1 = (effort2 as f64 * ratio).round() as u32;
        let adjusted_ratio = effort1 as f64 / effort2 as f64;
        let f1 = BusyWork::fun(effort1);
        let f2 = BusyWork::fun(effort2);

        let cfg = BenchCfg::default()
            .with_recording_unit(LatencyUnit::sub_sec(11))
            .with_warmup_millis(100);

        let out = duo::bench_run_arg_cfg(
            &cfg,
            f1,
            f2,
            RunLength::Count(batch * samp_size),
            Some(batch),
        );

        let latency_ratio = out.ratio_medians_f1_f2_r();
        let rel_diff = latency_ratio.abs_rel_diff(ratio);
        let adjusted_rel_diff = latency_ratio.abs_rel_diff(adjusted_ratio);

        println!(
            "out_f1().median_r()={:?}, out_f2().median_r()={:?}",
            out.out_f1().median_r(),
            out.out_f2().median_r()
        );

        let elapsed = start.elapsed();
        println!(
            "tgt1={tgt1:?}, effort1={effort1}, effort2={effort2}, target_ratio={ratio}, adjusted_ratio={adjusted_ratio}, latency_ratio={latency_ratio}, rel_diff={rel_diff}, adjusted_rel_diff={adjusted_rel_diff}, elapsed_time={elapsed:?}",
        );

        (adjusted_ratio, latency_ratio)
    }

    const RATIO: f64 = 1.20;

    // cargo test -r --lib --all-features -- load::busy_work_sha::validate_ratio::test_busy_work_ratio_10_nano --nocapture --test-threads=1
    #[test]
    // too small for proper calibration
    fn test_busy_work_ratio_10_nano() {
        const EPSILON: f64 = 0.10; // overtakes the ratio relative difference
        const SAMP_SIZE: usize = 200;
        let tgt1 = Duration::from_nanos(10);
        let batch = count_for_acc_ltncy(tgt1, ACC_LTNCY);
        let (adjusted_ratio, latency_ratio) = run(tgt1, RATIO, batch, SAMP_SIZE);
        rel_approx_eq!(adjusted_ratio, latency_ratio, EPSILON);
    }

    // cargo test -r --lib --all-features -- load::busy_work_sha::validate_ratio::test_busy_work_ratio_100_nano --nocapture --test-threads=1
    #[test]
    fn test_busy_work_ratio_100_nano() {
        const EPSILON: f64 = 0.02;
        const SAMP_SIZE: usize = 200;
        let tgt1 = Duration::from_nanos(100);
        let batch = count_for_acc_ltncy(tgt1, ACC_LTNCY);
        let (adjusted_ratio, latency_ratio) = run(tgt1, RATIO, batch, SAMP_SIZE);
        rel_approx_eq!(adjusted_ratio, latency_ratio, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_1_micro() {
        const EPSILON: f64 = 0.02;
        const SAMP_SIZE: usize = 200;
        let tgt1 = Duration::from_micros(1);
        let batch = count_for_acc_ltncy(tgt1, ACC_LTNCY);
        let (adjusted_ratio, latency_ratio) = run(tgt1, RATIO, batch, SAMP_SIZE);
        rel_approx_eq!(adjusted_ratio, latency_ratio, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_10_micro() {
        const EPSILON: f64 = 0.02;
        const SAMP_SIZE: usize = 200;
        let tgt1 = Duration::from_micros(1);
        let batch = count_for_acc_ltncy(tgt1, ACC_LTNCY);
        let (adjusted_ratio, latency_ratio) = run(tgt1, RATIO, batch, SAMP_SIZE);
        rel_approx_eq!(adjusted_ratio, latency_ratio, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_100_micro() {
        const EPSILON: f64 = 0.01;
        const SAMP_SIZE: usize = 200;
        let tgt1 = Duration::from_micros(100);
        let batch = 10;
        let (adjusted_ratio, latency_ratio) = run(tgt1, RATIO, batch, SAMP_SIZE);
        rel_approx_eq!(adjusted_ratio, latency_ratio, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_1_milli() {
        const EPSILON: f64 = 0.01;
        // const SAMP_SIZE: usize = 200;
        const SAMP_SIZE: usize = 100;
        let tgt1 = Duration::from_millis(1);
        let batch = 10;
        let (adjusted_ratio, latency_ratio) = run(tgt1, RATIO, batch, SAMP_SIZE);
        rel_approx_eq!(adjusted_ratio, latency_ratio, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_10_milli() {
        const EPSILON: f64 = 0.01;
        // const SAMP_SIZE: usize = 50;
        const SAMP_SIZE: usize = 40;
        let tgt1 = Duration::from_millis(10);
        let batch = 10;
        let (adjusted_ratio, latency_ratio) = run(tgt1, RATIO, batch, SAMP_SIZE);
        rel_approx_eq!(adjusted_ratio, latency_ratio, EPSILON);
    }
}
