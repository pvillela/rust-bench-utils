//! Empirically justifies `BenchCfg::DEFAULT_SIGFIG = 4` and demonstrates that the large
//! `rc_q_ns / stdev_r` gaps seen in some tiers of `mean_and_median_estimates.rs` are
//! outlier-driven, not a defect of the Rousseeuw-Croux `Q_n` robust scale estimator.
//!
//! Backs `reference/Q_n_resolution_and_outlier_analysis.md`; see that document for the narrative
//! and the theory behind each computation. This example only prints raw tables.
//!
//! Uses the same target-latency/batch/sample-size tiers as `mean_and_median_estimates.rs`.
//!
//! ```
//! cargo run -r --example q_resolution_analysis --features load,_test
//! ```

use bench_utils::{
    BenchCfg, BenchOut, FpSeconds, LatencyUnit, RunLength, bench_run_arg_cfg,
    load::{BusyWork, Calibration},
};
use std::time::Duration;

fn main() {
    let calibration = BusyWork::calibrate();

    for &(target_latency, batch, samp_size) in TIERS {
        println!("\n*** target_latency={target_latency:?}, batch={batch:?}, samp_size={samp_size}");
        analyze_tier(target_latency, calibration, batch, samp_size);
    }
}

/// Same tiers as `examples/mean_and_median_estimates.rs`.
const TIERS: &[(Duration, Option<usize>, usize)] = &[
    (Duration::from_nanos(1), Some(100_000), 100),
    (Duration::from_nanos(10), Some(10_000), 100),
    (Duration::from_nanos(100), Some(1_000), 100),
    (Duration::from_micros(1), Some(100), 100),
    (Duration::from_micros(10), Some(10), 100),
    (Duration::from_micros(100), None, 1000),
    (Duration::from_micros(100), Some(10), 100),
    (Duration::from_millis(1), None, 500),
    (Duration::from_millis(1), Some(10), 50),
    (Duration::from_millis(10), None, 200),
    (Duration::from_millis(10), Some(10), 20),
];

