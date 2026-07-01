use crate::{LatencyUnit, RunLength, latency, multi::LatencySrc};
use log::{Level, debug, log_enabled};
use std::time::{Duration, Instant};

/// Benchmark configuration, excluding the benchmark run length.
///
/// Encapsulates the following data:
/// - `warmup_millis`: warm-up duration in milliseconds
/// - `status_millis`: milliseconds between status reports during bench execution, if progress status reporting is enabled
/// - `recording_unit`: time unit for latency recording
/// - `sigfig`: as data is stored in an [HDR (high dynamic range) histogram](https://docs.rs/hdrhistogram/latest/hdrhistogram/index.html),
///   this is the number of significant decimal digits (of `recording_unit`) to which the histogram will maintain
///   value resolution and separation
#[derive(Debug, Clone)]
pub struct BenchCfg {
    warmup_millis: u64,
    status_millis: u64,
    recording_unit: LatencyUnit,
    sigfig: u8,
}

impl BenchCfg {
    /// Default warm-up duration in milliseconds.
    pub const DEFAULT_WARMUP_MILLIS: u64 = 3000;
    /// Default status reporting interval in milliseconds.
    pub const DEFAULT_STATUS_MILLIS: u64 = 1000;
    /// Default unit for recording latencies.
    pub const DEFAULT_RECORDING_UNIT: LatencyUnit = LatencyUnit::NANO;
    /// Default number of significant decimal digits for the HDR histogram.
    pub const DEFAULT_SIGFIG: u8 = 3;

    /// The number of milliseconds used to "warm-up" the benchmark.
    pub fn warmup_millis(&self) -> u64 {
        self.warmup_millis
    }

