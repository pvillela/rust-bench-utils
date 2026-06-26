use crate::{RunLength, latency};
use std::{hint::black_box, time::Duration};

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
    pub fn fun(effort: u32) -> impl FnMut() + Clone + use<> {
        let mut state = Self::pre_work();
        move || Self::work(effort, &mut state)
    }

    #[inline(always)]
    /// Does a significant amount of computation. Its latency is proportional to `effort`.
    fn work(effort: u32, state: &mut u64) {
        // const INNER_STRIDE: u64 = 64;
        const INNER_STRIDE: u64 = 1;
        const MULT: u64 = 6364136223846793005;
        const INC: u64 = 1442695040888963407;

        let total = (effort as u64).wrapping_mul(INNER_STRIDE);
        for _ in 0..black_box(total) {
            *state = black_box(state.wrapping_mul(MULT).wrapping_add(INC));
        }
    }

    /// Estimates the `effort` required for the resulting closure to have the `target_latency`, using
    /// an iterative process.
    ///
    /// Calls [`calibrate_with_budget`](Self::calibrate_with_budget) using a default budget.
    pub fn calibrate(target_latency: Duration) -> u32 {
        let budget: RunLength = RunLength::Time(Duration::from_millis(1).max(target_latency));
        Self::calibrate_with_budget(target_latency, budget)
    }

    /// Estimates the `effort` required for the resulting closure to have the `target_latency`, using
    /// an iterative process.
    /// `calibration_budget` limits the length of the iterative process by time and/or count
    /// (= accumulated calibration effort).
    ///
    /// The total calibration takes longer than `run_length` because a warm-up period is added.
    pub fn calibrate_with_budget(target_latency: Duration, budget: RunLength) -> u32 {
        let mut state = Self::pre_work();
        Self::warmup(&mut state, target_latency, budget);
        Self::calibrate_internal(&mut state, target_latency, budget)
    }

    #[inline(always)]
    /// Does the set-up for [`Self::work`] (using the 'sha2' crate) so that the latter's latency is
    /// directly proportional to `effort`.
    fn pre_work() -> u64 {
        0
    }

    /// Does the warm-up for [`Self::work`] (using the 'sha2' crate) so that the latter's latency is
    /// directly proportional to `effort`.
    fn warmup(state: &mut u64, target_latency: Duration, budget: RunLength) {
        Self::calibrate_internal(state, target_latency, budget);
    }

    /// Core estimation of the `effort` required for the resulting closure to have the `target_latency`,
    /// using an iterative process. Used by both [`Self::warmup`] and [`Self::calibrate_with_budget`].
    /// `calibration_budget` limits the length of the iterative process by time and/or count
    /// (= accumulated calibration effort).
    fn calibrate_internal(state: &mut u64, target_latency: Duration, budget: RunLength) -> u32 {
        let (budget_count, budget_dur) = budget.exec_count_and_duration();
        let mut acc_latency = Duration::ZERO;
        let mut acc_effort: u32 = 0;

        for i in 1.. {
            let iter_effort = 2u32.pow(i - 1);
            let iter_latency = latency(|| Self::work(iter_effort, state));

            acc_latency += iter_latency;
            acc_effort += iter_effort;

            // Castings to f64 to avoid integer overflow or truncation to zero.
            if iter_latency >= budget_dur / 3
                || acc_latency.as_secs_f64() >= budget_dur.as_secs_f64() * (2.0 / 3.0)
                || acc_effort as f64 >= budget_count as f64 * (2.0 / 3.0)
            {
                // Estimate of target effort based on latest iteration.
                let iter_target_effort = (target_latency.as_secs_f64() * iter_effort as f64
                    / iter_latency.as_secs_f64())
                .round() as u32;

                // Estimate of target effort based on weighted average of the estimated target efforts
                // for all iterations.
                let acc_target_effort = (target_latency.as_secs_f64() * acc_effort as f64
                    / acc_latency.as_secs_f64())
                .round() as u32;

                // The last iteration should have been the most efficient due to previous warming;
                // if that's not the case, returns the weighted average estimated target effort.
                return iter_target_effort.min(acc_target_effort);
            }
        }

        unreachable!("above loop must return at some point")
    }
}

