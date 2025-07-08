mod bench_cfg;
mod bench_out;
mod busy_work;
mod comp_out;
mod fake_work;
mod latency;
mod summary_stats;

pub(crate) mod crate_utils;

pub use bench_cfg::*;
pub use bench_out::*;
pub use busy_work::*;
pub use comp_out::*;
pub use fake_work::*;
pub use latency::*;
pub use summary_stats::*;

#[cfg(feature = "_bench_run")]
mod bench_run;
#[cfg(feature = "_bench_run")]
pub use bench_run::*;

/// Structs and enums for confidence intervals and hypothesis tests.
pub mod stats_types {
    pub use basic_stats::core::{AcceptedHyp, AltHyp, Ci, HypTestResult, PositionWrtCi};
}

#[cfg(test)]
#[cfg(feature = "_bench_run")]
pub mod test_support;
