use super::latency;
use sha2::{Digest, Sha256};
use std::{hint::black_box, time::Duration};

#[derive(Clone, Copy)]
/// Produces a closure which does a significant amount of computation to support validation of benchmarking frameworks.
/// Gated by feature **"busy_work"**.
///
/// The closure executes a work function whose latency is controlled by the `effort` value encapsulated in this struct.
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
mod test {
    use super::*;

    #[test]
    fn test_busy_work_minimal() {
        // Should not panic with minimal effort
        BusyWork::from_effort(1).fun()();
    }

    #[test]
    fn test_busy_work_zero() {
        // Should not panic with zero effort
        BusyWork::from_effort(0).fun()();
    }

    #[test]
    fn test_calibrate_busy_work_x() {
        // Calibration should return a positive effort value
        let effort = BusyWork::effort_from_latency_and_calibration_effort(
            Duration::from_nanos(1000),
            100_000,
        );
        assert!(effort > 0);
    }

    #[test]
    fn test_calibrate_busy_work() {
        // Default calibration should return a positive effort value
        let effort = BusyWork::new(Duration::from_nanos(2000)).effort();
        assert!(effort > 0);
    }
}
