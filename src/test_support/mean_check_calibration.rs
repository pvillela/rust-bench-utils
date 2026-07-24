//! Monte Carlo calibration of the [`crate::BenchOut::mean_check`] standardized-gap cutoffs
//! ([`crate::MEAN_CHECK_MILD_Z`] / [`crate::MEAN_CHECK_SIGNIFICANT_Z`]).
//!
//! Crate-internal because it reuses one [`BenchOut`] across trials via the crate-private
//! `reset()`/`capture_data()` (a fresh `BenchOut` per trial would allocate a multi-MB histogram each
//! time). It feeds *synthetic* lognormal batch means into a real `BenchOut` so `mean()`/`mean_rob()`
//! run exactly as in production, then studies the distribution of
//! `rel_gap = (naive_mean - mean_rob) / mean_rob` on clean and contaminated data.
//!
//! Backs `reference/analysis/Mean_check_threshold_calibration.md`; prints raw tables only.
//!
//! Run the synthetic study:
//! ```text
//! cargo test -r --lib --features _ALL_NON_TEST,_test -- \
//!     test_support::mean_check_calibration::synthetic_study --ignored --nocapture
//! ```
//! Run the BusyWork spot-check (also needs `load`):
//! ```text
//! cargo test -r --lib --features _ALL_NON_TEST,_test,load -- \
//!     test_support::mean_check_calibration::busywork_spotcheck --ignored --nocapture
//! ```

use crate::{BenchCfg, BenchOut, FpSeconds, dev_support::SplitMix64};

// ---- Tunable grid (kept modest so a run finishes in a few minutes under `-r`) ----
const SIGMAS: &[f64] = &[0.5, 1.0, 2.0];
const BATCH_SIZES: &[usize] = &[1, 64, 1000];
const N_BATCHES: &[usize] = &[50, 200];
const EPS: &[f64] = &[0.01, 0.05, 0.10, 0.20];
const LAMBDA: f64 = 10.0;
const TRIALS_CLEAN: usize = 2000;
const TRIALS_CONTAM: usize = 1000;
const BASE_SEED: u64 = 0xCAFE_F00D_1234_5678;
/// `exp(mu)`: median per-execution latency, chosen mid-histogram for the default recording unit.
const MEDIAN_LATENCY_SECS: f64 = 1e-3;

/// Candidate `(mild_z, sig_z)` standardized-gap cutoffs to score.
const Z_CANDIDATES: &[(f64, f64)] = &[(2.0, 3.0), (2.5, 4.0), (3.0, 5.0)];

#[derive(Clone, Copy, PartialEq)]
enum Pattern {
    Clean,
    /// Diffuse: each draw independently inflated ×LAMBDA with probability eps.
    Point,
    /// Concentrated: a fraction eps of whole batches inflated ×LAMBDA.
    Burst,
}

impl Pattern {
    fn label(self) -> &'static str {
        match self {
            Pattern::Clean => "clean",
            Pattern::Point => "point",
            Pattern::Burst => "burst",
        }
    }
    fn tag(self) -> u64 {
        match self {
            Pattern::Clean => 1,
            Pattern::Point => 2,
            Pattern::Burst => 3,
        }
    }
}

/// One simulation cell: the parameters of a single (sigma, k, g, pattern, eps) configuration.
#[derive(Clone, Copy)]
struct Cell {
    /// `ln` of the median per-execution latency.
    mu: f64,
    sigma: f64,
    k: usize,
    g: usize,
    pattern: Pattern,
    eps: f64,
}

/// Seeded standard-normal generator (Box-Muller over a `SplitMix64` uniform stream).
struct NormalGen {
    rng: SplitMix64,
    spare: Option<f64>,
}

impl NormalGen {
    fn new(seed: u64) -> Self {
        Self {
            rng: SplitMix64::new(seed),
            spare: None,
        }
    }

    /// Uniform in the open interval (0, 1) with 53-bit resolution.
    fn u01(&mut self) -> f64 {
        let bits = self.rng.next_u64() >> 11; // top 53 bits
        (bits as f64 + 0.5) / (1u64 << 53) as f64
    }

    fn normal(&mut self) -> f64 {
        if let Some(z) = self.spare.take() {
            return z;
        }
        let u1 = self.u01();
        let u2 = self.u01();
        let r = (-2.0 * u1.ln()).sqrt();
        let (s, c) = (2.0 * std::f64::consts::PI * u2).sin_cos();
        self.spare = Some(r * s);
        r * c
    }
}

