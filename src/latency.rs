use std::time::{Duration, Instant};

/// Invokes `f` once and returns its latency.
#[inline(always)]
pub fn latency(f: impl FnOnce()) -> Duration {
    let start = Instant::now();
    f();
    start.elapsed()
}

/// Unit of time used to record latencies. Used as an argument in benchmarking functions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LatencyUnit {
    /// Milliseconds.
    Milli,
    /// Microseconds.
    Micro,
    /// Nanoseconds.
    Nano,
}

impl LatencyUnit {
    const fn nano_equivalence(&self) -> f64 {
        match self {
            Self::Milli => 1_000_000.,
            Self::Micro => 1_000.,
            Self::Nano => 1.,
        }
    }

    /// Factor for conversion from `self` to `reporting_unit`.
    pub const fn conversion_factor(&self, reporting_unit: Self) -> f64 {
        self.nano_equivalence() / reporting_unit.nano_equivalence()
    }

    /// Converts a `latency` [`Duration`] to a `u64` value according to the unit `self`.
    #[inline(always)]
    pub fn latency_as_u64(&self, latency: Duration) -> u64 {
        match self {
            Self::Nano => latency.as_nanos() as u64,
            Self::Micro => latency.as_micros() as u64,
            Self::Milli => latency.as_millis() as u64,
        }
    }

    /// Converts a `u64` value to a [`Duration`] according to the unit `self`.
    #[inline(always)]
    pub fn latency_from_u64(&self, elapsed: u64) -> Duration {
        match self {
            Self::Nano => Duration::from_nanos(elapsed),
            Self::Micro => Duration::from_micros(elapsed),
            Self::Milli => Duration::from_millis(elapsed),
        }
    }

    /// Converts a `latency` [`Duration`] to an `f64` value according to the unit `self`.
    #[inline(always)]
    pub fn latency_as_f64(&self, latency: Duration) -> f64 {
        match self {
            Self::Nano => latency.as_nanos() as f64,
            Self::Micro => latency.as_nanos() as f64 / 1_000.0,
            Self::Milli => latency.as_nanos() as f64 / 1_000_000.0,
        }
    }

    /// Converts an `f64` value to a [`Duration`] according to the unit `self`.
    ///
    /// Rounds the floating point value, so precision can be lost in a round trip starting with `self.latency_as_f64`,
    /// followed by `self.latency_from_f64`, followed by `self.latency_as_f64`.
    #[inline(always)]
    pub fn latency_from_f64(&self, elapsed: f64) -> Duration {
        // self.latency_from_u64(elapsed as u64)
        match self {
            Self::Nano => Duration::from_nanos(elapsed as u64),
            Self::Micro => Duration::from_nanos((elapsed * 1_000.0) as u64),
            Self::Milli => Duration::from_nanos((elapsed * 1_000_000.0) as u64),
        }
    }
}

#[cfg(test)]
#[cfg(feature = "_dev_support")]
mod test {
    use super::*;
    use basic_stats::approx_eq;

    #[test]
    fn test_latency() {
        let dur = latency(|| {});
        // latency of a no-op closure should be sub-second
        assert!(dur < Duration::from_secs(1));
    }

    #[test]
    fn test_conversion_factor() {
        // Identity factors
        approx_eq!(
            1.0,
            LatencyUnit::Milli.conversion_factor(LatencyUnit::Milli),
            1e-15
        );
        approx_eq!(
            1.0,
            LatencyUnit::Micro.conversion_factor(LatencyUnit::Micro),
            1e-15
        );
        approx_eq!(
            1.0,
            LatencyUnit::Nano.conversion_factor(LatencyUnit::Nano),
            1e-15
        );

        // Convert from larger to smaller: Nano -> Micro -> Milli
        approx_eq!(
            0.001,
            LatencyUnit::Nano.conversion_factor(LatencyUnit::Micro),
            1e-15
        );
        approx_eq!(
            0.000_001,
            LatencyUnit::Nano.conversion_factor(LatencyUnit::Milli),
            1e-15
        );
        approx_eq!(
            0.001,
            LatencyUnit::Micro.conversion_factor(LatencyUnit::Milli),
            1e-15
        );

        // Convert from smaller to larger: Milli -> Micro -> Nano
        approx_eq!(
            1000.0,
            LatencyUnit::Micro.conversion_factor(LatencyUnit::Nano),
            1e-12
        );
        approx_eq!(
            1_000_000.0,
            LatencyUnit::Milli.conversion_factor(LatencyUnit::Nano),
            1e-9
        );
        approx_eq!(
            1000.0,
            LatencyUnit::Milli.conversion_factor(LatencyUnit::Micro),
            1e-12
        );
    }

