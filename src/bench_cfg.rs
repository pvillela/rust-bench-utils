use crate::{LatencyUnit, latency};
use basic_stats::aok::AokValue;
use log::debug;
use std::{
    ops::{Add, Div},
    time::Duration,
};

/// Specifies how long a benchmark should run for. Encapsulates a target number of iterations for the benchmark to run
/// and a time duration. The benchmark run length can be set as a number of iterations, a time duration, or
/// a number of iterations with a timeout duration.
#[derive(Debug, Clone, Copy)]
pub enum RunLength {
    /// Run for a fixed number of iterations.
    Count(u64),
    /// Run for a fixed duration.
    Duration(Duration),
    /// Run for a fixed number of iterations, but stop early if the given duration is exceeded.
    CountWithTimeout(u64, Duration),
}

impl RunLength {
    /// Returns both the number of iterations and time duration specified for the benchmark to run.
    ///
    /// The benchmark ends when the specified number of iterations is reached (or exceeded)
    /// or when the time duration is reached (or exceeded), whichever comes first.
    pub fn get_exec_count_and_duration(&self) -> (u64, Duration) {
        match self {
            Self::Count(count) => (*count, Duration::MAX),
            Self::Duration(duration) => (u64::MAX, *duration),
            Self::CountWithTimeout(count, duration) => (*count, *duration),
        }
    }

    /// Estimated number of iterations.
    pub fn estimated_count(&self, execs_per_milli: f64) -> u64 {
        match self {
            Self::Count(count) => *count,
            Self::Duration(duration) => {
                (duration.as_millis() as f64 * execs_per_milli).ceil() as u64
            }
            Self::CountWithTimeout(count, duration) => {
                let count_from_duration =
                    (duration.as_millis() as f64 * execs_per_milli).ceil() as u64;
                *count.min(&count_from_duration)
            }
        }
    }

    /// Estimated run duration.
    pub fn estimated_duration(&self, execs_per_milli: f64) -> Duration {
        match self {
            Self::Count(count) => {
                Duration::from_millis((*count as f64 / execs_per_milli).ceil() as u64)
            }
            Self::Duration(duration) => *duration,
            Self::CountWithTimeout(count, duration) => {
                let duration_from_count =
                    Duration::from_millis((*count as f64 / execs_per_milli).ceil() as u64);
                *duration.min(&duration_from_count)
            }
        }
    }
}

/// Global benchmark configuration.
///
/// Encapsulates the following data:
/// - `warmup_millis`: warm-up duration in milliseconds
/// - `recording_unit`: time unit for latency recording
/// - `sigfig`: as data is stored in an [HDR (high dynamic range) histogram](https://docs.rs/hdrhistogram/latest/hdrhistogram/index.html),
///   this is the number of significant decimal digits (of `recording_unit`) to which the histogram will maintain
///   value resolution and separation
/// - `status_millis`: milliseconds between status reports during bench execution
/// - `panic_on_error`: if set to `true`, library functions that don't return a [`Result`] should panic upon
///   encountering an error condition; when set to `false`, instead of panicking, functions should return a
///   tainted value, i.e., `NaN` or a data structure that has `NaN` in one or more fields.
#[derive(Debug, Clone)]
pub struct BenchCfg {
    warmup_millis: u64,
    recording_unit: LatencyUnit,
    sigfig: u8,
    status_millis: u64,
    panic_on_error: bool,
}

impl BenchCfg {
    /// Default warm-up duration in milliseconds.
    pub const DEFAULT_WARMUP_MILLIS: u64 = 3000;
    /// Default unit for recording latencies.
    pub const DEFAULT_RECORDING_UNIT: LatencyUnit = LatencyUnit::Nano;
    /// Default number of significant decimal digits for the HDR histogram.
    pub const DEFAULT_SIGFIG: u8 = 3;
    /// Default status reporting interval in milliseconds.
    pub const DEFAULT_STATUS_MILLIS: u64 = 1000;
    /// Default error behavior: do not panic on error, return NaN/tainted values instead.
    pub const DEFAULT_PANIC_ON_ERROR: bool = false;

    /// The number of milliseconds used to "warm-up" the benchmark.
    pub fn warmup_millis(&self) -> u64 {
        self.warmup_millis
    }

