//! Module that provides functions to simulate work, used to support the validation of benchmarking frameworks.

use super::latency;
use sha2::{Digest, Sha256};
use std::{hint::black_box, thread, time::Duration};

/// Function that sleeps to simulate work to support validation of benchmarking frameworks.
pub fn fake_work(target_latency: Duration) {
    thread::sleep(target_latency);
}

/// Function that does a significant amount of computation to support validation of benchmarking frameworks.
/// `effort` is the number of iterations that determines the amount of work performed.
pub fn busy_work(effort: u32) {
    let extent = black_box(effort);
    let seed = black_box(0_u64);
    let buf = seed.to_be_bytes();
    let mut hasher = Sha256::new();
    for _ in 0..extent {
        hasher.update(buf);
    }
    let hash = hasher.finalize();
    black_box(hash);
}

/// Returns an estimate of the number of iterations required for [`busy_work`] to have a latency
/// of `target_micros`.
///
/// Calls [`calibrate_busy_work_x`] with a predefined `calibration_effort`;
pub fn calibrate_busy_work(target_latency: Duration) -> u32 {
    const CALIBRATION_EFFORT: u32 = 200_000;
    calibrate_busy_work_x(CALIBRATION_EFFORT, target_latency)
}

/// Returns an estimate of the number of iterations required for [`busy_work`] to have a latency
/// of `target_micros`. `calibration_effort` is the number of iterations executed during calibration.
pub fn calibrate_busy_work_x(calibration_effort: u32, target_latency: Duration) -> u32 {
    let latency = latency(|| busy_work(calibration_effort));
    (target_latency.as_nanos() * calibration_effort as u128 / latency.as_nanos()) as u32
}
