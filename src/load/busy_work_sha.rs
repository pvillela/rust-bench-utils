use crate::{
    BenchCfg, FpSeconds, LatencyUnit, RunLength, bench_run_arg_cfg, latency, overhead,
    suggested_batch_size,
};
use log::debug;
use sha2::{Digest, Sha256};
use std::{hint::black_box, time::Duration};

#[derive(Debug, Clone, Copy)]
/// Output from [`BusyWork`] calibration functions.
///
/// A calibration models the mean latency of the [`BusyWork`] closure as proportional to its `effort`
/// argument, with the latency of one unit of effort measured at calibration time. This is an
/// approximation:
/// - The latency of the closure is only approximately proportional to `effort`.
/// - The speed of the machine may differ between the time of calibration and the time the closure is
///   executed, e.g., due to CPU frequency changes.
///
/// Consequently, the mean latency actually achieved typically differs from the calibrated latency by
/// a few percent, and may differ by more on machines whose CPU frequency varies.
pub struct Calibration {
    unit_ltncy: FpSeconds,
    /// The clock overhead of [`bench_run`](crate::bench_run) computed during calibration.
    overhead: FpSeconds,
}

impl Calibration {
    /// Returns the calibrated latency corresponding to the `effort` argument.
    ///
    /// This is an estimate of the mean latency of the [`BusyWork`] closure for `effort`; see
    /// [`Calibration`] for its accuracy.
    pub fn latency_for_effort(&self, effort: u32) -> FpSeconds {
        self.unit_ltncy * effort as f64
    }

    /// Returns a pair whose second component is the closest achievable mean latency and whose first component
    /// is the effort corresponding to the second component.
    ///
    /// "Closest achievable" is with respect to this calibration, whose smallest achievable latency is that of
    /// one unit of effort; see [`Calibration`] for its accuracy.
    pub fn effort_for_latency(&self, latency: FpSeconds) -> (u32, FpSeconds) {
        let ratio = latency / self.unit_ltncy;
        let effort = ratio.round().max(1.0);
        let calibr_ltncy = self.unit_ltncy * effort;
        (effort as u32, calibr_ltncy)
    }

    /// The clock overhead of [`bench_run`](crate::bench_run) computed during calibration.
    pub fn overhead(&self) -> FpSeconds {
        self.overhead
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
    /// Its latency is approximately proportional to `effort`.
    fn work(effort: u32) {
        for _ in 0..black_box(effort) {
            let buf = [0u8; 64];
            let mut hasher = Sha256::new();
            hasher.update(black_box(buf));
            black_box(hasher);
        }
    }

    const TOLERANCE: f64 = 0.005;
    const ITER_EFFORT: u32 = 1;
    const WARMUP_MILLIS: u64 = 500;
    const DEFAULT_BUDGET: RunLength = RunLength::Time(Duration::from_millis(10));

    /// Returns a [`Calibration`] object that can be used to estimate the effort required to achieve
    /// a desired mean latency.
    ///
    /// Calls [`calibrate_with_budget`](Self::calibrate_with_budget) using a default budget.
    pub fn calibrate() -> Calibration {
        Self::calibrate_with_budget(Self::DEFAULT_BUDGET)
    }

    /// Returns a [`Calibration`] object that can be used to estimate the effort required to achieve
    /// a desired mean latency, using an iterative process.
    ///
    /// `budget` limits the length of the iterative process by time and/or count
    /// (= accumulated calibration effort).
    /// The total calibration takes longer than `budget` because a warm-up period is added.
    ///
    /// See [`Calibration`] for the accuracy of the result.
    pub fn calibrate_with_budget(budget: RunLength) -> Calibration {
        // Warm-up and use the resulting preliminary calibration to prepare arguments for more accurate
        // linear calibration.
        let Calibration {
            unit_ltncy: prelim_unit_ltncy,
            overhead,
        } = Self::warmup(Self::WARMUP_MILLIS);

        let batch =
            suggested_batch_size(prelim_unit_ltncy, overhead, Self::TOLERANCE).ceil() as usize;
        let calib = Self::calibrate_linear(budget, Self::ITER_EFFORT, Some(batch), overhead);
        debug!(
            "BusyWork::calibrate_with_budget >>> budget={budget:?}, prelim_unit_ltncy={prelim_unit_ltncy:?}, iter_effort={}, calibr={calib:?}",
            Self::ITER_EFFORT
        );
        calib
    }

    /// Does the warm-up for [`Self::calibrate_with_budget`].
    fn warmup(warmup_millis: u64) -> Calibration {
        const NAIVE_RUN_LENGTH: RunLength = RunLength::Time(Duration::from_millis(10));

        let naive_calib = Self::calibrate_exponential(NAIVE_RUN_LENGTH);
        let overhead = overhead().median_r();
        let batch =
            suggested_batch_size(naive_calib.unit_ltncy, overhead, Self::TOLERANCE).ceil() as usize;

        let calib = Self::calibrate_linear(
            RunLength::Time(Duration::from_millis(warmup_millis)),
            Self::ITER_EFFORT,
            Some(batch),
            overhead,
        );

        calib
    }

    /// Estimation of the mean latency of one unit of `effort`, using an exponential iterative process
    /// and a naive estimator of mean.
    /// `budget` limits the length of the iterative process by time and/or count
    /// (= accumulated calibration effort).
    fn calibrate_exponential(budget: RunLength) -> Calibration {
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
                return Calibration {
                    unit_ltncy,
                    overhead: FpSeconds::ZERO,
                };
            }
        }