    /// Unit in which latencies are recorded.
    pub fn recording_unit(&self) -> LatencyUnit {
        self.recording_unit
    }

    /// Number of significant figures used for the HDR histogram.
    ///
    /// This is the number of significant decimal digits to which the histogram will maintain value resolution and separation.
    pub fn sigfig(&self) -> u8 {
        self.sigfig
    }

    /// Status reporting interval in milliseconds.
    pub fn status_millis(&self) -> u64 {
        self.status_millis
    }

    /// Flag determining error behavior of library functions that don't return a [`Result`].
    ///
    /// See [`BenchCfg`] struct documentation.
    pub fn panic_on_error(&self) -> bool {
        self.panic_on_error
    }

    /// Changes the number of milliseconds used to "warm-up" the benchmark.
    pub fn with_warmup_millis(mut self, warmup_millis: u64) -> Self {
        self.warmup_millis = warmup_millis;
        self
    }

    /// Sets the recording unit.
    pub fn with_recording_unit(mut self, recording_unit: LatencyUnit) -> Self {
        self.recording_unit = recording_unit;
        self
    }

    /// Sets the number of significant figures for the HDR histogram.
    pub fn with_sigfig(mut self, sigfig: u8) -> Self {
        self.sigfig = sigfig;
        self
    }

    /// Sets the status reporting interval in milliseconds.
    pub fn with_status_millis(mut self, status_millis: u64) -> Self {
        self.status_millis = status_millis;
        self
    }

    /// Flag determining error behavior of library functions that don't return a [`Result`].
    ///
    /// See [`BenchCfg`] struct documentation.
    pub fn with_panic_on_error(mut self, panic_on_error: bool) -> Self {
        self.panic_on_error = panic_on_error;
        self
    }

    fn execs_per_milli_budget(&self, exec_run_length: RunLength) -> RunLength {
        const WARMUP_DIVISOR: u32 = 3;
        const EXEC_DIVISOR: u32 = 30;

        // Computes the median of a vector of length 3 or less.
        fn median<T, D>(mut vec: Vec<T>, vec_len_2_divisor: D) -> Option<T>
        where
            T: Ord + Copy + Add<Output = T> + Div<D, Output = T>,
        {
            vec.sort();
            match vec.len() {
                0 => None,
                1 => Some(vec[0]),
                2 => Some((vec[0] + vec[1]) / vec_len_2_divisor),
                3 => Some(vec[1]),
                _ => panic!("vector length exceeds 3"),
            }
        }

        let adj_warmup_run_length = RunLength::Duration(Duration::from_millis(
            self.warmup_millis / WARMUP_DIVISOR as u64,
        ));
        let adj_status_run_length = RunLength::Duration(Duration::from_millis(self.status_millis));
        let adj_exec_run_length = match exec_run_length {
            RunLength::Count(count) => RunLength::Count(count / EXEC_DIVISOR as u64),
            RunLength::Duration(dur) => RunLength::Duration(dur / EXEC_DIVISOR),
            RunLength::CountWithTimeout(count, dur) => {
                RunLength::CountWithTimeout(count / EXEC_DIVISOR as u64, dur / EXEC_DIVISOR)
            }
        };

        let run_lengths = [
            adj_warmup_run_length,
            adj_status_run_length,
            adj_exec_run_length,
        ];
        debug!("run_lengths[warmup, status, exec]={run_lengths:?}");

        let counts = run_lengths
            .iter()
            .filter_map(|rl| match rl {
                RunLength::Count(count) => Some(*count),
                RunLength::CountWithTimeout(count, _) => Some(*count),
                _ => None,
            })
            .collect::<Vec<_>>();

        let durs = [adj_exec_run_length]
            .iter()
            .filter_map(|rl| match rl {
                RunLength::Duration(dur) => Some(*dur),
                RunLength::CountWithTimeout(_, dur) => Some(*dur),
                _ => None,
            })
            .collect::<Vec<_>>();

        let median_count = median(counts, 2);
        let median_dur = median(durs, 2);

        let budget = match (median_count, median_dur) {
            (Some(count), Some(dur)) => RunLength::CountWithTimeout(count, dur),
            (Some(count), None) => RunLength::Count(count),
            (None, Some(dur)) => RunLength::Duration(dur),
            (None, None) => unreachable!("impossible"),
        };

        debug!("budget={budget:?}");
        budget
    }

