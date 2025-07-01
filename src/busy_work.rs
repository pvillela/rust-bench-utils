use super::latency;
use std::{hint::black_box, time::Duration};

/// Invokes `f` 3 times and returns the median latency.
#[inline(always)]
fn latency_m(f: impl Fn()) -> Duration {
    let mut lats = [latency(&f), latency(&f), latency(&f)];
    for k in (0..2).rev() {
        for i in 0..k {
            lats.swap(i, i + 1);
        }
    }
    lats[1]
}

/// Function that does a significant amount of computation to support validation of benchmarking frameworks.
/// `effort` is the number of iterations that determines the amount of work performed.
pub fn busy_work(effort: u32) {
    const M: f64 = 104729.; // 10,000th prime number
    const F: f64 = 0.141_592_653_589_793_23; // 1st 17 decimal digits of pi
    let extent = black_box(effort);
    let mut vf = F;
    for _ in 0..extent {
        vf = black_box(((M - 1. + vf) / M + F) / 2.); // always in the open interval (0.07, 1)
    }
    black_box(vf);
}

/// Returns an estimate of the number of iterations required for [`busy_work`] to have latency `target_latency`.
///
/// Calls [`calibrate_busy_work_x`] with a predefined default `calibration_effort` of `20_000`;
pub fn calibrate_busy_work(target_latency: Duration) -> u32 {
    const CALIBRATION_EFFORT: u32 = 20_000;
    calibrate_busy_work_x(target_latency, CALIBRATION_EFFORT)
}

/// Returns an estimate of the number of iterations required for [`busy_work`] to have latency `target_latency`.
///
/// `calibration_effort` is the number of iterations executed during calibration.
pub fn calibrate_busy_work_x(target_latency: Duration, calibration_effort: u32) -> u32 {
    let latency = latency_m(|| busy_work(calibration_effort));
    (target_latency.as_nanos() * calibration_effort as u128 / latency.as_nanos()) as u32
}