    #[test]
    fn test_latency_as_u64() {
        let dur = Duration::new(1, 500_000_000); // 1.5 seconds
        assert_eq!(1500, LatencyUnit::Milli.latency_as_u64(dur));
        assert_eq!(1_500_000, LatencyUnit::Micro.latency_as_u64(dur));
        assert_eq!(1_500_000_000, LatencyUnit::Nano.latency_as_u64(dur));

        let zero = Duration::ZERO;
        assert_eq!(0, LatencyUnit::Milli.latency_as_u64(zero));
        assert_eq!(0, LatencyUnit::Micro.latency_as_u64(zero));
        assert_eq!(0, LatencyUnit::Nano.latency_as_u64(zero));
    }

    #[test]
    fn test_latency_from_u64_roundtrip() {
        // Milli round-trip
        let dur = LatencyUnit::Milli.latency_from_u64(42);
        assert_eq!(42, LatencyUnit::Milli.latency_as_u64(dur));

        // Micro round-trip
        let dur = LatencyUnit::Micro.latency_from_u64(42);
        assert_eq!(42, LatencyUnit::Micro.latency_as_u64(dur));

        // Nano round-trip
        let dur = LatencyUnit::Nano.latency_from_u64(42);
        assert_eq!(42, LatencyUnit::Nano.latency_as_u64(dur));

        // Zero
        let dur = LatencyUnit::Micro.latency_from_u64(0);
        assert_eq!(0, LatencyUnit::Micro.latency_as_u64(dur));

        // Large value (fits exactly in Duration)
        let val: u64 = 1_000_000_000_000_000; // 10^15 nanos = ~11.6 days
        let dur = LatencyUnit::Nano.latency_from_u64(val);
        assert_eq!(val, LatencyUnit::Nano.latency_as_u64(dur));
    }

    #[test]
    fn test_latency_as_f64() {
        let dur = Duration::from_nanos(2_001_001);
        approx_eq!(2_001_001.0, LatencyUnit::Nano.latency_as_f64(dur), 1e-6);
        approx_eq!(2_001.001, LatencyUnit::Micro.latency_as_f64(dur), 1e-9);
        approx_eq!(2.001_001, LatencyUnit::Milli.latency_as_f64(dur), 1e-12);

        // Duration in nanos less then 1 micro
        let small = Duration::from_nanos(999);
        approx_eq!(0.999, LatencyUnit::Micro.latency_as_f64(small), 1e-12);
    }

    #[test]
    fn test_latency_from_f64() {
        assert_eq!(
            LatencyUnit::Nano.latency_from_f64(500.7),
            Duration::from_nanos(500),
        );
        assert_eq!(
            LatencyUnit::Micro.latency_from_f64(500.7),
            Duration::from_nanos(500_700),
        );
        assert_eq!(
            LatencyUnit::Milli.latency_from_f64(1000.999_999),
            Duration::from_nanos(1_000_999_999),
        );
        assert_eq!(
            LatencyUnit::Milli.latency_from_f64(1000.000_001),
            Duration::from_nanos(1_000_000_001),
        );
    }

    #[test]
    fn test_latency_round_trip_f64() {
        // Round trip
        let nanos_u = 999 as u64;
        let dur = Duration::from_nanos(nanos_u);

        let nanos = LatencyUnit::Nano.latency_as_f64(dur);
        let micros = LatencyUnit::Micro.latency_as_f64(dur);
        let millis = LatencyUnit::Milli.latency_as_f64(dur);

        let dur_nan = LatencyUnit::Nano.latency_from_f64(nanos);
        let dur_mic = LatencyUnit::Micro.latency_from_f64(micros);
        let dur_mil = LatencyUnit::Milli.latency_from_f64(millis);

        assert_eq!(dur_nan, dur_mic);
        assert_eq!(dur_nan, dur_mil);
    }
}