    /// Estimates how many executions of `f` fit in one millisecond.
    ///
    /// Used in status reporting as well as in execution loop termination logic (to ensure adherence to the
    /// run length specified when the benchmark is executed).
    pub fn fn_execs_per_milli(&self, f: impl FnMut(), exec_run_length: RunLength) -> f64 {
        let budget = self.execs_per_milli_budget(exec_run_length);
        latency::fn_executions_per_milli(f, budget)
    }

    /// Estimates how many iterations of `src` can be done in one millisecond.
    ///
    /// Used in status reporting as well as in execution loop termination logic (to ensure adherence to the
    /// run length specified when the benchmark is executed).
    pub fn ltn_src_execs_per_milli<const K: usize>(
        &self,
        src: &mut impl Iterator<Item = [Duration; K]>,
        exec_run_length: RunLength,
    ) -> f64 {
        let budget = self.execs_per_milli_budget(exec_run_length);
        debug!("execs_per_milli_budget={budget:?}");
        latency::ltn_src_executions_per_milli(src.map(|arr| arr.iter().sum()), budget)
    }

    /// Number of executions between status updates, derived from `execs_per_milli`.
    pub fn status_freq(&self, execs_per_milli: f64) -> u64 {
        let status_freq = self.status_millis as f64 * execs_per_milli;
        1.max(status_freq.ceil() as u64)
    }
}

impl Default for BenchCfg {
    fn default() -> Self {
        Self {
            warmup_millis: Self::DEFAULT_WARMUP_MILLIS,
            recording_unit: Self::DEFAULT_RECORDING_UNIT,
            sigfig: Self::DEFAULT_SIGFIG,
            status_millis: Self::DEFAULT_STATUS_MILLIS,
            panic_on_error: Self::DEFAULT_PANIC_ON_ERROR,
        }
    }
}

#[doc(hidden)]
/// Extends [`AokValue`].
pub trait PanicIfNeeded: AokValue + Sized {
    /// Panics if `panic == true` and the receiver is tainted. Used only internally by this crate and `bench_diff`.
    fn panic_if_needed(self, panic: bool, msg: &str) -> Self {
        if panic && self.is_tainted() {
            panic!("{msg}")
        }
        self
    }
}

impl<T> PanicIfNeeded for T where T: AokValue + Sized {}

#[cfg(test)]
#[cfg(feature = "_test")]
mod test {
    use crate::{BenchCfg, LatencyUnit, RunLength};
    use std::time::Duration;

    #[test]
    fn test_bench_cfg_default() {
        let cfg = BenchCfg::default();

        println!("cfg={cfg:?}");
        assert_eq!(cfg.warmup_millis(), BenchCfg::DEFAULT_WARMUP_MILLIS);
        assert_eq!(cfg.recording_unit(), BenchCfg::DEFAULT_RECORDING_UNIT);
        assert_eq!(cfg.sigfig(), BenchCfg::DEFAULT_SIGFIG);
        assert_eq!(cfg.status_millis(), BenchCfg::DEFAULT_STATUS_MILLIS);
    }

    #[test]
    fn test_bench_cfg_builder_method_chaining() {
        let cfg = BenchCfg::default()
            .with_recording_unit(LatencyUnit::Micro)
            .with_warmup_millis(100)
            .with_sigfig(5)
            .with_status_millis(200)
            .with_panic_on_error(true);
        println!("cfg={cfg:?}");

        assert_eq!(cfg.warmup_millis(), 100);
        assert_eq!(cfg.recording_unit(), LatencyUnit::Micro);
        assert_eq!(cfg.sigfig(), 5);
        assert_eq!(200, cfg.status_millis);
        assert!(cfg.panic_on_error);
    }

    #[test]
    fn test_run_length_get_exec_count_and_duration() {
        // Count variant
        let (count, dur) = RunLength::Count(100).get_exec_count_and_duration();
        assert_eq!(count, 100);
        assert_eq!(dur, Duration::MAX);

        // Duration variant
        let (count, dur) =
            RunLength::Duration(Duration::from_secs(5)).get_exec_count_and_duration();
        assert_eq!(count, u64::MAX);
        assert_eq!(dur, Duration::from_secs(5));

        // CountWithTimeout variant
        let (count, dur) =
            RunLength::CountWithTimeout(100, Duration::from_secs(5)).get_exec_count_and_duration();
        assert_eq!(count, 100);
        assert_eq!(dur, Duration::from_secs(5));
    }

