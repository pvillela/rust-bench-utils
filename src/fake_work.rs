use std::{thread, time::Duration};

/// Function that sleeps to simulate work to support validation of benchmarking frameworks.
#[inline(always)]
pub fn fake_work(target_latency: Duration) {
    thread::sleep(target_latency);
}
