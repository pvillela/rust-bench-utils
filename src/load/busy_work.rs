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
        move || Self::work(effort)
    }

    #[inline(always)]
    /// Does a significant amount of computation. Its latency is proportional to `effort`.
    fn work(effort: u32) {
        // const INNER_STRIDE: u64 = 64;
        const INNER_STRIDE: u64 = 1;
        const MULT: u64 = 6364136223846793005;
        const INC: u64 = 1442695040888963407;

        let mut state = 0_u64;
        let total = (effort as u64).wrapping_mul(INNER_STRIDE);
        for _ in 0..black_box(total) {
            state = black_box(state.wrapping_mul(MULT).wrapping_add(INC));
        }
        // black_box(state);
    }

    /// Estimates the `effort` required for the resulting closure to have the `target_latency`, using
    /// an iterative process.
    ///
    /// Calls [`calibrate_with_budget`](Self::calibrate_with_budget) using a default budget.
    ///
    /// # Panics
    ///
    /// Panics if the measured latency of any iteration is zero,
    /// which would cause a division-by-zero panic. This should not occur under
    /// normal conditions since [`BusyWork::work`] uses an arithmetic loop and always consumes
    /// measurable wall-clock time.
    pub fn calibrate(target_latency: Duration) -> u32 {
        let budget: RunLength = RunLength::Time(Duration::from_millis(10).max(target_latency));
        Self::calibrate_with_budget(target_latency, budget)
    }

    /// Estimates the `effort` required for the resulting closure to have the `target_latency`, using
    /// an iterative process.
    /// `calibration_budget` limits the length of the iterative process by time and/or count
    /// (= accumulated calibration effort).
    ///
    /// # Panics
    ///
    /// Panics if the measured latency of any iteration is zero,
    /// which would cause a division-by-zero panic. This should not occur under
    /// normal conditions since [`BusyWork::work`] uses an arithmetic loop and always consumes
    /// measurable wall-clock time.
    pub fn calibrate_with_budget(target_latency: Duration, budget: RunLength) -> u32 {
        let (budget_count, budget_dur) = budget.exec_count_and_duration();
        // let mut acc_latency = Duration::ZERO;
        let mut acc_effort: u32 = 0;

        // Warm-up
        for i in 1.. {
            let iter_effort = 2u32.pow(i - 1);
            let iter_latency = latency(|| Self::work(iter_effort));

            acc_effort += iter_effort;

            // Castings to f64 to avoid integer overflow or truncation to zero.
            if iter_latency >= budget_dur / 3
                || acc_effort as f64 >= budget_count as f64 * (2.0 / 3.0)
            {
                break;
            }
        }

        for i in 1.. {
            let iter_effort = 2u32.pow(i - 1);
            let iter_latency = latency(|| Self::work(iter_effort));

            // acc_latency += iter_latency;
            acc_effort += iter_effort;

            // Castings to f64 to avoid integer overflow or truncation to zero.
            if iter_latency >= budget_dur / 3
                // || acc_latency.as_secs_f64() >= budget_dur.as_secs_f64() * (2.0 / 3.0)
                || acc_effort as f64 >= budget_count as f64 * (2.0 / 3.0)
            {
                // Estimate of target effort based on latest iteration.
                let iter_target_effort = (target_latency.as_secs_f64() * iter_effort as f64
                    / iter_latency.as_secs_f64())
                .round() as u32;

                // Estimate of target effort based on weighted average of the estimated target efforts
                // for all iterations.
                // let acc_target_effort = (target_latency.as_secs_f64() * acc_effort as f64
                //     / acc_latency.as_secs_f64())
                // .round() as u32;

                //=== bad comment
                // The last iteration should have been the most efficient due to previous warming;
                // if that's not the case, returns the weighted average estimated target effort.
                // debug!(
                //     "calibrate_with_budget >>> iter_target_effort={iter_target_effort}, acc_target_effort={acc_target_effort}"
                // );
                // return iter_target_effort.min(acc_target_effort).max(1);
                return iter_target_effort.max(1);
                // return iter_target_effort.max(acc_target_effort).max(1);
            }
        }

        unreachable!("above loop must return at some point")
    }
}

#[cfg(test)]
#[cfg(feature = "_bench")]
/// cargo test -r --lib --all-features -- busy_work::validate_latency --nocapture --test-threads=1
mod validate_latency {
    use super::*;
    use crate::{FpSeconds, latency, rel_approx_eq_fpsecs, test_support::AbsRelDiffFpSecs};

    fn run(dur: Duration) -> (FpSeconds, FpSeconds) {
        _ = env_logger::try_init();

        let effort = BusyWork::calibrate(dur);
        let f = BusyWork::fun(effort);
        let latency_fpsecs: FpSeconds = latency(f).into();
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
        let (_, latency_fpsecs) = run(dur);
        assert!(
            latency_fpsecs > FpSeconds::ZERO,
            "zero-target calibration must produce non-zero work"
        );
    }