        unreachable!("above loop must return at some point")
    }

    /// Estimation of the mean latency of one unit of `effort`, using a linear iterative process and
    /// a robust estimator of mean.
    /// `budget` limits the length of the iterative process by time and/or count
    /// (= accumulated calibration effort).
    fn calibrate_linear(
        budget: RunLength,
        iter_effort: u32,
        batch: Option<usize>,
        overhead: FpSeconds,
    ) -> Calibration {
        let cfg = BenchCfg::default()
            .with_warmup_millis(0)
            .with_recording_unit(LatencyUnit::sub_sec(11));
        let out = bench_run_arg_cfg(&cfg, || Self::work(iter_effort), budget, batch);
        let unit_ltncy = out.mean_rob() / iter_effort as f64;
        Calibration {
            unit_ltncy,
            overhead,
        }
    }
}

#[cfg(test)]
#[cfg(feature = "_bench")]
mod tcommon {
    pub const TOLERANCE: f64 = 0.005;
    pub const WARMUP_MILLIS: u64 = 500;
}

#[cfg(test)]
#[cfg(feature = "_bench")]
/// cargo test -r --lib --all-features -- load::busy_work_sha::validate_latency --nocapture --test-threads=1
mod validate_latency {
    use super::*;
    use crate::{
        BenchCfg, FpSeconds, bench_run_arg_cfg, rel_approx_eq_fpsecs, suggested_batch_size,
        test_support::AbsRelDiffFpSecs,
    };
    use std::time::Instant;

    fn run(tgt: Duration, samp_size: usize) -> (FpSeconds, FpSeconds) {
        _ = env_logger::try_init();

        let start = Instant::now();
        let calib = BusyWork::calibrate();
        let (effort, tgt_fpsecs) = calib.effort_for_latency(tgt.into());
        let f = BusyWork::fun(effort);

        let cfg = BenchCfg::default().with_warmup_millis(tcommon::WARMUP_MILLIS);
        let batch =
            suggested_batch_size(tgt_fpsecs, calib.overhead(), tcommon::TOLERANCE).ceil() as usize;

        let out = bench_run_arg_cfg(&cfg, f, RunLength::Count(batch * samp_size), Some(batch));
        let latency_fpsecs = out.mean_rob();
        let rel_diff = tgt_fpsecs.abs_rel_diff_fpsecs(latency_fpsecs);

        let elapsed = start.elapsed();
        println!(
            "tgt={:?}, effort={}, tgt_fpsecs={:?}, batch={}, latency_fpsecs={:?}, rel_diff={}, elapsed_time={:?}",
            tgt, effort, tgt_fpsecs, batch, latency_fpsecs, rel_diff, elapsed
        );
        if effort == 1 && FpSeconds::from_duration(tgt).abs_rel_diff_fpsecs(tgt_fpsecs) > 0.2 {
            println!("*** tgt={tgt:?} too small for calibration");
        }
        (tgt_fpsecs, latency_fpsecs)
    }

    #[test]
    fn test_busy_work_ltncy_zero() {
        let tgt = Duration::ZERO;
        const SAMP_SIZE: usize = 100;
        let (_, latency_fpsecs) = run(tgt, SAMP_SIZE);
        assert!(
            latency_fpsecs > FpSeconds::ZERO,
            "zero-target calibration must produce non-zero work"
        );
    }

