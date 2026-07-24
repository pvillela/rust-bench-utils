mod abs_rel_diff;
mod miscellaneous;
mod test_finder;

pub use abs_rel_diff::*;
pub use miscellaneous::*;
pub use test_finder::*;

// Crate-internal Monte Carlo harness for calibrating the `mean_check` verdict thresholds. Gated
// `#[cfg(test)]` (this crate's own test build only) because it reuses one `BenchOut` across trials
// via the crate-private `reset()`, so it cannot live in an external example.
#[cfg(test)]
mod mean_check_calibration;
