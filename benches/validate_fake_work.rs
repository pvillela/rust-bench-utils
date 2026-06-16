use std::time::Duration;

use basic_stats::{dev_utils::ApproxEq, rel_approx_eq};
use bench_utils::{latency, load::fake_work};

fn main() {
    fn run(dur: Duration) -> (f64, f64) {
        let latency_secs = latency(|| fake_work(dur)).as_secs_f64();
        let dur_secs = dur.as_secs_f64();
        let rel_diff = dur_secs.abs_rel_diff(latency_secs);
        println!(
            "dur={:?}, dur_secs={}, latency_secs={}, rel_diff={}",
            dur, dur_secs, latency_secs, rel_diff
        );
        (dur_secs, latency_secs)
    }

    // fake_work_zero
    {
        let dur = Duration::ZERO;
        let (_, latency_secs) = run(dur);
        assert!(latency_secs > 0.0);
    }

    // fake_work_1_nano
    {
        const EPSILON: f64 = 2.0;
        let dur = Duration::from_nanos(1);
        let (dur_secs, latency_secs) = run(dur);
        rel_approx_eq!(dur_secs, latency_secs, EPSILON);
    }

    // fake_work_1_micro
    {
        const EPSILON: f64 = 2.0;
        let dur = Duration::from_micros(1);
        let (dur_secs, latency_secs) = run(dur);
        rel_approx_eq!(dur_secs, latency_secs, EPSILON);
    }

    // fake_work_50_micro
    {
        const EPSILON: f64 = 0.75;
        let dur = Duration::from_micros(50);
        let (dur_secs, latency_secs) = run(dur);
        rel_approx_eq!(dur_secs, latency_secs, EPSILON);
    }

    // fake_work_100_micro
    {
        const EPSILON: f64 = 0.75;
        let dur = Duration::from_micros(100);
        let (dur_secs, latency_secs) = run(dur);
        rel_approx_eq!(dur_secs, latency_secs, EPSILON);
    }

    // fake_work_200_micro
    {
        const EPSILON: f64 = 0.30;
        let dur = Duration::from_micros(200);
        let (dur_secs, latency_secs) = run(dur);
        rel_approx_eq!(dur_secs, latency_secs, EPSILON);
    }

    // fake_work_1_milli
    {
        const EPSILON: f64 = 0.10;
        let dur = Duration::from_millis(1);
        let (dur_secs, latency_secs) = run(dur);
        rel_approx_eq!(dur_secs, latency_secs, EPSILON);
    }

    // fake_work_50_millis
    {
        const EPSILON: f64 = 0.01;
        let dur = Duration::from_millis(50);
        let (dur_secs, latency_secs) = run(dur);
        rel_approx_eq!(dur_secs, latency_secs, EPSILON);
    }
}
