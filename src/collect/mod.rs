//! Functionality to colect benchmark output.

#[cfg(feature = "_bench_one")]
mod bench_one;

mod bench_out;
mod comp;
mod summary_stats;

#[cfg(feature = "_bench_one")]
pub use bench_one::*;

pub use bench_out::*;
pub use comp::*;
pub use summary_stats::*;
