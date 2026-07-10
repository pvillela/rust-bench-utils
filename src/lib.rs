#![doc = include_str!("lib.md")]

//! # Quick start
//!
//! ```rust,no_run
#![doc = include_str!("../examples/doc_bench_run.rs")]
//! ```
//!
//! Two closures benchmarked together with custom configuration, status reporting, and batching.
//!
//! ```rust,no_run
#![doc = include_str!("../examples/doc_bench_run_duo.rs")]
//! ```
//!
//! Two closures benchmarked in parallel with custom configuration and batching.
//!
//! ```rust,no_run
#![doc = include_str!("../examples/doc_bench_run_parallel.rs")]
//! ```
//!
//! # Feature flags
//!
//! | Feature | Purpose |
//! |---------|---------|
//! | `default` | For access to all of the library's benchmarking functions and types.
//! | `load` | Enables synthetic loads: `fake_work(Duration)` (thread sleep) and arithmetic-loop CPU work via [`BusyWork`] |
//!
//! # Log-normal assumption
//!
//! The inferential statistics in this crate (Student's t, Welch's t) are computed
//! on `ln(latency)` rather than raw latency. This reflects the widely-supported
//! assumption that latency distributions are approximately log-normal. Under this
//! assumption `mean(ln(latency)) == ln(median(latency))`, so confidence intervals
//! and hypothesis tests on log-latencies translate directly to statements about
//! median latencies.

#![allow(clippy::new_without_default)]

mod bench_cfg;
mod bench_out;
mod bench_run;
mod comp;
mod latency;
mod summary_stats;

pub use bench_cfg::*;
pub use bench_out::*;
pub use bench_run::*;
pub use comp::*;
pub use latency::*;
pub use summary_stats::*;

pub mod duo;
pub mod multi;
pub mod status;

#[doc(hidden)]
pub mod dev_support;

#[cfg(feature = "load")]
pub mod load;

/// Structs and enums for confidence intervals and hypothesis tests.
pub mod stats_types {
    pub use basic_stats::core::{AcceptedHyp, AltHyp, Ci, HypTestResult, PositionWrtCi};
}

#[cfg(feature = "_bench")]
pub mod bench_support;

#[cfg(feature = "_test_support")]
pub mod test_support;