    #[test]
    fn test_run_length_estimated_count() {
        let execs_per_milli = 1000.0; // 1 execution per microsecond

        // Count: estimated count is just the count
        assert_eq!(RunLength::Count(50).estimated_count(execs_per_milli), 50);

        // Duration: count derived from time
        // 3 seconds * 1000 execs/milli = 3_000_000
        let est = RunLength::Duration(Duration::from_secs(3)).estimated_count(execs_per_milli);
        assert_eq!(est, 3_000_000);

        // CountWithTimeout: min of count and time-based estimate
        // Time: 1_000ms * 1000/milli = 1_000_000. Count = 10. Min = 10
        assert_eq!(
            RunLength::CountWithTimeout(10, Duration::from_secs(1))
                .estimated_count(execs_per_milli),
            10
        );

        // CountWithTimeout: timeout is shorter
        // Time: 1ms * 1000/milli = 1000. Count = 10_000. Min = 1000
        assert_eq!(
            RunLength::CountWithTimeout(10_000, Duration::from_millis(1))
                .estimated_count(execs_per_milli),
            1000
        );

        // Zero executions per milli
        assert_eq!(RunLength::Count(5).estimated_count(0.0), 5);

        // Zero executions per milli with Duration: 0 * 10 = 0
        let est = RunLength::Duration(Duration::from_millis(10)).estimated_count(0.0);
        assert_eq!(est, 0);
    }

    #[test]
    fn test_run_length_estimated_duration() {
        let execs_per_milli = 1000.0;

        // Count: duration derived from count
        assert_eq!(
            RunLength::Count(5000).estimated_duration(execs_per_milli),
            Duration::from_millis(5) // 5000 / 1000 = 5ms
        );

        // Duration: just the duration
        assert_eq!(
            RunLength::Duration(Duration::from_secs(2)).estimated_duration(execs_per_milli),
            Duration::from_secs(2)
        );

        // CountWithTimeout: min of count-derived and timeout
        // Count: 1000/1000 = 1ms. Timeout: 10ms. Min = 1ms
        assert_eq!(
            RunLength::CountWithTimeout(1000, Duration::from_millis(10))
                .estimated_duration(execs_per_milli),
            Duration::from_millis(1)
        );

        // CountWithTimeout: timeout is shorter
        // Count: 50000/1000 = 50ms. Timeout: 10ms. Min = 10ms
        assert_eq!(
            RunLength::CountWithTimeout(50_000, Duration::from_millis(10))
                .estimated_duration(execs_per_milli),
            Duration::from_millis(10)
        );

        // Zero execs_per_milli results in large duration (division by zero treated as inf -> u64 max milliseconds)
        let zero_est = RunLength::Count(5000).estimated_duration(0.0);
        assert_eq!(zero_est, Duration::from_millis(u64::MAX));

        // Large count
        let huge = RunLength::Count(1_000_000_000).estimated_duration(1.0);
        assert_eq!(huge, Duration::from_millis(1_000_000_000));
    }

    #[test]
    fn test_bench_cfg_status_freq() {
        let cfg = BenchCfg::default();
        println!("cfg={cfg:?}");

        // 1000ms interval, 500 execs/milli => 500_000 status freq
        let freq = cfg.status_freq(500.0);
        assert_eq!(freq, 500_000);

        // 1000ms interval, 1.5 execs/milli => ceil(1500) = 1500
        let freq = cfg.status_freq(1.5);
        assert_eq!(freq, 1500);

        // Zero execs_per_milli => 1
        let freq = cfg.status_freq(0.0);
        assert_eq!(freq, 1);
    }

    #[test]
    fn test_bench_cfg_executions_per_milli() {
        let cfg = BenchCfg::default();
        // Using a no-op closure, the calibration should return a reasonable positive value
        let epms = cfg.fn_execs_per_milli(|| {}, RunLength::Count(10));
        assert!(epms.is_finite());
    }
}
