mod bench_cfg;
mod bench_out;
mod comp;
mod fake_work;
mod latency;
mod status;
mod summary_stats;

pub use bench_cfg::*;
pub use bench_out::*;
pub use comp::*;
pub use fake_work::*;
pub use latency::*;
pub use status::*;
pub use summary_stats::*;

mod bench_run;
pub use bench_run::*;

/// Benchmark multiple closures together, executing each of them in every
/// benchmarking iteration.
///
/// The benchmarking functions in this module
/// produce a single [`BenchOut<K>`](crate::multi::BenchOut) that holds one
/// [`BenchOut`] per closure.
pub mod multi;

#[cfg(feature = "busy_work")]
mod busy_work;
#[cfg(feature = "busy_work")]
pub use busy_work::*;

/// Structs and enums for confidence intervals and hypothesis tests.
pub mod stats_types {
    pub use basic_stats::core::{AcceptedHyp, AltHyp, Ci, HypTestResult, PositionWrtCi};
}

/// Validates that the latency-measurement overhead per function execution is acceptable.
///
/// The function [`validate_latency_overhead`](crate::bench_support::validate_latency_overhead)
/// compares solo vs. grouped execution latencies to detect overhead from the measurement harness.
/// Requires feature `_bench`.
#[cfg(feature = "_bench")]
pub mod bench_support;

/// Lognormal sample generators, a [`StringWriter`](crate::test_support::StringWriter) for testing status output,
/// and constants for low/high log-standard-deviation.
/// Requires feature `_test_support`.
#[cfg(feature = "_test_support")]
pub mod test_support;