    #[test]
    fn test_busy_work_new_1_nano() {
        const EPSILON: f64 = 2.0;
        let dur = Duration::from_nanos(1);
        let (dur_secs, latency_secs) = run(dur);
        rel_approx_eq_fpsecs!(dur_secs, latency_secs, EPSILON);
    }

    #[test]
    fn test_busy_work_new_100_nano() {
        const EPSILON: f64 = 0.15;
        let dur = Duration::from_nanos(100);
        let (dur_secs, latency_secs) = run(dur);
        rel_approx_eq_fpsecs!(dur_secs, latency_secs, EPSILON);
    }

    #[test]
    fn test_busy_work_new_1_micro() {
        const EPSILON: f64 = 0.15;
        let dur = Duration::from_micros(1);
        let (dur_secs, latency_secs) = run(dur);
        rel_approx_eq_fpsecs!(dur_secs, latency_secs, EPSILON);
    }

    // cargo test -r --lib --all-features -- busy_work::validate_latency::test_busy_work_new_1_milli --nocapture --test-threads=1
    #[test]
    fn test_busy_work_new_1_milli() {
        const EPSILON: f64 = 0.05;
        let dur = Duration::from_millis(1);
        let (dur_secs, latency_secs) = run(dur);
        rel_approx_eq_fpsecs!(dur_secs, latency_secs, EPSILON);
    }

    // cargo test -r --lib --all-features -- busy_work::validate_latency::test_busy_work_new_10_milli --nocapture --test-threads=1
    #[test]
    fn test_busy_work_new_10_milli() {
        const EPSILON: f64 = 0.05;
        let dur = Duration::from_millis(10);
        let (dur_secs, latency_secs) = run(dur);
        rel_approx_eq_fpsecs!(dur_secs, latency_secs, EPSILON);
    }

    // cargo test -r --lib --all-features -- busy_work::validate_latency::test_busy_work_new_50_milli --nocapture --test-threads=1
    #[test]
    fn test_busy_work_new_50_milli() {
        const EPSILON: f64 = 0.05;
        let dur = Duration::from_millis(50);
        let (dur_secs, latency_secs) = run(dur);
        rel_approx_eq_fpsecs!(dur_secs, latency_secs, EPSILON);
    }
}

#[cfg(test)]
#[cfg(feature = "_bench")]
// cargo test -r --lib --all-features -- busy_work::validate_ratio --nocapture
//
/// Test whether two busy work functions produce latencies that are proportional to the ratio of their
/// `effort` attributes. Checking is based on the cumulative latencies over a number of `repeats`.
mod validate_ratio {
    use super::*;
    use crate::latency;
    use basic_stats::{dev_utils::ApproxEq, rel_approx_eq};

    fn run(dur1: Duration, ratio: f64, repeats: u32) -> f64 {
        let effort1 = BusyWork::calibrate(dur1);
        let effort2 = (effort1 as f64 * ratio) as u32;
        let mut f1 = BusyWork::fun(effort1);
        let mut f2 = BusyWork::fun(effort2);

        let mut latency1 = Duration::ZERO;
        let mut latency2 = Duration::ZERO;

        for _ in 0..repeats {
            latency1 += latency(&mut f1);
            latency2 += latency(&mut f2);
        }

        let latency_ratio = latency2.as_secs_f64() / latency1.as_secs_f64();
        let rel_diff = latency_ratio.abs_rel_diff(ratio);

        println!(
            "dur1={:?}, latency_ratio={}, ratio={}, rel_diff={}",
            dur1, latency_ratio, ratio, rel_diff
        );

        latency_ratio
    }

    const RATIO: f64 = 1.10;

    #[test]
    fn test_busy_work_ratio_10_nano() {
        const EPSILON: f64 = 0.5; // not reliable at nano scale
        let dur1 = Duration::from_nanos(10);
        let repeats = 100_000;
        let latency_ratio = run(dur1, RATIO, repeats);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_100_nano() {
        const EPSILON: f64 = 0.10; // not reliable at nano scale
        let dur1 = Duration::from_nanos(100);
        let repeats = 100_000;
        let latency_ratio = run(dur1, RATIO, repeats);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_100_micro() {
        const EPSILON: f64 = 0.05;
        let dur1 = Duration::from_micros(10);
        let repeats = 1_000;
        let latency_ratio = run(dur1, RATIO, repeats);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_1_milli() {
        const EPSILON: f64 = 0.02;
        let dur1 = Duration::from_millis(1);
        let repeats = 100;
        let latency_ratio = run(dur1, RATIO, repeats);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_10_milli() {
        const EPSILON: f64 = 0.02;
        let dur1 = Duration::from_millis(10);
        let repeats = 10;
        let latency_ratio = run(dur1, RATIO, repeats);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }
}
