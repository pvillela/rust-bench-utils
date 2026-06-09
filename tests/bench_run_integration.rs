#![cfg(feature = "_test")]

use basic_stats::approx_eq;
use bench_utils::{BenchCfg, Comp, RunLength, bench_run_arg_cfg};

#[test]
fn test_bench_run_to_comp_roundtrip_with_fn() {
    let cfg = BenchCfg::default().with_warmup_millis(10);
    // Run benchmark for f1 and f2 separately
    let out1 = bench_run_arg_cfg(&cfg, || {}, RunLength::Count(10));
    let out2 = bench_run_arg_cfg(&cfg, || {}, RunLength::Count(10));

    assert_eq!(out1.n(), 10);
    assert_eq!(out2.n(), 10);

    // Compare them via Comp
    let comp = Comp::new(&out1, &out2);

    // Both ran the same no-op, so ratio should be ~1.0
    let ratio = comp.ratio_medians_f1_f2();
    assert!(
        ratio > 0.5 && ratio < 2.0,
        "ratio should be close to 1.0, got {}",
        ratio
    );
}

#[test]
fn test_bench_run_to_comp_accept_null_hyp() {
    use bench_utils::multi::bench_run_arg_cfg;
    use bench_utils::multi::test_support::LognormalLatencySrc;
    use bench_utils::stats_types::{AltHyp, PositionWrtCi};
    use std::time::Duration;

    let cfg = BenchCfg::default().with_warmup_millis(0);
    let target = Duration::from_millis(10);

    // Two independent LognormalLatencySrc instances with the same target median —
    // each draws from its own RNG, so their samples differ.
    let src1 = LognormalLatencySrc::<1>::new_with_default_sigmas([target]);
    let src2 = LognormalLatencySrc::<1>::new_with_default_sigmas([target]);

    // Sample sizes different to show `Comp` works with different sample sizes.
    let out1 = bench_run_arg_cfg(&cfg, src1, RunLength::Count(1000));
    let out2 = bench_run_arg_cfg(&cfg, src2, RunLength::Count(1001));

    assert_eq!(out1.n(), 1000);
    assert_eq!(out2.n(), 1001);

    // BenchOut<1>: Deref → &BenchOut
    let comp = Comp::new(&out1, &out2);

    // Same target median → ratio should be ~1.0
    let ratio = comp.ratio_medians_f1_f2();
    assert!(
        (0.95..=1.05).contains(&ratio),
        "ratio should be close to 1.0, got {}",
        ratio
    );

    // Cannot reject the null hypothesis that ln(median) difference is 0
    let p = comp.welch_ln_p(0.0, AltHyp::Ne);
    assert!(p > 0.05, "p-value should be > 0.05, got {}", p);

    // The CI for ln-difference should contain 0.0
    let ci = comp.welch_ln_ci(0.05);
    assert_eq!(
        ci.position_of(0.0),
        PositionWrtCi::In,
        "CI {:?} should contain 0.0",
        ci
    );
}

#[test]
fn test_bench_run_to_comp_reject_null_hyp() {
    use bench_utils::multi::bench_run_arg_cfg;
    use bench_utils::multi::test_support::LognormalLatencySrc;
    use bench_utils::stats_types::{AltHyp, PositionWrtCi};
    use std::time::Duration;

    let cfg = BenchCfg::default().with_warmup_millis(0);
    let target1 = Duration::from_millis(10);
    let target2 = Duration::from_millis(9);

    // Two independent LognormalLatencySrc instances with the same target median —
    // each draws from its own RNG, so their samples differ.
    let src1 = LognormalLatencySrc::<1>::new_with_default_sigmas([target1]);
    let src2 = LognormalLatencySrc::<1>::new_with_default_sigmas([target2]);

    let out1 = bench_run_arg_cfg(&cfg, src1, RunLength::Count(1000));
    let out2 = bench_run_arg_cfg(&cfg, src2, RunLength::Count(1000));

    assert_eq!(out1.n(), 1000);
    assert_eq!(out2.n(), 1000);

    // BenchOut<1>: Deref → &BenchOut
    let comp = Comp::new(&out1, &out2);

    // Ratio should be ~10.0/9.0
    let ratio = comp.ratio_medians_f1_f2();
    approx_eq!(10.0 / 9.0, ratio, 0.01);

    // Must reject the null hypothesis that ln(median) difference is 0
    let p = comp.welch_ln_p(0.0, AltHyp::Gt);
    assert!(p < 0.05, "p-value should be < 0.05, got {}", p);

    // The CI for the ratio should contain 10.0 / 9.0
    let ci = comp.welch_ratio_ci(0.05);
    assert_eq!(
        ci.position_of(10.0 / 9.0),
        PositionWrtCi::In,
        "CI {:?} should contain `10.0 / 9.0`",
        ci
    );
}