/// Deterministic per-config seed so every cell is independent yet reproducible.
fn cfg_seed(cell: Cell) -> u64 {
    let mut s = SplitMix64::new(BASE_SEED);
    let mut mix = |v: u64| {
        s = SplitMix64::new(s.next_u64() ^ v.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    };
    mix(cell.sigma.to_bits());
    mix(cell.k as u64);
    mix(cell.g as u64);
    mix(cell.pattern.tag());
    mix(cell.eps.to_bits());
    s.next_u64()
}

/// Runs one trial: fills `out` with `g` batch means (size `k`) from `LogNormal(mu, sigma)`
/// contaminated per `pattern`/`eps`, then returns `(rel_gap, sigma_Y_hat)`.
fn run_trial(out: &mut BenchOut, ng: &mut NormalGen, cell: Cell) -> (f64, f64) {
    let Cell {
        mu,
        sigma,
        k,
        g,
        pattern,
        eps,
    } = cell;
    out.reset();
    for _ in 0..g {
        let burst_hit = pattern == Pattern::Burst && ng.u01() < eps;
        let mut sum = 0.0;
        for _ in 0..k {
            let mut x = (mu + sigma * ng.normal()).exp();
            if pattern == Pattern::Point && ng.u01() < eps {
                x *= LAMBDA;
            }
            sum += x;
        }
        let mut y = sum / k as f64;
        if burst_hit {
            y *= LAMBDA;
        }
        out.capture_data(FpSeconds(y));
    }
    let rel_gap = out.mean_check().rel_gap;
    let sigma_y = out.rousseeuw_croux_q_ls();
    (rel_gap, sigma_y)
}

fn percentile(sorted: &[f64], q: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return f64::NAN;
    }
    let pos = q.clamp(0.0, 1.0) * (n as f64 - 1.0);
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let f = pos - lo as f64;
        sorted[lo] * (1.0 - f) + sorted[hi] * f
    }
}

fn frac_ge(sorted: &[f64], thr: f64) -> f64 {
    let c = sorted.iter().filter(|&&x| x >= thr).count();
    c as f64 / sorted.len() as f64
}

/// Collects `trials` rel_gaps for one (sigma, k, g, pattern, eps) cell, ascending-sorted, plus the
/// mean `sigma_Y_hat`.
fn collect_cell(out: &mut BenchOut, cell: Cell, trials: usize) -> CellStats {
    let mut ng = NormalGen::new(cfg_seed(cell));
    let sqrt_g = (cell.g as f64).sqrt();
    let mut gaps = Vec::with_capacity(trials);
    let mut zs = Vec::with_capacity(trials);
    let mut sy_acc = 0.0;
    for _ in 0..trials {
        let (gap, sy) = run_trial(out, &mut ng, cell);
        // The production statistic: standardize the gap by its clean sampling scale sigma_Y/sqrt(g).
        let z = if sy > 0.0 { gap * sqrt_g / sy } else { 0.0 };
        gaps.push(gap);
        zs.push(z);
        sy_acc += sy;
    }
    gaps.sort_by(f64::total_cmp);
    zs.sort_by(f64::total_cmp);
    CellStats {
        gaps,
        zs,
        mean_sigma_y: sy_acc / trials as f64,
    }
}

/// Per-cell ascending-sorted `rel_gap`s and standardized `z`s, plus the mean `sigma_Y`.
struct CellStats {
    gaps: Vec<f64>,
    zs: Vec<f64>,
    mean_sigma_y: f64,
}

