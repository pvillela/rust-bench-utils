use std::{thread, time::Duration};

/// Function that sleeps to simulate work to support validation of benchmarking frameworks.
#[inline(always)]
pub fn fake_work(target_latency: Duration) {
    thread::sleep(target_latency);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_fake_work_zero() {
        // Should not panic with zero duration
        fake_work(Duration::ZERO);
    }

    #[test]
    fn test_fake_work_nonzero() {
        // Should not panic with a small duration
        fake_work(Duration::from_nanos(1));
    }
}
