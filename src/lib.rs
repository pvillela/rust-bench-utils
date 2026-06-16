#![doc = include_str!("lib.md")]

//! # Quick start
//!
//! ```rust
//! use bench_utils::{bench_run, BenchCfg, RunLength};
//!
//! // Benchmark a no-op closure for 1000 iterations with default configuration.
//! let out = bench_run(|| {}, RunLength::Count(1000));
//! println!("median: {:?}", out.median());
//! ```
//!
//! With a custom configuration and two closures benchmarked together:
//!
//! ```rust,no_run
//! use bench_utils::{BenchCfg, RunLength};
//! use bench_utils::multi::{bench_run_arg_cfg, LatencySrc2};
//! use std::time::Duration;
//!
//! let cfg = BenchCfg::default()
//!     .with_warmup_millis(500);
//!
//! let f1: fn() = || std::thread::sleep(Duration::from_micros(10));
//! let f2: fn() = || std::thread::sleep(Duration::from_micros(20));
//!
//! let out = bench_run_arg_cfg(
//!     &cfg,
//!     &mut LatencySrc2::new(f1, f2),
//!     RunLength::Time(Duration::from_secs(1)),
//! );
//! println!("n = {}, medians = {:?}", out.n(), out.medians());
//! ```
//!
//! # Feature flags
//!
//! | Feature | Purpose |
//! |---------|---------|
//! | `default` | For access to all of the library's benchmarking functions and types.
//! | `busy_work` | Enables synthetic loads using SHA-256-based CPU work via [`BusyWork`] (uses `sha2` crate) |
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
mod comp;
mod latency;
mod summary_stats;

pub use bench_cfg::*;
pub use bench_out::*;
pub use comp::*;
pub use latency::*;
pub use summary_stats::*;

mod bench_run;
pub use bench_run::*;

pub mod duo;
pub mod multi;
pub mod status;

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
