//! Functions to support the "naive" comparison benchmarking approach, where each function is benchmarked separately.

use bench_utils::LatencyUnit;
use std::env::{self, VarError};

#[derive(Debug)]
pub struct Args {
    pub target_ratio: f64,
    pub latency_unit: LatencyUnit,
    pub base_median: f64,
    pub nrepeats: usize,
}

fn with_default(res: Result<String, VarError>, deflt: &str) -> String {
    match res {
        Ok(s) if !s.is_empty() => s,
        _ => deflt.into(),
    }
}

pub fn get_args() -> Args {
    let target_ratio_str = with_default(env::var("TARGET_RATIO"), "1.1");
    let target_ratio_msg = || -> f64 {
        panic!(
            "TARGET_RATIO, if provided, must be a non-negative number; was \"{target_ratio_str}\""
        )
    };
    let target_ratio = target_ratio_str
        .parse::<f64>()
        .map(|r| {
            if !(r > 0.) {
                target_ratio_msg();
            }
            r
        })
        .unwrap_or_else(|_| target_ratio_msg());

    let latency_unit_str = with_default(env::var("LATENCY_UNIT"), "micro");
    let latency_unit = match latency_unit_str.to_lowercase() {
        s if s == "nano" => LatencyUnit::Nano,
        s if s == "micro" => LatencyUnit::Micro,
        s if s == "milli" => LatencyUnit::Milli,
        s => panic!("invalid LATENCY_UNIT environment variable value: {s}"),
    };

    let base_median_str = with_default(env::var("BASE_MEDIAN"), "100");
    let base_median = base_median_str.parse::<f64>().unwrap_or_else(|_| {
        panic!("BASE_MEDIAN, if provided, must be a non-negative number; was \"{base_median_str}\"")
    });
    assert!(
        base_median >= 0.,
        "BASE_MEDIAN, if provided, must be a non-negative number; was \"{base_median_str}\""
    );

    let nrepeats_str = with_default(env::var("NREPEATS"), "10");
    let nrepeats = nrepeats_str.parse::<usize>().unwrap_or_else(|_| {
        panic!("NREPEATS, if provided, must be a non-negative integer; was \"{nrepeats_str}\"")
    });

    Args {
        target_ratio,
        latency_unit,
        base_median,
        nrepeats,
    }
}
