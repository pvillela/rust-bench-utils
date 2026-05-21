use basic_stats::{approx_eq, dev_utils::ApproxEq, rel_approx_eq};
use bench_utils::{BusyWork, latency};
use std::time::Duration;

fn main() {
    validate_core();
    validate_ratio();
}

fn run_core(dur: Duration) -> (f64, f64) {
    let f = BusyWork::new(dur).fun();
    let latency_secs = latency(f).as_secs_f64();
    let dur_secs = dur.as_secs_f64();
    let rel_diff = dur_secs.abs_rel_diff(latency_secs);
    println!(
        "dur={:?}, dur_secs={}, latency_secs={}, rel_diff={}",
        dur, dur_secs, latency_secs, rel_diff
    );
    (dur_secs, latency_secs)
}

fn validate_core() {
    // test_busy_work_new_zero()
    {
        const EPSILON: f64 = 0.005;
        let dur = Duration::ZERO;
        let (dur_secs, latency_secs) = run_core(dur);
        approx_eq!(dur_secs, latency_secs, EPSILON);
    }

    // test_busy_work_new_1_nano()
    {
        const EPSILON: f64 = 2.0;
        let dur = Duration::from_nanos(1);
        let (dur_secs, latency_secs) = run_core(dur);
        rel_approx_eq!(dur_secs, latency_secs, EPSILON);
    }

    // test_busy_work_new_1_micro()
    {
        const EPSILON: f64 = 0.75;
        let dur = Duration::from_micros(1);
        let (dur_secs, latency_secs) = run_core(dur);
        rel_approx_eq!(dur_secs, latency_secs, EPSILON);
    }

    // test_busy_work_new_1_milli()
    {
        const EPSILON: f64 = 0.25;
        let dur = Duration::from_millis(1);
        let (dur_secs, latency_secs) = run_core(dur);
        rel_approx_eq!(dur_secs, latency_secs, EPSILON);
    }

    // test_busy_work_new_50_millis()
    {
        const EPSILON: f64 = 0.25;
        let dur = Duration::from_millis(50);
        let (dur_secs, latency_secs) = run_core(dur);
        rel_approx_eq!(dur_secs, latency_secs, EPSILON);
    }
}

fn validate_ratio() {
    fn run_ratio(dur1: Duration, ratio: f64, repeats: u32) -> f64 {
        let bw1 = BusyWork::new(dur1);
        let effort1 = bw1.effort();
        let effort2 = (effort1 as f64 * ratio) as u32;

        let f1 = bw1.fun();
        let f2 = BusyWork::from_effort(effort2).fun();

        let mut latency_secs1 = 0.0;
        let mut latency_secs2 = 0.0;

        for _ in 0..repeats {
            latency_secs1 += latency(&f1).as_secs_f64();
            latency_secs2 += latency(&f2).as_secs_f64();
        }

        let latency_ratio = latency_secs2 / latency_secs1;
        let rel_diff = latency_ratio.abs_rel_diff(ratio);

        println!(
            "dur1={:?}, latency_ratio={}, ratio={}, rel_diff={}",
            dur1, latency_ratio, ratio, rel_diff
        );

        latency_ratio
    }

    const RATIO: f64 = 1.10;

    // test_busy_work_ratio_100_nano()
    {
        const EPSILON: f64 = 0.5; // not reliable at nano scale
        let dur1 = Duration::from_nanos(100);
        let repeats = 100_000;
        let latency_ratio = run_ratio(dur1, RATIO, repeats);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }

    // test_busy_work_ratio_100_micro()
    {
        const EPSILON: f64 = 0.50;
        let dur1 = Duration::from_micros(10);
        let repeats = 1_000;
        let latency_ratio = run_ratio(dur1, RATIO, repeats);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }

    // test_busy_work_ratio_1_milli()
    {
        const EPSILON: f64 = 0.05;
        let dur1 = Duration::from_millis(1);
        let repeats = 100;
        let latency_ratio = run_ratio(dur1, RATIO, repeats);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }

    // test_busy_work_ratio_10_millis()
    {
        const EPSILON: f64 = 0.10;
        let dur1 = Duration::from_millis(10);
        let repeats = 10;
        let latency_ratio = run_ratio(dur1, RATIO, repeats);
        rel_approx_eq!(latency_ratio, RATIO, EPSILON);
    }
}