    #[test]
    fn test_busy_work_ltncy_1_nano() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 200;
        let tgt = Duration::from_nanos(1);
        let (tgt_fpsecs, latency_fpsecs) = run(tgt, SAMP_SIZE);
        rel_approx_eq_fpsecs!(tgt_fpsecs, latency_fpsecs, EPSILON);
    }

    #[test]
    fn test_busy_work_ltncy_10_nano() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 200;
        let tgt = Duration::from_nanos(10);
        let (tgt_fpsecs, latency_fpsecs) = run(tgt, SAMP_SIZE);
        rel_approx_eq_fpsecs!(tgt_fpsecs, latency_fpsecs, EPSILON);
    }

    #[test]
    fn test_busy_work_ltncy_100_nano() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 200;
        let tgt = Duration::from_nanos(100);
        let (tgt_fpsecs, latency_fpsecs) = run(tgt, SAMP_SIZE);
        rel_approx_eq_fpsecs!(tgt_fpsecs, latency_fpsecs, EPSILON);
    }

    #[test]
    fn test_busy_work_ltncy_1_micro() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 200;
        let tgt = Duration::from_micros(1);
        let (tgt_fpsecs, latency_fpsecs) = run(tgt, SAMP_SIZE);
        rel_approx_eq_fpsecs!(tgt_fpsecs, latency_fpsecs, EPSILON);
    }

    // cargo test -r --lib --all-features -- load::busy_work_sha::validate_latency::test_busy_work_ltncy_1_milli --nocapture --test-threads=1
    #[test]
    fn test_busy_work_ltncy_1_milli() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 200;
        let tgt = Duration::from_millis(1);
        let (tgt_fpsecs, latency_fpsecs) = run(tgt, SAMP_SIZE);
        rel_approx_eq_fpsecs!(tgt_fpsecs, latency_fpsecs, EPSILON);
    }

    // cargo test -r --lib --all-features -- load::busy_work_sha::validate_latency::test_busy_work_ltncy_10_milli --nocapture --test-threads=1
    #[test]
    fn test_busy_work_ltncy_10_milli() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 50;
        let tgt = Duration::from_millis(10);
        let (tgt_fpsecs, latency_fpsecs) = run(tgt, SAMP_SIZE);
        rel_approx_eq_fpsecs!(tgt_fpsecs, latency_fpsecs, EPSILON);
    }

    // cargo test -r --lib --all-features -- load::busy_work_sha::validate_latency::test_busy_work_ltncy_50_milli --nocapture --test-threads=1
    #[test]
    fn test_busy_work_ltncy_50_milli() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 20;
        let tgt = Duration::from_millis(50);
        let (tgt_fpsecs, latency_fpsecs) = run(tgt, SAMP_SIZE);
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
    use crate::{BenchCfg, duo, suggested_batch_size};
    use basic_stats::{dev_utils::ApproxEq, rel_approx_eq};
    use std::time::Instant;

    fn run(tgt1: Duration, ratio: f64, samp_size: usize) -> (f64, f64) {
        _ = env_logger::try_init();

        let start = Instant::now();
        let calib = BusyWork::calibrate();
        let (effort2, _) = calib.effort_for_latency(tgt1.into());
        let effort1 = (effort2 as f64 * ratio).round() as u32;
        let adjusted_ratio = effort1 as f64 / effort2 as f64;
        let f1 = BusyWork::fun(effort1);
        let f2 = BusyWork::fun(effort2);

        let cfg = BenchCfg::default().with_warmup_millis(tcommon::WARMUP_MILLIS);
        let batch =
            suggested_batch_size(tgt1.into(), calib.overhead(), tcommon::TOLERANCE).ceil() as usize;

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
            "tgt1={tgt1:?}, batch={batch}, effort1={effort1}, effort2={effort2}, target_ratio={ratio}, adjusted_ratio={adjusted_ratio}, latency_ratio={latency_ratio}, rel_diff={rel_diff}, adjusted_rel_diff={adjusted_rel_diff}, elapsed_time={elapsed:?}",
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
        let (adjusted_ratio, latency_ratio) = run(tgt1, RATIO, SAMP_SIZE);
        rel_approx_eq!(adjusted_ratio, latency_ratio, EPSILON);
    }

    // cargo test -r --lib --all-features -- load::busy_work_sha::validate_ratio::test_busy_work_ratio_100_nano --nocapture --test-threads=1
    #[test]
    fn test_busy_work_ratio_100_nano() {
        const EPSILON: f64 = 0.02;
        const SAMP_SIZE: usize = 200;
        let tgt1 = Duration::from_nanos(100);
        let (adjusted_ratio, latency_ratio) = run(tgt1, RATIO, SAMP_SIZE);
        rel_approx_eq!(adjusted_ratio, latency_ratio, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_1_micro() {
        const EPSILON: f64 = 0.02;
        const SAMP_SIZE: usize = 200;
        let tgt1 = Duration::from_micros(1);
        let (adjusted_ratio, latency_ratio) = run(tgt1, RATIO, SAMP_SIZE);
        rel_approx_eq!(adjusted_ratio, latency_ratio, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_10_micro() {
        const EPSILON: f64 = 0.02;
        const SAMP_SIZE: usize = 200;
        let tgt1 = Duration::from_micros(10);
        let (adjusted_ratio, latency_ratio) = run(tgt1, RATIO, SAMP_SIZE);
        rel_approx_eq!(adjusted_ratio, latency_ratio, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_100_micro() {
        const EPSILON: f64 = 0.01;
        const SAMP_SIZE: usize = 200;
        let tgt1 = Duration::from_micros(100);
        let (adjusted_ratio, latency_ratio) = run(tgt1, RATIO, SAMP_SIZE);
        rel_approx_eq!(adjusted_ratio, latency_ratio, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_1_milli() {
        const EPSILON: f64 = 0.01;
        // const SAMP_SIZE: usize = 200;
        const SAMP_SIZE: usize = 100;
        let tgt1 = Duration::from_millis(1);
        let (adjusted_ratio, latency_ratio) = run(tgt1, RATIO, SAMP_SIZE);
        rel_approx_eq!(adjusted_ratio, latency_ratio, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_10_milli() {
        const EPSILON: f64 = 0.01;
        // const SAMP_SIZE: usize = 50;
        const SAMP_SIZE: usize = 40;
        let tgt1 = Duration::from_millis(10);
        let (adjusted_ratio, latency_ratio) = run(tgt1, RATIO, SAMP_SIZE);
        rel_approx_eq!(adjusted_ratio, latency_ratio, EPSILON);
    }
}

#[cfg(test)]
#[cfg(feature = "_bench")]
// cargo test -r --lib --all-features -- load::busy_work_sha::validate_proportionality --nocapture --test-threads=1
//
/// Test whether the latency of the busy work function is proportional to its `effort` argument, by
/// comparing interleaved executions with `effort = n` and `effort = 1`.
mod validate_proportionality {
    use super::*;
    use crate::{BenchCfg, duo, suggested_batch_size};
    use basic_stats::{dev_utils::ApproxEq, rel_approx_eq};
    use std::time::Instant;

    /// Returns the ratio of the latency with `effort = n` to the latency with `effort = 1`, divided by `n`.
    /// The result is `1` if latency is exactly proportional to `effort`.
    fn run(n: u32, samp_size: usize) -> f64 {
        _ = env_logger::try_init();

        let start = Instant::now();
        let f1 = BusyWork::fun(n);
        let f2 = BusyWork::fun(1);

        let cfg = BenchCfg::default().with_warmup_millis(tcommon::WARMUP_MILLIS);

        // Preliminary unbatched estimate of unit latency, only used to size the batch.
        let rough = bench_run_arg_cfg(&cfg, f2.clone(), RunLength::Count(10_000), None).median_r();
        let batch =
            suggested_batch_size(rough, overhead().median_r(), tcommon::TOLERANCE).ceil() as usize;

        let out = duo::bench_run_arg_cfg(
            &cfg,
            f1,
            f2,
            RunLength::Count(batch * samp_size),
            Some(batch),
        );

        // Normalized ratio, expected to be close to 1.
        let norm_ratio = out.mean_ratio_f1_f2_rob() / n as f64;
        let rel_diff = norm_ratio.abs_rel_diff(1.0);

        let elapsed = start.elapsed();
        println!(
            "n={n}, batch={batch}, out_f1().median_r()={:?}, out_f2().median_r()={:?}, norm_ratio={norm_ratio}, rel_diff={rel_diff}, elapsed_time={elapsed:?}",
            out.out_f1().median_r(),
            out.out_f2().median_r(),
        );

        norm_ratio
    }

    const EPSILON: f64 = 0.03;

    #[test]
    fn test_busy_work_proportionality_2() {
        const SAMP_SIZE: usize = 200;
        let norm_ratio = run(2, SAMP_SIZE);
        rel_approx_eq!(1.0, norm_ratio, EPSILON);
    }

    #[test]
    fn test_busy_work_proportionality_4() {
        const SAMP_SIZE: usize = 200;
        let norm_ratio = run(4, SAMP_SIZE);
        rel_approx_eq!(1.0, norm_ratio, EPSILON);
    }

    #[test]
    fn test_busy_work_proportionality_36() {
        const SAMP_SIZE: usize = 200;
        let norm_ratio = run(36, SAMP_SIZE);
        rel_approx_eq!(1.0, norm_ratio, EPSILON);
    }

    #[test]
    fn test_busy_work_proportionality_3600() {
        const SAMP_SIZE: usize = 100;
        let norm_ratio = run(3_600, SAMP_SIZE);
        rel_approx_eq!(1.0, norm_ratio, EPSILON);
    }
}
