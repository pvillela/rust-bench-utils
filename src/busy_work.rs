use crate::crate_utils::comb_sort;

use super::latency;
use std::{hint::black_box, time::Duration};

/// Invokes `f` `R` times and returns the median latency.
#[inline(always)]
fn latency_m<const R: usize>(f: impl Fn()) -> Duration {
    if R <= 1 {
        return latency(&f);
    }

    let mut lats = [Duration::new(0, 0); R];

    for i in 0..R {
        lats[i] = latency(&f);
    }

    comb_sort(&mut lats);

    if R % 2 == 1 {
        lats[R / 2]
    } else {
        let m1 = lats[R / 2 - 1].as_nanos();
        let m2 = lats[R / 2].as_nanos();
        let m = (m1 + m2) / 2;
        Duration::from_nanos(m as u64)
    }
}

/// Function that does a significant amount of computation to support validation of benchmarking frameworks.
/// `effort` is the number of iterations that determines the amount of work performed.
pub fn busy_work(effort: u32) {
    const F: f64 = 0.5;
    let extent = black_box(effort);
    let mut vf = F;
    for _ in 0..extent {
        vf = black_box(((1. + vf) * (1. + vf)).fract());
    }
    black_box(vf);
}

/// Returns an estimate of the number of iterations required for [`busy_work`] to have latency `target_latency`.
///
/// Calls [`calibrate_busy_work_x`] with predefined default `calibration_effort` and `R` values.
pub fn calibrate_busy_work(target_latency: Duration) -> u32 {
    const CALIBRATION_EFFORT: u32 = 100_000;
    const R: usize = 0;
    calibrate_busy_work_x::<R>(target_latency, CALIBRATION_EFFORT)
}

/// Returns an estimate of the number of iterations required for [`busy_work`] to have latency `target_latency`.
///
/// # Generic parameters:
/// - `R`: the number of times the calibration is run. The median calibration is returned. An extremely high value
///   for `R` will cause a stack overflow.
///
/// # Arguments
/// - `target_latency`: target latency.
/// - `calibration_effort`: the number of iterations executed during calibration.
pub fn calibrate_busy_work_x<const R: usize>(
    target_latency: Duration,
    calibration_effort: u32,
) -> u32 {
    let latency = latency_m::<R>(|| busy_work(calibration_effort));
    (target_latency.as_nanos() * calibration_effort as u128 / latency.as_nanos()) as u32
}