#[test]
#[ignore = "long-running Monte Carlo study; run explicitly with --ignored --nocapture"]
fn synthetic_study() {
    let cfg = BenchCfg::default();
    // One reused BenchOut (histogram allocated once; reset() per trial). `batch` is irrelevant to
    // `mean_check` (mean/mean_rob don't use bsz), so `None` is fine.
    let mut out = BenchOut::new(&cfg, None);
    let mu = MEDIAN_LATENCY_SECS.ln();

    println!("# mean_check standardized-gap (z) threshold calibration (synthetic lognormal MC)");
    println!("# statistic: z = rel_gap * sqrt(g) / sigma_Y   (sigma_Y = rc_q_ls, g = n_r)");
    println!(
        "# LAMBDA={LAMBDA}, TRIALS_CLEAN={TRIALS_CLEAN}, TRIALS_CONTAM={TRIALS_CONTAM}, seed={BASE_SEED:#x}"
    );

    // (sigma, z95, z99) per clean cell; pooled clean z for the target regime (sigma<=1) and all.
    let mut clean_z_by_cell: Vec<(f64, f64, f64)> = Vec::new();
    let mut pooled_clean_z_target: Vec<f64> = Vec::new();
    // Burst@eps=5% detection under each candidate mild_z, pooled over the target regime.
    let mut burst5_det: Vec<Vec<f64>> = vec![Vec::new(); Z_CANDIDATES.len()];

    println!("\n## Clean cells: rel_gap and standardized-z tail percentiles");
    println!(
        "{:>6} {:>6} {:>6} {:>10} {:>9} {:>9} {:>9} {:>9}",
        "sigma", "k", "g", "sigmaY", "gap_p95", "gap_p99", "z_p95", "z_p99"
    );
    for &sigma in SIGMAS {
        for &k in BATCH_SIZES {
            for &g in N_BATCHES {
                let cell = Cell {
                    mu,
                    sigma,
                    k,
                    g,
                    pattern: Pattern::Clean,
                    eps: 0.0,
                };
                let st = collect_cell(&mut out, cell, TRIALS_CLEAN);
                let gap_p95 = percentile(&st.gaps, 0.95);
                let gap_p99 = percentile(&st.gaps, 0.99);
                let z95 = percentile(&st.zs, 0.95);
                let z99 = percentile(&st.zs, 0.99);
                println!(
                    "{sigma:>6.2} {k:>6} {g:>6} {:>10.4} {gap_p95:>9.4} {gap_p99:>9.4} {z95:>9.3} {z99:>9.3}",
                    st.mean_sigma_y
                );
                clean_z_by_cell.push((sigma, z95, z99));
                if sigma <= 1.0 {
                    pooled_clean_z_target.extend_from_slice(&st.zs);
                }
            }
        }
    }

    println!("\n## Contaminated cells: median z + detection at (mild_z=2.5, sig_z=4.0)");
    println!(
        "{:>6} {:>6} {:>6} {:>6} {:>6} {:>10} {:>10} {:>10}",
        "sigma", "k", "g", "patt", "eps", "med_z", "det_mild", "det_sig"
    );
    for &sigma in SIGMAS {
        for &k in BATCH_SIZES {
            for &g in N_BATCHES {
                for &pattern in &[Pattern::Point, Pattern::Burst] {
                    for &eps in EPS {
                        let cell = Cell {
                            mu,
                            sigma,
                            k,
                            g,
                            pattern,
                            eps,
                        };
                        let st = collect_cell(&mut out, cell, TRIALS_CONTAM);
                        let med_z = percentile(&st.zs, 0.50);
                        let det_mild = frac_ge(&st.zs, 2.5);
                        let det_sig = frac_ge(&st.zs, 4.0);
                        println!(
                            "{sigma:>6.2} {k:>6} {g:>6} {:>6} {eps:>6.2} {med_z:>10.3} {det_mild:>10.3} {det_sig:>10.3}",
                            pattern.label()
                        );
                        if pattern == Pattern::Burst && (eps - 0.05).abs() < 1e-9 && sigma <= 1.0 {
                            for (i, &(mz, _)) in Z_CANDIDATES.iter().enumerate() {
                                burst5_det[i].push(frac_ge(&st.zs, mz));
                            }
                        }
                    }
                }
            }
        }
    }

    // ---- Summary ----
    pooled_clean_z_target.sort_by(f64::total_cmp);
    let max_by = |sel: &dyn Fn(&(f64, f64, f64)) -> f64, only_target: bool| -> f64 {
        clean_z_by_cell
            .iter()
            .filter(|c| !only_target || c.0 <= 1.0)
            .map(sel)
            .fold(f64::MIN, f64::max)
    };
    println!("\n## Clean z tail (worst cell)");
    println!(
        "sigma<=1 (target regime): z95_max={:.3}  z99_max={:.3}",
        max_by(&|c| c.1, true),
        max_by(&|c| c.2, true)
    );
    println!(
        "all cells:                z95_max={:.3}  z99_max={:.3}   (sigma=2 = high-dispersion caveat)",
        max_by(&|c| c.1, false),
        max_by(&|c| c.2, false)
    );

    println!(
        "\n## Candidate z-cutoffs: pooled clean false-alarm (sigma<=1) vs mean burst@eps=5% detection"
    );
    println!(
        "{:>7} {:>7} {:>13} {:>13} {:>16}",
        "mild_z", "sig_z", "clean_FPmild", "clean_FPsig", "burst5_det_mild"
    );
    for (i, &(mz, sz)) in Z_CANDIDATES.iter().enumerate() {
        let fp_mild = frac_ge(&pooled_clean_z_target, mz);
        let fp_sig = frac_ge(&pooled_clean_z_target, sz);
        let det = burst5_det[i].iter().sum::<f64>() / burst5_det[i].len().max(1) as f64;
        println!("{mz:>7.2} {sz:>7.2} {fp_mild:>13.4} {fp_sig:>13.4} {det:>16.3}");
    }
    println!("\n# Recommendation: mild_z ~ clean z95_max (sigma<=1), sig_z ~ clean z99_max (sigma<=1).");
    println!("# sigma=2 (large sigma_Y) breaks the standardization -> documented high-dispersion caveat.");
}

