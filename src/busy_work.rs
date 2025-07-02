use crate::crate_utils::comb_sort;

use super::latency;
use std::{hint::black_box, time::Duration};

/// Invokes `f` `r*2 + 1` times and returns the median latency.
#[inline(always)]
fn latency_m(f: impl Fn(), r: u8) -> Duration {
    if r == 0 {
        return latency(&f);
    }

    let size = (2 * r + 1) as usize;
    let mut lats = Vec::<Duration>::with_capacity(size);

    for _ in 0..size {
        lats.push(latency(&f));
    }

    comb_sort(&mut lats);
    lats[r as usize]
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
/// Calls [`calibrate_busy_work_x`] with a predefined default `calibration_effort` of `200_000` and `r` value of 0 (1 run);
pub fn calibrate_busy_work(target_latency: Duration) -> u32 {
    const CALIBRATION_EFFORT: u32 = 200_000;
    const R: u8 = 0;
    calibrate_busy_work_x(target_latency, CALIBRATION_EFFORT, R)
}

/// Returns an estimate of the number of iterations required for [`busy_work`] to have latency `target_latency`.
///
/// # Arguments
/// - `target_latency`: target latency.
/// - `calibration_effort`: the number of iterations executed during calibration.
/// - `r`: the calibration is run `r*2 + 1` times and the median value is returned.
pub fn calibrate_busy_work_x(target_latency: Duration, calibration_effort: u32, r: u8) -> u32 {
    let latency = latency_m(|| busy_work(calibration_effort), r);
    (target_latency.as_nanos() * calibration_effort as u128 / latency.as_nanos()) as u32
}
