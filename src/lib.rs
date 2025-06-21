mod bench_cfg;
mod bench_out;
mod comp;
mod fake_work;
mod latency;
mod summary_stats;

pub use bench_cfg::*;
pub use bench_out::*;
pub use comp::*;
pub use fake_work::*;
pub use latency::*;
pub use summary_stats::*;

#[cfg(feature = "_bench_run")]
mod bench_run;
#[cfg(feature = "_bench_run")]
pub use bench_run::*;

#[cfg(feature = "busy_work")]
mod busy_work;
#[cfg(feature = "busy_work")]
pub use busy_work::*;

/// Structs and enums for confidence intervals and hypothesis tests.
pub mod stats_types {
    pub use basic_stats::core::{AcceptedHyp, AltHyp, Ci, HypTestResult, PositionWrtCi};
}

#[cfg(test)]
#[cfg(feature = "_bench_run")]
pub mod test_support;
