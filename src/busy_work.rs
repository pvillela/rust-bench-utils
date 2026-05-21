use super::latency;
use sha2::{Digest, Sha256};
use std::{hint::black_box, time::Duration};

#[derive(Clone, Copy)]
/// Produces a closure which does a significant amount of computation to support validation of benchmarking frameworks.
/// Gated by feature **"busy_work"**.
///
/// The closure executes a work function whose latency is controlled by the `effort` value encapsulated in this struct.
///
/// Given a desired target latency, the latency of the resulting closure is not as reliable as using
/// `|| thread::sleep(target_latency)`. However, the busy work closure is a more realistic sythetic load as its latency
/// is the result of computations. Nonetheless, the ratio of the latencies of two closures created from two [`BusyWork`]
/// instances are reliably proportional to the ratios of the respective `effort` attributes, the more so the higher the
/// sample size.
pub struct BusyWork {
    effort: u32,
}

impl BusyWork {
    /// Constructs a new intance with the provided `effort`.
    pub fn from_effort(effort: u32) -> Self {
        BusyWork { effort }
    }

    /// Constructs a new intance whose resulting closure has a given target latency, using an iterative process
    /// to estimate the target effort.
    ///
    /// The estimation process executes a work function multiple times with increasing effort values.
    ///
    /// # Arguments
    ///
    /// `target_latency` - the target latency of [`Self::closure`].
    /// `calibration_budget` - limits the duration of the iterative process.
    pub fn from_latency_and_budget(target_latency: Duration, calibration_budget: Duration) -> Self {
        let effort = Self::effort_from_latency_and_budget(target_latency, calibration_budget);
        Self::from_effort(effort)
    }

    /// Constructs a new intance whose resulting closure has a given target latency.
    ///
    /// Calls [`Self::from_latency_and_budget`] with `calibration_budget = max(target_latency * 2, 1 millisecond)`.
    pub fn new(target_latency: Duration) -> Self {
        let calibration_budget = Duration::from_millis(1).max(target_latency / 2);
        Self::from_latency_and_budget(target_latency, calibration_budget)
    }

    /// Constructs a new intance whose resulting closure has a given target latency, using a one-shot process
    /// to estimate the target effort.
    ///
    /// The estimation process executes a work function with `effort = calibration_effort` and does a proportionality
    /// calculation.
    pub fn from_latency_and_calibration_effort(
        target_latency: Duration,
        calibration_effort: u32,
    ) -> Self {
        let effort =
            Self::effort_from_latency_and_calibration_effort(target_latency, calibration_effort);
        Self::from_effort(effort)
    }

    /// The number of work iterations performed by [`Self::closure`].
    pub fn effort(&self) -> u32 {
        self.effort
    }

    /// Closure which does a significant amount of computation to support validation of benchmarking frameworks.
    ///
    /// The documentation for [`BusyWork`] and its constructor methods describes how to control the closure's
    /// latency.
    pub fn fun(&self) -> impl Fn() + use<> {
        let effort = self.effort;
        move || Self::work(effort)
    }

    #[inline(always)]
    /// Does a significant amount of computation and its latency is controlled by `effort`.
    pub fn work(effort: u32) {
        let effort = black_box(effort);
        let seed = black_box(0_u64);
        let buf = seed.to_be_bytes();
        let mut hasher = Sha256::new();
        for _ in 0..effort {
            hasher.update(buf);
        }
        let hash = hasher.finalize();
        black_box(hash);
    }

    // This was the default used by the old `calibrate_busy_work` function.
    // const DEFAULT_CALIBRATION_EFFORT: u32 = 200_000;

    // Below was previously the function `calibrate_busy_work_x`

    /// Estimates the `effort` required for the resulting closure to have the `target_latency`, using a
    /// one-shot process that executes a work function with `effort = calibration_effort` and does a
    /// proportionality calculation.
    fn effort_from_latency_and_calibration_effort(
        target_latency: Duration,
        calibration_effort: u32,
    ) -> u32 {
        let busy_work = BusyWork::from_effort(calibration_effort).fun();
        let latency = latency(busy_work);
        (target_latency.as_nanos() * calibration_effort as u128 / latency.as_nanos()) as u32
    }

    /// Estimates the `effort` required for the resulting closure to have the `target_latency`, using
    /// an iterative process.
    /// `calibration_budget` limits the duration of the iterative process.
    fn effort_from_latency_and_budget(
        target_latency: Duration,
        calibration_budget: Duration,
    ) -> u32 {
        let mut acc_latency = Duration::from_nanos(0);

        for i in 1.. {
            let iter_effort = 2u32.pow(i - 1);
            let iter_latency = latency(|| Self::work(iter_effort));
            acc_latency += iter_latency;

            if iter_latency >= calibration_budget / 2 || acc_latency >= calibration_budget {
                // Estimate of target effort based on latest iteration.
                let iter_target_effort = (target_latency.as_nanos() * iter_effort as u128
                    / iter_latency.as_nanos()) as u32;

                // Estimate of target effort based on weighted average of the estimated target efforts
                // for all iterations.
                let acc_effort = iter_effort * 2 - 1;
                let acc_target_effort = (target_latency.as_nanos() * acc_effort as u128
                    / acc_latency.as_nanos()) as u32;

                // The last iteration should have been the most efficient due to previous warming;
                // if that's not the case, returns the weighted average estimated target effort.
                return iter_target_effort.min(acc_target_effort);
            }
        }

        unreachable!("above loop must return at some point")
    }
}

