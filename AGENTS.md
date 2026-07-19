# AGENTS.md

This file provides guidance to coding agents when working with code in this repository.

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

- **Public features**: `default` (= `basic_stats/normal`), `load` (gates the `load` module: `sha2`-based CPU work via [`BusyWork`], with associated functions to calibrate `effort` to a target latency)
- **Helper features**: `__null` enables the (optional) `basic_stats` dependency
- **Internal features**:
  - `_test_support` (= `basic_stats/_dev_utils` + `basic_stats/detm_samp` + `basic_stats/rand_samp` + `dep:regex` + `dep:syn` + `dep:walkdir`) — test-only utilities for this crate and friend crates
  - `_bench` (= `_test_support` + `load` + `dep:criterion`) — used by some benches and long-running validation tests
  - `_test` (= `_test_support`) — used only by tests
  - `_experimental` (= `basic_stats/wilcoxon`) — functionality developed but not intended for public clients
  - `_bench_diff` (= `_experimental`) — bundles what the sibling `bench_diff` crate needs
  - `_ALL_NON_TEST` (= `default` + `load` + `_experimental` + `_bench_diff`) — union of all non-test features, used by build scripts
  - `_ignore` (= `[]`) — marks test modules to be skipped

Most tests require `_test_support`. The feature `_bench_diff` is for use by the sibling `bench_diff` crate.

## Architecture

`bench_utils` is a Rust library for measuring latency and synthesizing workloads with predictable latency. It publishes to crates.io.
**Core data flow**: [`LatencySrc`] trait abstraction → `bench_run_x(cfg, src, run_length, s)` → warm-up → execute `src.next()` repeatedly → record each latency (as [`FpSeconds`], a floating-point-seconds newtype) in an HDR histogram → produce [`BenchOut`] with descriptive + inferential (Student's t on log-latencies) statistics, including batching-bias-corrected robust estimators of the median.

The library supports benchmarking multiple functions simultaneously via const-generic `K`-arity: `BenchOut<K>` holds `K` `BenchOut` instances and `bench_run` accepts `[impl FnMut(); K]`.

### Key modules

| Module | Purpose |
|---|---|
| `latency` | `latency(f)`, `latency_n(f, n)`, `median_batch_latency(f, batch, n_batches)` (wall-clock via `Instant`); `FpSeconds` — a floating-point-seconds newtype over `f64` (with arithmetic ops and `Duration` conversions) used throughout the crate in place of `Duration` for finer-than-nanosecond granularity; `LatencyUnit` struct (associated consts `NANO`/`MICRO`/`MILLI`/`SEC`) for recording precision; `RunLength` enum (`Count`, `Time`, `CountWithTimeout`). `iters_per_sec` calibration is crate-private. |
| `bench_cfg` | `BenchCfg` struct (warmup millis, recording unit, sigfig, status interval). Configured via builder pattern. |
| `bench_out` | `BenchOut` — the result of benchmarking a single function. Holds an HDR histogram + raw sums/sum² for latencies and ln(latencies). Methods compute mean, stdev, median, Student's t-test/CIs on log-latencies (assumes log-normal distribution), plus batching-bias-corrected robust median estimators (`median_rob()`, backed by the Rousseeuw-Croux Q statistic and an S-estimator of scale, selecting between `median_rmom_estimator()`/`median_log_space_estimator()`). Also `iter_with_counts()` and `iter()` for iterating over histogram data. |
| `bench_run` | Crate-level `bench_run`, `bench_run_x`, `bench_run_with_status`, and `*_arg_cfg` variants. Each takes a `batch: Option<usize>` argument (`None` = no batching) and thinly delegates to `multi::bench_run_x` via `LatencySrc1`/`LatencySrc1b`. |
| `multi` | Directory module with `bench_out` (const-generic `BenchOut<K>` wrapping `[BenchOut; K]`, derefs to `BenchOut` when K=1), `bench_run` (const-generic `bench_run_x` etc.), and `latency_src` (`LatencySrc<K>` trait yielding `[FpSeconds; K]`, concrete types `LatencySrc1`/`LatencySrc1b`/`LatencySrc2`/`LatencySrc2b`, plus test-only sources). |
| `comp` | `Comp` compares two `BenchOut`s via `&BenchOut` references. Welch's t-test/CIs on difference of ln-means (i.e., ratio of medians), plus `diff_medians_f1_f2_rob()`/`ratio_medians_f1_f2_rob()` using the batching-bias-corrected robust medians. Wilcoxon rank sum behind `_experimental` feature. |
| `status` | `Status<'a>` trait for benchmarking progress callbacks (warm-up and execution phases). `NoStatus` (no-op) and `DefaultStatus<W: Write>` (prints warmup/exec progress with backspace-overwriting). |
| `summary_stats` | `SummaryStats` struct (mean, stdev, percentiles p1 through p99, min, max). |
| `load` | Feature-gated behind `load`. Directory module exposing `BusyWork` struct with `work(u32)` / `fun(effort)` (SHA-256 hashing loop, via the `sha2` crate) plus `Calibration`/calibration functions to map a target latency to an `effort`. (An alternate, arithmetic-loop `busy_work_simple` implementation exists in the module but is currently unused/not publicly exported.) |
| `duo` | Directory module with `bench_run` (K=2 convenience functions including `bench_run_parallel*` for non-interleaved execution) and `DuoOut` alias for `multi::BenchOut<2>` with `comp()`, `out_f1()`, `out_f2()` helpers and the same Welch/robust-median/Wilcoxon methods as `Comp`. |
| `test_support` | Directory module gated behind `_test_support`, used by this crate's tests and friend crates: `abs_rel_diff` (relative-difference traits/macros for `Duration`/`FpSeconds`), `miscellaneous` (lognormal sample generators, `StringWriter`, batch/count helpers), `test_finder` (`syn`/`walkdir`-based scanner that lists `_bench`-gated test functions, used by the `list_*_bench_tests` examples). |
| `dev_support` | `#[doc(hidden)]` directory module, always compiled (not feature-gated): `quicksort` (in-place quicksort/quickselect) and `dev_utils` (batching helpers `bsz`/`batched_run_length`, memoization helpers `memoized_value`/`memoized_fn`). |
| `bench_support` | `validate_latency_overhead` for verifying that latency measurement overhead is acceptable. Gated behind `_bench` feature. |

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
