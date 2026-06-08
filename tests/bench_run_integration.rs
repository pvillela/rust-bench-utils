#![cfg(feature = "_test")]

use bench_utils::{BenchCfg, Comp, RunLength, bench_run_arg_cfg};

#[test]
fn test_bench_run_to_comp_roundtrip() {
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