#[cfg(test)]
#[cfg(feature = "_test_support")]
/// cargo test --package bench_utils --lib --all-features -- busy_work::test --nocapture
mod core_tests {
    use super::*;
    use crate::latency;
    use basic_stats::{approx_eq, dev_utils::ApproxEq, rel_approx_eq};

    fn run(dur: Duration) -> (f64, f64) {
        let f = BusyWork::new(dur).fun();
        let latency_secs = latency(f).as_secs_f64();
        let dur_secs = dur.as_secs_f64();
        let rel_diff = dur_secs.abs_rel_diff(latency_secs);
        println!(
            "dur={:?}, dur_secs={}, latency_secs={}, rel_diff={}",
            dur, dur_secs, latency_secs, rel_diff
        );
        (dur_secs, latency_secs)
    }

    #[test]
    fn test_busy_work_new_zero() {
        const EPSILON: f64 = 0.005;
        let dur = Duration::ZERO;
        let (dur_secs, latency_secs) = run(dur);
        approx_eq!(dur_secs, latency_secs, EPSILON);
    }

    #[test]
    fn test_busy_work_new_1_nano() {
        const EPSILON: f64 = 2.0;
        let dur = Duration::from_nanos(1);
        let (dur_secs, latency_secs) = run(dur);
        rel_approx_eq!(dur_secs, latency_secs, EPSILON);
    }

    #[test]
    fn test_busy_work_new_1_micro() {
        const EPSILON: f64 = 0.75;
        let dur = Duration::from_micros(1);
        let (dur_secs, latency_secs) = run(dur);
        rel_approx_eq!(dur_secs, latency_secs, EPSILON);
    }

    #[test]
    fn test_busy_work_new_1_milli() {
        const EPSILON: f64 = 0.25;
        let dur = Duration::from_millis(1);
        let (dur_secs, latency_secs) = run(dur);
        rel_approx_eq!(dur_secs, latency_secs, EPSILON);
    }

    #[test]
    fn test_busy_work_new_50_millis() {
        const EPSILON: f64 = 0.25;
        let dur = Duration::from_millis(50);
        let (dur_secs, latency_secs) = run(dur);
        rel_approx_eq!(dur_secs, latency_secs, EPSILON);
    }
}

#[cfg(test)]
#[cfg(feature = "_test_support")]
// cargo test -r --package bench_utils --lib --all-features -- busy_work::ratio_tests --nocapture
//
/// Test whether two busy work functions produce latencies that are proportional to the ratio of their
/// `effort` attributes. Checking is based on the cumulative latencies over a number of `repeats`.
mod ratio_tests {
    use super::*;
    use crate::latency;
    use basic_stats::{dev_utils::ApproxEq, rel_approx_eq};

    fn run(dur1: Duration, ratio: f64, repeats: u32) -> f64 {
        let bw1 = BusyWork::new(dur1);
        let effort1 = bw1.effort();
        let effort2 = (effort1 as f64 * ratio) as u32;

        let f1 = bw1.fun();
        let f2 = BusyWork::from_effort(effort2).fun();

        let mut latency_secs1 = 0.0;
        let mut latency_secs2 = 0.0;

        for _ in 0..repeats {
            latency_secs1 += latency(&f1).as_secs_f64();
            latency_secs2 += latency(&f2).as_secs_f64();
        }

        let latency_ratio = latency_secs2 / latency_secs1;
        let rel_diff = latency_ratio.abs_rel_diff(ratio);

        println!(
            "dur1={:?}, latency_ratio={}, ratio={}, rel_diff={}",
            dur1, latency_ratio, ratio, rel_diff
        );

        latency_ratio
    }

    const RATIO: f64 = 1.10;

    #[test]
    fn test_busy_work_ratio_100_nano() {
        const EPSILON: f64 = 0.5; // not reliable at nano scale
        let dur1 = Duration::from_nanos(100);
        let repeats = 100_000;
        let latency_ratio = run(dur1, RATIO, repeats);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_100_micro() {
        const EPSILON: f64 = 0.50;
        let dur1 = Duration::from_micros(10);
        let repeats = 1_000;
        let latency_ratio = run(dur1, RATIO, repeats);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_1_milli() {
        const EPSILON: f64 = 0.05;
        let dur1 = Duration::from_millis(1);
        let repeats = 100;
        let latency_ratio = run(dur1, RATIO, repeats);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }

    #[test]
    fn test_busy_work_ratio_10_millis() {
        const EPSILON: f64 = 0.10;
        let dur1 = Duration::from_millis(10);
        let repeats = 10;
        let latency_ratio = run(dur1, RATIO, repeats);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }
}