#[cfg(test)]
fn batch_for_samp_size(samp_size: usize, total_count: usize) -> usize {
    assert!(
        total_count >= samp_size,
        "batch_for_samp_size >>> total_count={total_count} must be >= samp_size={samp_size}"
    );
    total_count / samp_size
}

#[cfg(test)]
#[cfg(feature = "_bench")]
/// cargo test -r --lib --all-features -- load::busy_work_simple::validate_latency --nocapture --test-threads=1
mod validate_latency {
    use super::*;
    use crate::{
        FpSeconds, median_batch_latency, rel_approx_eq_fpsecs, test_support::AbsRelDiffFpSecs,
    };

    fn run(dur: Duration, count: usize, samp_size: usize) -> (FpSeconds, FpSeconds) {
        _ = env_logger::try_init();

        let batch = batch_for_samp_size(samp_size, count);
        let n_batches = count.div_ceil(batch);

        let effort = BusyWork::calibrate(dur);
        let f = BusyWork::fun(effort);

        let latency_fpsecs = median_batch_latency(f, batch, n_batches);
        let dur_fpsecs: FpSeconds = dur.into();
        let rel_diff = dur_fpsecs.abs_rel_diff_fpsecs(latency_fpsecs);
        println!(
            "dur={:?}, effort={}, dur_fpsecs={:?}, latency_fpsecs={:?}, rel_diff={}",
            dur, effort, dur_fpsecs, latency_fpsecs, rel_diff
        );
        (dur_fpsecs, latency_fpsecs)
    }

    #[test]
    fn test_busy_work_new_zero() {
        let dur = Duration::ZERO;
        const SAMP_SIZE: usize = 100;
        let count = 100_000_000;
        let (_, latency_fpsecs) = run(dur, count, SAMP_SIZE);
        assert!(
            latency_fpsecs > FpSeconds::ZERO,
            "zero-target calibration must produce non-zero work"
        );
    }

    //=== too small for proper calibration
    // #[test]
    // fn test_busy_work_new_1_nano() {
    //     const EPSILON: f64 = 0.70;
    //     const SAMP_SIZE: usize = 100;
    //     let dur = Duration::from_nanos(1);
    //     let count = 100_000_000;
    //     let (dur_secs, latency_secs) = run(dur, count, SAMP_SIZE);
    //     rel_approx_eq_fpsecs!(dur_secs, latency_secs, EPSILON);
    // }

    #[test]
    fn test_busy_work_new_100_nano() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 100;
        let dur = Duration::from_nanos(100);
        let count = 10_000_000;
        let (dur_secs, latency_secs) = run(dur, count, SAMP_SIZE);
        rel_approx_eq_fpsecs!(dur_secs, latency_secs, EPSILON);
    }

    #[test]
    fn test_busy_work_new_1_micro() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 20;
        let dur = Duration::from_micros(1);
        let count = 200_000;
        let (dur_secs, latency_secs) = run(dur, count, SAMP_SIZE);
        rel_approx_eq_fpsecs!(dur_secs, latency_secs, EPSILON);
    }

    // cargo test -r --lib --all-features -- load::busy_work_simple::validate_latency::test_busy_work_new_1_milli --nocapture --test-threads=1
    #[test]
    fn test_busy_work_new_1_milli() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 20;
        let dur = Duration::from_millis(1);
        let count = 200;
        let (dur_secs, latency_secs) = run(dur, count, SAMP_SIZE);
        rel_approx_eq_fpsecs!(dur_secs, latency_secs, EPSILON);
    }

    // cargo test -r --lib --all-features -- load::busy_work_simple::validate_latency::test_busy_work_new_10_milli --nocapture --test-threads=1
    #[test]
    fn test_busy_work_new_10_milli() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 10;
        let dur = Duration::from_millis(10);
        let count = 50;
        let (dur_secs, latency_secs) = run(dur, count, SAMP_SIZE);
        rel_approx_eq_fpsecs!(dur_secs, latency_secs, EPSILON);
    }

    // cargo test -r --lib --all-features -- load::busy_work_simple::validate_latency::test_busy_work_new_50_milli --nocapture --test-threads=1
    #[test]
    fn test_busy_work_new_50_milli() {
        const EPSILON: f64 = 0.05;
        const SAMP_SIZE: usize = 1;
        let dur = Duration::from_millis(50);
        let count = 20;
        let (dur_secs, latency_secs) = run(dur, count, SAMP_SIZE);
        rel_approx_eq_fpsecs!(dur_secs, latency_secs, EPSILON);
    }
}

