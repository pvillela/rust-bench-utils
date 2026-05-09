use crate::{LatencyUnit, latency};
use std::{
    hint::black_box,
    sync::Mutex,
    time::{Duration, Instant},
};

/// Specifies how long a benchmark should run for. Encapsulates a target number of iterations for the benchmark to run
/// and a time duration. The benchmark run length can be set as a number of iterations, a time duration, or
/// a number of iterations with a timeout duration.
#[derive(Debug, Clone, Copy)]
pub enum RunLength {
    Count(usize),
    Duration(Duration),
    CountWithTimeout(usize, Duration),
}

impl RunLength {
    /// Returns both the number of iterations and time duration specified for the benchmark to run.
    ///
    /// The benchmark ends when the specified number of iterations is reached (or exceeded)
    /// or when the time duration is reached (or exceeded), whichever comes first.
    pub fn get_exec_count_and_duration(&self) -> (usize, Duration) {
        match self {
            Self::Count(count) => (*count, Duration::MAX),
            Self::Duration(duration) => (usize::MAX, *duration),
            Self::CountWithTimeout(count, duration) => (*count, *duration),
        }
    }

    // Used only for status reporting, not for execution control
    pub(crate) fn estimated_count<T>(&self, cfg: &BenchCfg, f: impl FnMut() -> T) -> usize {
        match self {
            Self::Count(count) => *count,
            Self::Duration(duration) => {
                (duration.as_millis() as f64 * cfg.executions_per_milli(f)) as usize
            }
            Self::CountWithTimeout(count, duration) => {
                let count_from_duration =
                    (duration.as_millis() as f64 * cfg.executions_per_milli(f)) as usize;
                *count.min(&count_from_duration)
            }
        }
    }

    // Used only for status reporting, not for execution control
    pub(crate) fn estimated_duration<T>(&self, cfg: &BenchCfg, f: impl FnMut() -> T) -> Duration {
        match self {
            Self::Count(count) => {
                Duration::from_millis((*count as f64 / cfg.executions_per_milli(f)) as u64)
            }
            Self::Duration(duration) => *duration,
            Self::CountWithTimeout(count, duration) => {
                let duration_from_count =
                    Duration::from_millis((*count as f64 / cfg.executions_per_milli(f)) as u64);
                *duration.min(&duration_from_count)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct BenchCfg {
    warmup_millis: u64,
    recording_unit: LatencyUnit,
    reporting_unit: LatencyUnit,
    sigfig: u8,
    base_status_calibr: u64,
    status_millis: u64,
    static_ref: &'static Mutex<BenchCfg>,
}

impl BenchCfg {
    #[doc(hidden)]
    pub const fn new(
        warmup_millis: u64,
        recording_unit: LatencyUnit,
        reporting_unit: LatencyUnit,
        sigfig: u8,
        base_status_calibr: u64,
        status_millis: u64,
        static_ref: &'static Mutex<BenchCfg>,
    ) -> BenchCfg {
        BenchCfg {
            warmup_millis,
            recording_unit,
            reporting_unit,
            sigfig,
            base_status_calibr,
            status_millis,
            static_ref,
        }
    }

    /// The currently defined [`RunLength`] used to "warm-up" the benchmark. The default is 3,000 ms.
    pub fn warmup_millis(&self) -> u64 {
        self.warmup_millis
    }

    pub fn recording_unit(&self) -> LatencyUnit {
        self.recording_unit
    }

    pub fn reporting_unit(&self) -> LatencyUnit {
        self.reporting_unit
    }

    pub fn conversion_factor(&self) -> f64 {
        self.recording_unit.conversion_factor(self.reporting_unit)
    }

    pub fn sigfig(&self) -> u8 {
        self.sigfig
    }

    /// Changes the number of milliseconds used to "warm-up" the benchmark. The default is 3,000 ms.
    pub fn with_warmup_millis(mut self, warmup_millis: u64) -> Self {
        self.warmup_millis = warmup_millis;
        self
    }

    pub fn with_recording_unit(mut self, recording_unit: LatencyUnit) -> Self {
        self.recording_unit = recording_unit;
        self
    }

    pub fn with_reporting_unit(mut self, reporting_unit: LatencyUnit) -> Self {
        self.reporting_unit = reporting_unit;
        self
    }

    pub fn with_sigfig(mut self, sigfig: u8) -> Self {
        self.sigfig = sigfig;
        self
    }

    pub fn with_status_calibr(mut self, status_calibr: u64) -> Self {
        self.base_status_calibr = status_calibr;
        self
    }

    pub fn with_status_millis(mut self, status_millis: u64) -> Self {
        self.status_millis = status_millis;
        self
    }

    pub fn set(self) {
        let mut guard = self.static_ref.lock().unwrap();
        *guard = self;
    }

    pub(crate) fn executions_per_milli<T>(&self, mut f: impl FnMut() -> T) -> f64 {
        let start = Instant::now();

        for i in 0.. {
            let iter_start = Instant::now();
            for _ in 0..self.base_status_calibr * 2u64.pow(i) {
                black_box(f());
            }

            let iter_latency = iter_start.elapsed().as_millis() as u64;
            let acc_latency = start.elapsed().as_millis() as u64;

            if iter_latency >= self.status_millis / 2 || acc_latency >= self.status_millis {
                let iter_execs_per_milli =
                    (self.base_status_calibr * i as u64) as f64 / iter_latency as f64;
                let acc_execs_per_milli =
                    (self.base_status_calibr * 2u64.pow(i + 1) - 1) as f64 / acc_latency as f64;
                return iter_execs_per_milli.min(acc_execs_per_milli);
            }
        }

        unreachable!("above loop must return at some point")
    }

    pub(crate) fn status_freq(&self, f: impl FnMut()) -> usize {
        let executions_per_milli = self.executions_per_milli(f);
        let executions_per_status_millis = self.status_millis as f64 * executions_per_milli;
        executions_per_status_millis as usize
    }
}

#[cfg(test)]
#[cfg(feature = "_bench_run")]
mod test {
    use crate::{LatencyUnit, get_bench_cfg};

    #[test]
    fn test_bench_cfg() {
        let cfg = get_bench_cfg();
        println!("cfg={cfg:?}");
        assert_eq!(cfg.warmup_millis(), 3000);
        assert_eq!(cfg.recording_unit(), LatencyUnit::Nano);
        assert_eq!(cfg.reporting_unit(), LatencyUnit::Micro);
        assert_eq!(cfg.sigfig(), 3);

        cfg.with_recording_unit(LatencyUnit::Micro)
            .with_warmup_millis(100)
            .with_reporting_unit(LatencyUnit::Milli)
            .with_sigfig(5)
            .set();
        let cfg = get_bench_cfg();
        println!("cfg={cfg:?}");
        assert_eq!(cfg.warmup_millis(), 100);
        assert_eq!(cfg.recording_unit(), LatencyUnit::Micro);
        assert_eq!(cfg.reporting_unit(), LatencyUnit::Milli);
        assert_eq!(cfg.sigfig(), 5);
    }
}