fn analyze_tier(
    target_latency: Duration,
    calibration: Calibration,
    batch: Option<usize>,
    samp_size: usize,
) {
    let bsz = batch.unwrap_or(1).max(1);
    let run_length = RunLength::Count(bsz * samp_size);
    let (effort, _calibr_ltncy) = calibration.effort_for_latency(target_latency.into());
    let f = BusyWork::fun(effort);

    // Ground-truth capture at the finest resolution under consideration (sigfig=5). Re-recording
    // this *same* sample at sigfig 3 and 4 (below) simulates "what a coarser histogram would have
    // produced" without repeat-measurement noise confounding the 3-vs-4-vs-5 comparison.
    let cfg5 = BenchCfg::default()
        .with_recording_unit(LatencyUnit::sub_sec(12))
        .with_warmup_millis(100)
        .with_sigfig(5);
    let out5 = bench_run_arg_cfg(&cfg5, f, run_length, batch);
    let sample: Vec<FpSeconds> = out5.iter().collect();

    let mut out3 = BenchOut::new(&cfg5.clone().with_sigfig(3), batch);
    out3.record_from_iter(sample.iter().copied());
    let mut out4 = BenchOut::new(&cfg5.clone().with_sigfig(4), batch);
    out4.record_from_iter(sample.iter().copied());

    // --- Part 1: resolution table (sigfig 3 vs 4 vs 5 on the identical sample) ---
    println!(
        "{:<7} {:>12} {:>12} {:>10} {:>14} {:>14}",
        "sigfig", "rc_q_ns", "rc_q_ls", "tie_frac", "n_r", "n_buckets"
    );
    for (sigfig, out) in [(3, &out3), (4, &out4), (5, &out5)] {
        let tie_frac = tie_fraction(out);
        let n_buckets = out.iter_with_counts().count();
        println!(
            "{:<7} {:>12?} {:>12.3e} {:>10.4} {:>14} {:>14}",
            sigfig,
            out.rousseeuw_croux_q_ns(),
            out.rousseeuw_croux_q_ls(),
            tie_frac,
            out.n_r(),
            n_buckets
        );
    }

    // --- Part 2: Tukey-fence / Winsorized-SD outlier analysis, at the chosen default (sigfig=4) ---
    // Operates on the expanded, bucket-quantized sample (`iter()`), i.e. the same data
    // `rousseeuw_croux_q_ns` itself sees internally, for a fair like-for-like comparison. This
    // differs negligibly (given Part 1's resolution finding) from `stdev_r()`'s exact-sum basis.
    let expanded: Vec<f64> = out4.iter().map(|v| v.as_f64()).collect();
    let summary4 = out4.summary();
    let p25 = summary4.p25.as_f64();
    let p75 = summary4.p75.as_f64();
    let iqr = p75 - p25;
    let lower_fence = p25 - 1.5 * iqr;
    let upper_fence = p75 + 1.5 * iqr;

    let n_outliers = expanded
        .iter()
        .filter(|&&v| v < lower_fence || v > upper_fence)
        .count();
    let winsorized: Vec<f64> = expanded
        .iter()
        .map(|&v| v.clamp(lower_fence, upper_fence))
        .collect();

    let stdev_r_raw = sample_stdev(&expanded);
    let stdev_r_winsorized = sample_stdev(&winsorized);
    let iqr_over_1349 = iqr / 1.349;
    let rc_q_ns = out4.rousseeuw_croux_q_ns().as_f64();

    println!(
        "outliers(Tukey): n={}, fences=[{:.6e}, {:.6e}]s, p25={:.6e}s, p75={:.6e}s",
        n_outliers, lower_fence, upper_fence, p25, p75
    );
    println!(
        "stdev_r_raw={stdev_r_raw:.6e}s, stdev_r_winsorized={stdev_r_winsorized:.6e}s, iqr/1.349={iqr_over_1349:.6e}s, rc_q_ns={rc_q_ns:.6e}s"
    );
    println!(
        "rc_q_ns/stdev_r_raw={:.3}, rc_q_ns/stdev_r_winsorized={:.3}, rc_q_ns/(iqr/1.349)={:.3}",
        rc_q_ns / stdev_r_raw,
        rc_q_ns / stdev_r_winsorized,
        rc_q_ns / iqr_over_1349
    );

    // --- Log-space analog (ln Y), for symmetry with the natural-space table above ---
    let ln_expanded: Vec<f64> = expanded.iter().map(|v| v.ln()).collect();
    let mut ln_sorted = ln_expanded.clone();
    ln_sorted.sort_by(|a, b| a.total_cmp(b));
    let p25_ln = sample_quantile(&ln_sorted, 0.25);
    let p75_ln = sample_quantile(&ln_sorted, 0.75);
    let iqr_ln = p75_ln - p25_ln;
    let lower_fence_ln = p25_ln - 1.5 * iqr_ln;
    let upper_fence_ln = p75_ln + 1.5 * iqr_ln;
    let winsorized_ln: Vec<f64> = ln_expanded
        .iter()
        .map(|&v| v.clamp(lower_fence_ln, upper_fence_ln))
        .collect();
    let stdev_ln_r_raw = sample_stdev(&ln_expanded);
    let stdev_ln_r_winsorized = sample_stdev(&winsorized_ln);
    let iqr_ln_over_1349 = iqr_ln / 1.349;
    let rc_q_ls = out4.rousseeuw_croux_q_ls();

    println!(
        "stdev_ln_r_raw={stdev_ln_r_raw:.3e}, stdev_ln_r_winsorized={stdev_ln_r_winsorized:.3e}, iqr_ln/1.349={iqr_ln_over_1349:.3e}, rc_q_ls={rc_q_ls:.3e}"
    );
    println!(
        "rc_q_ls/stdev_ln_r_raw={:.3}, rc_q_ls/stdev_ln_r_winsorized={:.3}, rc_q_ls/(iqr_ln/1.349)={:.3}",
        rc_q_ls / stdev_ln_r_raw,
        rc_q_ls / stdev_ln_r_winsorized,
        rc_q_ls / iqr_ln_over_1349
    );
}

/// Fraction of the `C(n,2)` pairwise comparisons that fall between two observations in the *same*
/// HDR bucket (`Σ C(count_i,2) / C(n,2)`), i.e. the same-bucket "tie" fraction that
/// `rousseeuw_croux_q_general` (`src/bench_out.rs`) must fall back to a bucket-width
/// approximation for. A large tie fraction relative to the ~25th-percentile target rank is the
/// mechanism behind `Q_n` collapsing toward the quantization floor.
fn tie_fraction(out: &BenchOut) -> f64 {
    let mut same_bucket_pairs: u128 = 0;
    let mut n: u128 = 0;
    for (_, count) in out.iter_with_counts() {
        let c = count as u128;
        same_bucket_pairs += c * c.saturating_sub(1) / 2;
        n += c;
    }
    let total_pairs = n * n.saturating_sub(1) / 2;
    if total_pairs == 0 {
        0.0
    } else {
        same_bucket_pairs as f64 / total_pairs as f64
    }
}

/// Sample standard deviation (n-1 denominator) of a slice of `f64`.
fn sample_stdev(v: &[f64]) -> f64 {
    let n = v.len() as f64;
    let mean = v.iter().sum::<f64>() / n;
    let var = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    var.sqrt()
}

/// Linear-interpolation sample quantile of an already-sorted slice.
fn sample_quantile(sorted: &[f64], q: f64) -> f64 {
    let n = sorted.len();
    if n == 1 {
        return sorted[0];
    }
    let pos = q * (n - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let frac = pos - lo as f64;
        sorted[lo] * (1.0 - frac) + sorted[hi] * frac
    }
}
