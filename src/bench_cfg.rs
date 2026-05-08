use crate::{LatencyUnit, latency};
use std::{sync::Mutex, time::Duration};

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
    /// Returns a [`RunLength`] that specifies the benchmark will run for `count` iterations, with no timeout.
    pub const fn from_count(count: usize) -> Self {
        Self::Count(count)
    }

    /// Returns a [`RunLength`] that specifies the benchmark will run for until `duration` is reached or exceeded,
    /// regardless of the number of iterations required.
    pub const fn from_duration(duration: Duration) -> Self {
        Self::Duration(duration)
    }

    /// Returns a [`RunLength`] that specifies the benchmark will run for `count` iterations, with `duration` as
    /// the timeout limit.
    pub const fn from_count_and_duration(count: usize, timeout: Duration) -> Self {
        Self::CountWithTimeout(count, timeout)
    }

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

    pub fn estimated_count<T>(&self, cfg: &BenchCfg, f: impl FnMut() -> T) -> usize {
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

    pub fn estimated_duration<T>(&self, cfg: &BenchCfg, f: impl FnMut() -> T) -> Duration {
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
    warmup_run_length: RunLength,
    recording_unit: LatencyUnit,
    reporting_unit: LatencyUnit,
    sigfig: u8,
    status_calibr: u64,
    status_millis: u64,
    static_ref: &'static Mutex<BenchCfg>,
}

impl BenchCfg {
    #[doc(hidden)]
    pub const fn new(
        warmup_run_length: RunLength,
        recording_unit: LatencyUnit,
        reporting_unit: LatencyUnit,
        sigfig: u8,
        status_calibr: u64,
        status_millis: u64,
        static_ref: &'static Mutex<BenchCfg>,
    ) -> BenchCfg {
        BenchCfg {
            warmup_run_length,
            recording_unit,
            reporting_unit,
            sigfig,
            status_calibr,
            status_millis,
            static_ref,
        }
    }

    /// The currently defined [`RunLength`] used to "warm-up" the benchmark. The default is 3,000 ms.
    pub fn warmup_run_length(&self) -> RunLength {
        self.warmup_run_length
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
    pub fn with_warmup_run_length(mut self, warmup_run_length: RunLength) -> Self {
        self.warmup_run_length = warmup_run_length;
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
        self.status_calibr = status_calibr;
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

    pub fn executions_per_milli<T>(&self, mut f: impl FnMut() -> T) -> f64 {
        let latency_millis = latency(|| {
            for _ in 0..self.status_calibr {
                f();
            }
        })
        .as_secs_f64()
            * 1000.;

        self.status_calibr as f64 / latency_millis
    }

    pub fn status_freq(&self, f: impl FnMut()) -> usize {
        let executions_per_milli = self.executions_per_milli(f);
        let executions_per_status_millis = self.status_millis as f64 * executions_per_milli;
        executions_per_status_millis as usize
    }
}

#[cfg(test)]
#[cfg(feature = "_bench_run")]
mod test {
    use std::time::Duration;

    use crate::{LatencyUnit, RunLength, get_bench_cfg};

    #[test]
    fn test_bench_cfg() {
        let cfg = get_bench_cfg();
        println!("cfg={cfg:?}");
        assert_eq!(
            cfg.warmup_run_length().get_exec_count_and_duration().1,
            Duration::from_millis(3000)
        );
        assert_eq!(cfg.recording_unit(), LatencyUnit::Nano);
        assert_eq!(cfg.reporting_unit(), LatencyUnit::Micro);
        assert_eq!(cfg.sigfig(), 3);

        cfg.with_recording_unit(LatencyUnit::Micro)
            .with_warmup_run_length(RunLength::Duration(Duration::from_millis(100)))
            .with_reporting_unit(LatencyUnit::Milli)
            .with_sigfig(5)
            .set();
        let cfg = get_bench_cfg();
        println!("cfg={cfg:?}");
        assert_eq!(
            cfg.warmup_run_length().get_exec_count_and_duration().1,
            Duration::from_millis(100)
        );
        assert_eq!(cfg.recording_unit(), LatencyUnit::Micro);
        assert_eq!(cfg.reporting_unit(), LatencyUnit::Milli);
        assert_eq!(cfg.sigfig(), 5);
    }
}
