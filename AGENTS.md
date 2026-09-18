# AGENTS.md

This file provides guidance to coding agents when working with code in this repository.

**After completing modifications to any Rust file, ensure it conforms to standard rust formatting, e.g., by running `rustfmt` on it.**

## Build, test, and lint

```bash
./check.sh                  # cargo check --all-targets --all-features
./test.sh                   # cargo nextest run + doc tests (uses nextest, not cargo test)
./clippy.sh                 # cargo clippy --all-targets --all-features
./check-features.sh         # check multiple feature flag combinations
./gen-doc.sh                # generate rustdoc (excludes common crate dependencies)
./coverage.sh               # generate coverage report via cargo-llvm-cov
./validate.sh               # run long-running validation tests (needs _bench feature; throttled to 1 thread)
```

Run a single test with nextest:
```bash
cargo nextest run --lib --features _ALL_NON_TEST,_test --target-dir target/test-target -- <test_name>
```

Run a single test with `cargo test` (latency-sensitive tests need `-r` for release optimizations):
```bash
cargo test -r --lib --features _ALL_NON_TEST,_test -- <test_name>
```

## Feature flags

This crate has complex feature gating with several tiers:

- **Public**: `default`, `load` (gates the `load` module).
- **Helper**: `__null` enables the (optional) `basic_stats` dependency.
- **Internal**: `_test_support`, `_bench`, `_test`, `_experimental`, `_bench_diff`, `_ALL_NON_TEST`, `_ignore` (marks test modules to be skipped).

See `[features]` in `Cargo.toml` for the expansions. Most tests require `_test_support`. The feature `_bench_diff` is for use by the sibling `bench_diff` crate.

## Architecture

`bench_utils` is a Rust library for measuring latency and synthesizing workloads with predictable latency. It publishes to crates.io.
**Core data flow**: [`LatencySrc`] trait abstraction → `bench_run_x(cfg, src, run_length, s)` → warm-up → execute `src.next()` repeatedly → record each latency (as [`FpSeconds`], a floating-point-seconds newtype) in an HDR histogram → produce [`BenchOut`] with descriptive + inferential (Student's t on log-latencies) statistics, including batching-bias-corrected robust estimators of the median.

The library supports benchmarking multiple functions simultaneously via const-generic `K`-arity: `BenchOut<K>` holds `K` `BenchOut` instances and `bench_run` accepts `[impl FnMut(); K]`.

### Key design patterns

- **Stats delegation**: Inferential statistics (t-tests, CIs) are delegated to the sibling `basic_stats` crate. All results are checked via `.expect()` and always panic on error.
- **Log-normal assumption**: Latency distributions are treated as approximately log-normal. Statistics on `ln(latency)` are central to the API (Student's t on one sample, Welch's t for two-sample comparison).
- **`FpSeconds` newtype**: A floating-point-seconds wrapper over `f64` (`src/latency.rs`) is the crate's primary time representation, used in place of `Duration` wherever finer-than-nanosecond precision or convenient arithmetic (`+`, `-`, `*`, `/`, `Sum`) is needed. Converts to/from `Duration` via `From`/`as_duration()`.
- **HDR histogram**: Latencies are recorded into a resizable `hdrhistogram::Histogram<u64>`, which provides quantile/percentile queries.
- **Batching-bias-corrected robust median estimators**: Batching (recording the latency of `n` executions as one observation) biases naive median/mean estimators. `BenchOut::median_rob()` corrects for this using the Rousseeuw-Croux Q statistic and an S-estimator of scale to choose between a "reciprocal of mean of `1/x`"-style estimator and a log-space estimator, based on batch size and estimated dispersion. `Comp`/`DuoOut` expose the same correction via `diff_medians_f1_f2_rob()`/`ratio_medians_f1_f2_rob()`.
- **Const-generic K-arity**: `bench_run` functions accept `[impl FnMut(); K]` and produce `BenchOut<K>`, allowing simultaneous benchmarking of `K` functions with interleaved execution to reduce time-dependent noise.
- **Feature-gated API surface**: `Comp`/`DuoOut` Wilcoxon methods gated behind `_experimental`. `load` gates the `BusyWork` workload. `test_support` gated behind `_test_support`. `bench_support` gated behind `_bench`.
- **[`LatencySrc`] trait** (in `multi::latency_src`): Abstracts latency measurement into iterators that yield `[FpSeconds; K]`. Concrete implementations `LatencySrc1`/`LatencySrc1b` (batch `n`) and `LatencySrc2`/`LatencySrc2b` (batch `n`) wrap closures and measure their wall-clock latency on each `next()` call. The crate-level `bench_run` module wraps single closures via `LatencySrc1`/`LatencySrc1b` (selected by the `batch: Option<usize>` argument) and delegates to the `multi` module, keeping the K=1 path uniform with K>1.
- **[`Status`] trait**: Benchmarks accept an owned `impl Status<'a>` that provides optional warm-up and execution progress callbacks. `NoStatus` is a no-op; `DefaultStatus<W: Write>` prints backspace-overwriting progress lines. The trait method `part_apply` partially applies `(est_time, est_count)` so the inner execution loop only receives the iteration index.
- **`stats_types` re-exports**: `pub mod stats_types` re-exports `AcceptedHyp`, `AltHyp`, `Ci`, `HypTestResult`, `PositionWrtCi` from `basic_stats::core` for convenience.
- **Duo parallel functions**: The `duo::bench_run` module provides `bench_run_parallel*` variants that run the two closures independently (non-interleaved) in parallel threads, as an alternative to the default interleaved execution.

### Sibling crates

- `basic_stats` (at `../basic-stats`) — normal, Student's t, Welch's t, Wilcoxon extensions
- `bench_diff` — uses `bench_utils` with `_bench_diff` feature for paired latency comparisons

### Tests

To run all non-latency-sensitive tests, use `./test.sh` (which uses `cargo nextest`).

Tests that call `latency::latency` or `Duration::elapsed()` to compute latencies should be executed with `cargo test -r` because latency measurements can be highly unreliable when the code is not compiled with release optimization. When run via `cargo nextest`, use `cargo nextest run -r` for the same reason.

Long-running validation tests are tagged with `_bench` and excluded from `./test.sh`. They can be run with `./validate.sh`, but you should NOT execute `./validate.sh` unless explicitly requested by the user.

A coverage report can be generated with `./coverage.sh` (requires `cargo-llvm-cov` and `llvm-tools-preview`).

Additional scripts (invoke only when explicitly requested):
- `./bench-criterion-*.sh` — criterion-based benchmark harnesses
- `./bench-validate_*.sh` — single long-validation benchmarks
- `./run-benches.sh` — run multiple criterion benchmarks
- `./validate1.sh` / `./validate-integr-repeat.sh` / `./validate-unit-repeat.sh` — targeted/repeated re-runs of individual `_bench`-gated tests (discovered via `list_bench_tests` examples), useful for chasing down flaky validation failures
- `./coverage-add-validations.sh` — like `./coverage.sh` but includes `_bench`-gated tests in the coverage run
- `./exec-omp.sh` / `./exec-herdr.sh` / `./exec-zellij.sh` — convenience launchers
