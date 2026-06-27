//! Traits and macros for relative absolute difference.
//!
//! Gated by feature **"_test_support"**.

use crate::FpSeconds;
use basic_stats::dev_utils::ApproxEq;
use std::time::Duration;

/// Extension trait providing [`abs_rel_diff`](AbsRelDiffDur::abs_rel_diff) on [`Duration`].
///
/// Computes the absolute relative difference between two durations,
/// delegating to [`f64::abs_rel_diff`] on their second-resolution values.
pub trait AbsRelDiffDur {
    /// Computes the absolute value of the relative difference between `self` and `other`.
    ///
    /// Returns `|self - other| / max(self, other)`, or 0 when both are zero.
    fn abs_rel_diff_dur(self, other: Duration) -> f64;
}

impl AbsRelDiffDur for Duration {
    fn abs_rel_diff_dur(self, other: Duration) -> f64 {
        self.as_secs_f64().abs_rel_diff(other.as_secs_f64())
    }
}

/// Extension trait providing [`abs_rel_diff`](AbsRelDiffFpSecs::abs_rel_diff) on [`FpSeconds`].
///
/// Computes the absolute relative difference between two durations,
/// delegating to [`f64::abs_rel_diff`] on their second-resolution values.
pub trait AbsRelDiffFpSecs {
    /// Computes the absolute value of the relative difference between `self` and `other`.
    ///
    /// Returns `|self - other| / max(self, other)`, or 0 when both are zero.
    fn abs_rel_diff_fpsecs(self, other: FpSeconds) -> f64;
}

impl AbsRelDiffFpSecs for FpSeconds {
    fn abs_rel_diff_fpsecs(self, other: FpSeconds) -> f64 {
        self.as_f64().abs_rel_diff(other.as_f64())
    }
}

#[macro_use]
mod macros {
    /// Asserts that two durations are approximately equal within `epsilon` relative to their magnitudes.
    #[macro_export]
    macro_rules! rel_approx_eq_dur {
        ($a:expr, $b:expr, $epsilon:expr $(,)?) => {
            let rel_diff = $crate::test_support::AbsRelDiffDur::abs_rel_diff_dur($a, $b);
            if !basic_stats::dev_utils::ApproxEq::rel_approx_eq($a.as_secs_f64(), $b.as_secs_f64(), $epsilon) {
                panic!(
                    "assertion for relative approximate equality failed: left={:?}, right={:?}, rel_diff={}, epsilon={})",
                    $a, $b, rel_diff, $epsilon
                );
            }
        };
    }

    /// Asserts that two durations are approximately equal within `epsilon` relative to their magnitudes.
    #[macro_export]
    macro_rules! rel_approx_eq_fpsecs {
        ($a:expr, $b:expr, $epsilon:expr $(,)?) => {
            let rel_diff = $crate::test_support::AbsRelDiffFpSecs::abs_rel_diff_fpsecs($a, $b);
            if !basic_stats::dev_utils::ApproxEq::rel_approx_eq($a.as_f64(), $b.as_f64(), $epsilon) {
                panic!(
                    "assertion for relative approximate equality failed: left={:?}, right={:?}, rel_diff={}, epsilon={})",
                    $a, $b, rel_diff, $epsilon
                );
            }
        };
    }
}