#[cfg(feature = "load")]
#[test]
#[ignore = "BusyWork spot-check; run explicitly with --ignored --nocapture"]
fn busywork_spotcheck() {
    use crate::{RunLength, bench_run_arg_cfg, load::BusyWork};
    use std::time::Duration;

    let calibration = BusyWork::calibrate();
    let cfg = BenchCfg::default().with_warmup_millis(200);

    println!("# BusyWork spot-check: real measured mean_check on clean + 5%-burst-injected samples");
    println!(
        "{:>10} {:>6} {:>6} {:>10} {:>10} {:>8} {:>14} {:>8} {:>14}",
        "target", "k", "g", "sigmaY", "clean_gap", "clean_z", "clean_verd", "burst_z", "burst_verd"
    );

    // A couple of tiers spanning batched / unbatched.
    let tiers: &[(Duration, Option<usize>, usize)] = &[
        (Duration::from_micros(1), Some(100), 200),
        (Duration::from_millis(1), None, 200),
    ];

    for &(target, batch, g) in tiers {
        let k = batch.unwrap_or(1).max(1);
        let (effort, _) = calibration.effort_for_latency(target.into());
        let f = BusyWork::fun(effort);
        let out = bench_run_arg_cfg(&cfg, f, RunLength::Count(k * g), batch);

        // Real measured mean_check on the clean run.
        let clean = out.mean_check();
        // Inject 5% burst contamination into the measured batch means and re-run mean_check.
        let sample: Vec<FpSeconds> = out.iter().collect();
        let burst = injected_check(&cfg, &sample, batch, 0.05, 0xB0B1);

        println!(
            "{:>10?} {k:>6} {g:>6} {:>10.4} {:>10.4} {:>8.3} {:>14?} {:>8.2} {:>14?}",
            target,
            clean.sigma_y,
            clean.rel_gap,
            clean.std_gap,
            clean.verdict,
            burst.std_gap,
            burst.verdict,
        );
    }
    println!(
        "# Compare clean std_gap / verdict against the synthetic clean tail for the matching sigma_Y;"
    );
    println!("# burst injection should push std_gap past MEAN_CHECK_MILD_Z (2.5).");
}

/// Records `sample` (measured batch means) into a fresh `BenchOut` after injecting 5% burst
/// contamination (a fraction `eps` of recorded values ×LAMBDA), and returns the resulting
/// [`crate::MeanCheck`]. Used by the BusyWork spot-check.
#[cfg(feature = "load")]
fn injected_check(
    cfg: &BenchCfg,
    sample: &[FpSeconds],
    batch: Option<usize>,
    eps: f64,
    seed: u64,
) -> crate::MeanCheck {
    let mut ng = NormalGen::new(seed);
    let mut out = BenchOut::new(cfg, batch);
    for &y in sample {
        let mut v = y.0;
        if ng.u01() < eps {
            v *= LAMBDA;
        }
        out.capture_data(FpSeconds(v));
    }
    out.mean_check()
}
