use std::sync::Mutex;

use crate::{LatencyUnit, latency};

#[derive(Debug, Clone)]
pub struct BenchCfg {
    warmup_millis: u64,
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
        warmup_millis: u64,
        recording_unit: LatencyUnit,
        reporting_unit: LatencyUnit,
        sigfig: u8,
        status_calibr: u64,
        status_millis: u64,
        static_ref: &'static Mutex<BenchCfg>,
    ) -> BenchCfg {
        BenchCfg {
            warmup_millis,
            recording_unit,
            reporting_unit,
            sigfig,
            status_calibr,
            status_millis,
            static_ref,
        }
    }

    /// The currently defined number of milliseconds used to "warm-up" the benchmark. The default is 3,000 ms.
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

    pub fn status_freq(&self, mut f: impl FnMut()) -> usize {
        let latency_millis = latency(|| {
            for _ in 0..self.status_calibr {
                f()
            }
        })
        .as_secs_f64()
            * 1000.;
        let inverse_millis_per_iteration = self.status_calibr as f64 / latency_millis;
        let iterations_per_status_millis = self.status_millis as f64 * inverse_millis_per_iteration;
        iterations_per_status_millis as usize
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
