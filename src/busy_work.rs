use super::latency;
use sha2::{Digest, Sha256};
use std::{hint::black_box, time::Duration};

/// Function that does a significant amount of computation to support validation of benchmarking frameworks.
/// `effort` is the number of iterations that determines the amount of work performed.
/// Gated by feature **"busy_work"**.
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
/// of `target_latency`.
/// Gated by feature **"busy_work"**.
///
/// Calls [`calibrate_busy_work_x`] with a predefined `calibration_effort`.
pub fn calibrate_busy_work(target_latency: Duration) -> u32 {
    const CALIBRATION_EFFORT: u32 = 200_000;
    calibrate_busy_work_x(CALIBRATION_EFFORT, target_latency)
}

/// Returns an estimate of the number of iterations required for [`busy_work`] to have a latency
/// of `target_latency`. `calibration_effort` is the number of iterations executed during calibration.
/// Gated by feature **"busy_work"**.
pub fn calibrate_busy_work_x(calibration_effort: u32, target_latency: Duration) -> u32 {
    let latency = latency(|| busy_work(calibration_effort));
    (target_latency.as_nanos() * calibration_effort as u128 / latency.as_nanos()) as u32
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_busy_work_minimal() {
        // Should not panic with minimal effort
        busy_work(1);
    }

    #[test]
    fn test_busy_work_zero() {
        // Should not panic with zero effort
        busy_work(0);
    }

    #[test]
    fn test_calibrate_busy_work_x() {
        // Calibration should return a positive effort value
        let effort = calibrate_busy_work_x(100_000, Duration::from_nanos(1000));
        assert!(effort > 0);
    }

    #[test]
    fn test_calibrate_busy_work() {
        // Default calibration should return a positive effort value
        let effort = calibrate_busy_work(Duration::from_nanos(2000));
        assert!(effort > 0);
    }
}