    /// Status reporting interval in milliseconds.
    pub fn status_millis(&self) -> u64 {
        self.status_millis
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

    /// Sets the number of milliseconds used to "warm-up" the benchmark.
    pub fn with_warmup_millis(mut self, warmup_millis: u64) -> Self {
        self.warmup_millis = warmup_millis;
        self
    }

    /// Sets the status reporting interval in milliseconds.
    pub fn with_status_millis(mut self, status_millis: u64) -> Self {
        self.status_millis = status_millis;
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

    fn execs_per_sec_budget(&self, exec_run_length: RunLength) -> RunLength {
        const WARMUP_DIVISOR: u32 = 3;
        const EXEC_DIVISOR: u32 = 30;

        // Computes the median of a vector of length 3 or less.
        fn median(mut vec: Vec<f64>) -> Option<f64> {
            vec.sort_by(f64::total_cmp);
            match vec.len() {
                0 => None,
                1 => Some(vec[0]),
                2 => Some((vec[0] + vec[1]) / 2.0),
                3 => Some(vec[1]),
                _ => panic!("vector length exceeds 3"),
            }
        }

        let adj_warmup_run_length = RunLength::Time(Duration::from_millis(
            self.warmup_millis / WARMUP_DIVISOR as u64,
        ));
        let adj_exec_run_length = match exec_run_length {
            RunLength::Count(count) => RunLength::Count(count / EXEC_DIVISOR as usize),
            RunLength::Time(dur) => RunLength::Time(dur / EXEC_DIVISOR),
            RunLength::CountWithTimeout(count, dur) => {
                RunLength::CountWithTimeout(count / EXEC_DIVISOR as usize, dur / EXEC_DIVISOR)
            }
        };

        let run_lengths = [adj_warmup_run_length, adj_exec_run_length];
        debug!("execs_per_sec_budget >>> run_lengths[warmup, exec]={run_lengths:?}");

        let counts = run_lengths
            .iter()
            .filter_map(|rl| match rl {
                RunLength::Count(count) => Some(*count as f64),
                RunLength::CountWithTimeout(count, _) => Some(*count as f64),
                _ => None,
            })
            .collect::<Vec<_>>();
        debug!("execs_per_sec_budget >>> counts={counts:?}");

        let durs = run_lengths
            .iter()
            .filter_map(|rl| match rl {
                RunLength::Time(dur) => Some(dur.as_secs_f64()),
                RunLength::CountWithTimeout(_, dur) => Some(dur.as_secs_f64()),
                _ => None,
            })
            .collect::<Vec<_>>();
        debug!("execs_per_sec_budget >>> durs={durs:?}");

        let median_count = median(counts);
        let median_dur = median(durs);
        debug!("execs_per_sec_budget >>> median_count={median_count:?}, median_dur={median_dur:?}");

        let budget = match (median_count, median_dur) {
            (Some(count), Some(dur)) => {
                RunLength::CountWithTimeout(count.round() as usize, Duration::from_secs_f64(dur))
            }
            (Some(count), None) => RunLength::Count(count.round() as usize),
            (None, Some(dur)) => RunLength::Time(Duration::from_secs_f64(dur)),
            (None, None) => unreachable!("impossible"),
        };

        debug!("execs_per_sec_budget >>> budget={budget:?}");
        budget
    }

    /// Estimates how many iterations of `src` can be done in one second.
    ///
    /// Used in status reporting as well as in execution loop termination logic (to ensure adherence to the
    /// run length specified when the benchmark is executed).
    /// ///
    /// # May return [`f64::INFINITY`]:
    /// Returns `f64::INFINITY` if the aggregate latency for any iteration is zero.
    /// In particular, this can happen if `src` is finite and its length is less than or equal to one half
    /// of the estimation budget count (see [`latency::execs_per_sec`] and [`Self::execs_per_sec_budget`]).
    pub(crate) fn execs_per_sec<const K: usize>(
        &self,
        src: &mut impl LatencySrc<K>,
        exec_run_length: RunLength,
    ) -> f64 {
        let start = if log_enabled!(Level::Debug) {
            Some(Instant::now())
        } else {
            None
        };

        let budget = self.execs_per_sec_budget(exec_run_length);
        let eps = latency::execs_per_sec(src.aggregate(), budget);

        debug!(
            "execs_per_sec >>> execs_per_sec={eps:?}, elapsed={:?}",
            start.map(|start| start.elapsed())
        );

        eps
    }

    /// Number of executions between status updates, derived from `execs_per_second`.
    pub(crate) fn status_count(&self, execs_per_second: f64) -> usize {
        let status_count = self.status_millis as f64 / 1000.0 * execs_per_second;
        1.max(status_count.ceil() as usize)
    }
}

impl Default for BenchCfg {
    fn default() -> Self {
        Self {
            warmup_millis: Self::DEFAULT_WARMUP_MILLIS,
            status_millis: Self::DEFAULT_STATUS_MILLIS,
            recording_unit: Self::DEFAULT_RECORDING_UNIT,
            sigfig: Self::DEFAULT_SIGFIG,
        }
    }
}

#[cfg(test)]
#[cfg(feature = "_test")]
mod test {
    use crate::multi::LatencySrc1;
    use crate::multi::test_support::LognormalLatencySrc;
    use crate::{BenchCfg, FpSeconds, LatencyUnit, RunLength};
    use basic_stats::rel_approx_eq;
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
            .with_recording_unit(LatencyUnit::MICRO)
            .with_warmup_millis(100)
            .with_sigfig(5)
            .with_status_millis(200);

        assert_eq!(cfg.warmup_millis(), 100);
        assert_eq!(cfg.recording_unit(), LatencyUnit::MICRO);
        assert_eq!(cfg.sigfig(), 5);
        assert_eq!(200, cfg.status_millis);
    }

    #[test]
    fn test_run_length_get_exec_count_and_time() {
        // Count variant
        let (count, dur) = RunLength::Count(100).exec_count_and_duration();
        assert_eq!(count, 100);
        assert_eq!(dur, Duration::MAX);

        // Duration variant
        let (count, dur) = RunLength::Time(Duration::from_secs(5)).exec_count_and_duration();
        assert_eq!(count, usize::MAX);
        assert_eq!(dur, Duration::from_secs(5));

        // CountWithTimeout variant
        let (count, dur) =
            RunLength::CountWithTimeout(100, Duration::from_secs(5)).exec_count_and_duration();
        assert_eq!(count, 100);
        assert_eq!(dur, Duration::from_secs(5));
    }

    #[test]
    fn test_run_length_estimated_count() {
        let execs_per_second = 1_000_000.0; // 1 execution per microsecond

        // Count: estimated count is just the count
        assert_eq!(RunLength::Count(50).estimated_count(execs_per_second), 50);

        // Duration: count derived from time
        // 3 seconds * 1_000_000 execs/sec = 3_000_000
        let est = RunLength::Time(Duration::from_secs(3)).estimated_count(execs_per_second);
        assert_eq!(est, 3_000_000);

        // CountWithTimeout: min of count and time-based estimate
        // Time: 1s * 1_000_000/s = 1_000_000. Count = 10. Min = 10
        assert_eq!(
            RunLength::CountWithTimeout(10, Duration::from_secs(1))
                .estimated_count(execs_per_second),
            10
        );

        // CountWithTimeout: timeout is shorter
        // Time: 0.001s * 1_000_000/s = 1000. Count = 10_000. Min = 1000
        assert_eq!(
            RunLength::CountWithTimeout(10_000, Duration::from_millis(1))
                .estimated_count(execs_per_second),
            1000
        );

        // Zero executions per second panics
        let result = std::panic::catch_unwind(|| {
            RunLength::Count(5).estimated_count(0.0);
        });
        assert!(
            result.is_err(),
            "estimated_count should panic for execs_per_second == 0"
        );
    }

    #[test]
    fn test_run_length_estimated_time() {
        let execs_per_second = 1_000_000.0;

        // Count: duration derived from count
        assert_eq!(
            RunLength::Count(5000).estimated_time(execs_per_second),
            Duration::from_millis(5) // 5000 / 1_000_000/s = 5ms
        );

        // Duration: just the duration
        assert_eq!(
            RunLength::Time(Duration::from_secs(2)).estimated_time(execs_per_second),
            Duration::from_secs(2)
        );

        // CountWithTimeout: min of count-derived and timeout
        // Count: 1000/1000 = 1ms. Timeout: 10ms. Min = 1ms
        assert_eq!(
            RunLength::CountWithTimeout(1000, Duration::from_millis(10))
                .estimated_time(execs_per_second),
            Duration::from_millis(1)
        );

        // CountWithTimeout: timeout is shorter
        // Count: 50000/1000 = 50ms. Timeout: 10ms. Min = 10ms
        assert_eq!(
            RunLength::CountWithTimeout(50_000, Duration::from_millis(10))
                .estimated_time(execs_per_second),
            Duration::from_millis(10)
        );

        // Zero execs_per_second results in panic
        let result = std::panic::catch_unwind(|| RunLength::Count(5000).estimated_time(0.0));
        assert!(
            result.is_err(),
            "should panic when execs_per_second is zero"
        );

        // Large count
        let huge = RunLength::Count(1_000_000_000).estimated_time(1000.0);
        assert_eq!(huge, Duration::from_secs(1_000_000));
    }

    #[test]
    fn test_bench_cfg_status_count() {
        let cfg = BenchCfg::default().with_status_millis(2000);
        println!("cfg={cfg:?}");

        // 2000ms interval, 500_000 execs/s => 500_000 status count
        let count = cfg.status_count(500_000.0);
        assert_eq!(count, 1_000_000);

        // 2000ms interval, 1_5000 execs/s => ceil(3000) = 3000
        let count = cfg.status_count(1500.0);
        assert_eq!(count, 3000);

        // Zero execs_per_second => 1
        let count = cfg.status_count(0.0);
        assert_eq!(count, 1);
    }

    #[test]
    // cargo test --package bench_utils --lib --all-features -- bench_cfg::test::test_bench_cfg_execs_per_second --exact --nocapture --include-ignored
    fn test_bench_cfg_execs_per_second() {
        _ = env_logger::try_init();
        let cfg = BenchCfg::default();
        // Using a no-op closure, the calibration should return a reasonable positive value
        let mut src = LatencySrc1::new(|| {});
        let eps = cfg.execs_per_sec(&mut src, RunLength::Count(10));
        assert!(eps.is_finite());
    }

    #[test]
    fn test_src_execs_per_sec_estimation() {
        let cfg = BenchCfg::default();
        let mut src =
            LognormalLatencySrc::<1>::new_with_default_sigmas([FpSeconds::from_millis(10)], 1);
        let eps = cfg.execs_per_sec(&mut src, RunLength::Count(500));
        // Expected: 1000ms / 10ms = 100.0
        rel_approx_eq!(100.0, eps, 0.05);
    }

    #[test]
    fn test_src_execs_per_sec_time_run_length() {
        let cfg = BenchCfg::default();
        let mut src =
            LognormalLatencySrc::<1>::new_with_default_sigmas([FpSeconds::from_millis(1)], 1);
        let eps = cfg.execs_per_sec(&mut src, RunLength::Time(Duration::from_millis(5)));
        assert!(eps.is_finite() && eps > 0.0);
        // Rough check: should be close to 1000 (1000ms / 1ms)
        rel_approx_eq!(1000.0, eps, 0.30);
    }

    #[test]
    fn test_src_execs_per_sec_count_with_timeout() {
        let cfg = BenchCfg::default();
        let mut src =
            LognormalLatencySrc::<1>::new_with_default_sigmas([FpSeconds::from_millis(5)], 1);
        let eps = cfg.execs_per_sec(
            &mut src,
            RunLength::CountWithTimeout(200, Duration::from_millis(5)),
        );
        assert!(eps > 0.0);
        // Expected: ~200 (1000ms / 5ms)
        rel_approx_eq!(200.0, eps, 0.50);
    }
}
