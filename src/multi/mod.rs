//! Benchmark multiple closures together, executing each of them in every benchmarking iteration.
//!
//! The benchmarking functions in this module produce a single [`BenchOut<K>`] that holds one
//! [`crate::BenchOut`] per closure.

mod bench_out;
mod bench_run;
mod latency_src;

pub use bench_out::*;
pub use bench_run::*;
pub use latency_src::*;