#[cfg(test)]
#[cfg(feature = "_bench")]
// cargo test -r --lib --all-features -- load::busy_work_simple::validate_ratio --nocapture --test-threads=1
//
/// Test whether two busy work functions produce latencies that are proportional to the ratio of their
/// `effort` attributes. Checking is based on the cumulative latencies over a number of `repeats`.
mod validate_ratio {
    use super::*;
    use crate::{BenchCfg, LatencyUnit::SubSec, duo};
    use basic_stats::{dev_utils::ApproxEq, rel_approx_eq};

    fn run(dur1: Duration, ratio: f64, count: usize, samp_size: usize) -> f64 {
        _ = env_logger::try_init();

        let effort1 = BusyWork::calibrate(dur1);
        let effort2 = (effort1 as f64 * ratio).round() as u32;
        let f1 = BusyWork::fun(effort1);
        let f2 = BusyWork::fun(effort2);

        let batch = batch_for_samp_size(samp_size, count);
        let cfg = BenchCfg::default()
            .with_recording_unit(SubSec(11))
            .with_warmup_millis(100);

        let out = duo::bench_run_arg_cfg_b(&cfg, f1, f2, RunLength::Count(count), batch);

        let latency_ratio = 1.0 / out.ratio_medians_f1_f2();
        let rel_diff = latency_ratio.abs_rel_diff(ratio);

        println!(
            "out_f1().median()={:?}, out_f2().median()={:?}",
            out.out_f1().median(),
            out.out_f2().median()
        );

        println!(
            "dur1={dur1:?}, effort1={effort1}, effort2={effort2}, target_ratio={ratio}, latency_ratio={latency_ratio}, rel_diff={rel_diff}",
        );

        latency_ratio
    }

    const RATIO: f64 = 1.10;

    // cargo test -r --lib --all-features -- load::busy_work_simple::validate_ratio::test_busy_work_ratio_10_nano --nocapture --test-threads=1
    #[test]
    // too small for proper calibration
    fn test_busy_work_ratio_10_nano() {
        const EPSILON: f64 = 0.10; // overtakes the ratio relative difference
        const SAMP_SIZE: usize = 1000;
        let dur1 = Duration::from_nanos(10);
        let count = 10_000_000;
        let latency_ratio = run(dur1, RATIO, count, SAMP_SIZE);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }

    // cargo test -r --lib --all-features -- load::busy_work_simple::validate_ratio::test_busy_work_ratio_100_nano --nocapture --test-threads=1
    #[test]
    fn test_busy_work_ratio_100_nano() {
        const EPSILON: f64 = 0.01;
        const SAMP_SIZE: usize = 1000;
        let dur1 = Duration::from_nanos(100);
        let count = 1_000_000;
        let latency_ratio = run(dur1, RATIO, count, SAMP_SIZE);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_1_micro() {
        const EPSILON: f64 = 0.01;
        const SAMP_SIZE: usize = 20;
        let dur1 = Duration::from_micros(1);
        let count = 100_000;
        let latency_ratio = run(dur1, RATIO, count, SAMP_SIZE);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_100_micro() {
        const EPSILON: f64 = 0.01;
        const SAMP_SIZE: usize = 20;
        let dur1 = Duration::from_millis(1);
        let count = 1000;
        let latency_ratio = run(dur1, RATIO, count, SAMP_SIZE);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_1_milli() {
        const EPSILON: f64 = 0.01;
        const SAMP_SIZE: usize = 20;
        let dur1 = Duration::from_millis(1);
        let count = 100;
        let latency_ratio = run(dur1, RATIO, count, SAMP_SIZE);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_10_milli() {
        const EPSILON: f64 = 0.01;
        const SAMP_SIZE: usize = 10;
        let dur1 = Duration::from_millis(10);
        let count = 30;
        let latency_ratio = run(dur1, RATIO, count, SAMP_SIZE);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }
}
