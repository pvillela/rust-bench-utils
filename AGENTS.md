# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build, test, and lint

```bash
./check.sh                  # cargo check --all-targets --all-features
./test.sh                   # cargo nextest run + doc tests (uses nextest, not cargo test)
./clippy.sh                 # cargo clippy --all-targets --all-features
./check-features.sh         # check multiple feature flag combinations
./test-features.sh          # test multiple feature flag combinations
./gen-doc.sh                # generate rustdoc (excludes common crate dependencies)
```

Run a single test with nextest:
```bash
cargo nextest run --lib --features _test_support,busy_work --target-dir target/test-target -- <test_name>
```

## Feature flags

This crate has complex feature gating with several tiers:

- **Public features**: `default` (= `basic_stats/normal`), `busy_work` (gates `sha2`-based CPU work)
- **Helper features**: `__null` enables the `basic_stats` dependency
- **Internal features**:
  - `_test_support` (= `basic_stats/_dev_utils` + `dep:regex`) — test-only utilities for this crate and friend crates
  - `_bench` (= `_test_support` + `busy_work` + `dep:criterion`) — used by some benches and long-running validation tests
  - `_test` (= `_test_support`) — used only by tests
  - `_experimental` (= `basic_stats/wilcoxon`) — functionality developed but not intended for public clients
  - `_bench_diff` (= `_experimental`) — bundles what the sibling `bench_diff` crate needs
  - `_ALL_NON_TEST` — union of all non-test features, used by build scripts

Most tests require `_test_support`. The feature `_bench_diff` is for use by the sibling `bench_diff` crate.

## Architecture

`bench_utils` is a Rust library for measuring latency and synthesizing workloads with predictable latency. It publishes to crates.io.
**Core data flow**: [`LatencySrc`] trait abstraction → `bench_run_x(src, exec_run_length, s)` → warm-up → execute `src.next()` repeatedly → record each latency in an HDR histogram → produce [`BenchOut`] with descriptive + inferential (Student's t on log-latencies) statistics.

The library supports benchmarking multiple functions simultaneously via const-generic `K`-arity: `BenchOut<K>` holds `K` `BenchOut` instances and `bench_run` accepts `[impl FnMut(); K]`.

### Key modules

| Module | Purpose |
|---|---|
 | `latency` | `latency(f)` function (wall-clock via `Instant`) and `LatencyUnit` enum (Milli/Micro/Nano). Also `fn_executions_per_milli` and `ltn_src_executions_per_milli` for calibration. |
 | `bench_cfg` | `BenchCfg` struct (warmup millis, recording/reporting units, sigfig, status interval). Configured via builder pattern. Also `RunLength` enum (Count, Duration, CountWithTimeout). `ltn_src_execs_per_second` method for calibration. |
 | `bench_out` | `BenchOut` — the result of benchmarking a single function. Holds an HDR histogram + raw sums/sum² for latencies and ln(latencies). Methods compute mean, stdev, median, Student's t-test/CIs on log-latencies (assumes log-normal distribution). Also `iter_with_counts()` and `iter_flat()` for iterating over histogram data. |
 | `bench_run` | `bench_run`, `bench_run_x`, `bench_run_with_status`, and `*_arg_cfg` variants. Thin wrappers that delegate to `multi::bench_run_x` via `LatencySrc1(f)`, accepting `s: impl Status<'a>`. |
 | `multi` | Const-generic K-arity benchmarking: `LatencySrc<const K: usize>` trait (iterator yielding `[Duration; K]`), concrete types `LatencySrc1` and `LatencySrc2`. `BenchOut<K>` (wraps `[BenchOut; K]`, derefs to `BenchOut` when `K=1`), `bench_run`, `bench_run_x`, etc. Also `BenchOut::from_iter` for constructing from duration iterators. |
 | `comp` | `Comp` compares two `BenchOut`s via `&BenchOut` references. Welch's t-test/CIs on difference of ln-means (i.e., ratio of medians). Wilcoxon rank sum behind `_experimental` feature. |
 | `status` | `Status<'a>` trait for benchmarking progress callbacks (warm-up and execution phases). `NoStatus` (no-op) and `DefaultStatus<W: Write>` (prints warmup/exec progress with backspace-overwriting). |
 | `summary_stats` | `SummaryStats` struct (mean, stdev, percentiles p1 through p99, min, max). Type alias `Timing = Histogram<u64>`. |
 | `fake_work` | `fake_work(Duration)` — thread sleep. For validating benchmarking frameworks. |
 | `busy_work` | `busy_work(u32)` — SHA-256 hashing loop. Feature-gated behind `busy_work`. Calibration functions available. |
 | `test_support` | Lognormal sample generators and `StringWriter`. Gated behind `_test_support` feature. Used by this crate's tests and friend crates. |
 | `bench_support` | `validate_latency_overhead` for verifying that latency measurement overhead is acceptable. Gated behind `_bench` feature. |

### Key design patterns

- **Stats delegation**: Inferential statistics (t-tests, CIs) are delegated to the sibling `basic_stats` crate. All results are checked via `.expect()` and always panic on error.
- **Log-normal assumption**: Latency distributions are treated as approximately log-normal. Statistics on `ln(latency)` are central to the API (Student's t on one sample, Welch's t for two-sample comparison).
- **HDR histogram**: Latencies are recorded into a resizable `hdrhistogram::Histogram<u64>`, which provides quantile/percentile queries.
- **Const-generic K-arity**: `bench_run` functions accept `[impl FnMut(); K]` and produce `BenchOut<K>`, allowing simultaneous benchmarking of `K` functions with interleaved execution to reduce time-dependent noise.
- **Feature-gated API surface**: Some `Comp` methods are gated behind `_experimental` (Wilcoxon). `test_support` is gated behind `_test_support`. `bench_support` is gated behind `_bench`.
- **[`LatencySrc`] trait**: Abstracts latency measurement into iterators that yield `[Duration; K]`. Concrete implementations [`LatencySrc1`] and [`LatencySrc2`] wrap closures and measure their wall-clock latency on each `next()` call. The `bench_run` module in `src/bench_run.rs` wraps single closures via `LatencySrc1(f)` and delegates to the `multi` module, keeping the K=1 path uniform with K>1.
- **[`Status`] trait**: Benchmarks accept an owned `impl Status<'a>` that provides optional warm-up and execution progress callbacks. [`NoStatus`] is a no-op; [`DefaultStatus<W: Write>`] prints backspace-overwriting progress lines. The trait method `part_apply` partially applies `(est_time, est_count)` so the inner execution loop only receives the iteration index.
- **`stats_types` re-exports**: `pub mod stats_types` re-exports `AcceptedHyp`, `AltHyp`, `Ci`, `HypTestResult`, `PositionWrtCi` from `basic_stats::core` for convenience.

### Sibling crates

- `basic_stats` (at `../basic-stats`) — normal, Student's t, Welch's t, Wilcoxon, AOK extensions
- `bench_diff` — uses `bench_utils` with `_bench_diff` feature for paired latency comparisons

### Tests

Tests in this crate that call the `latency::latency` function or call `Duration::elapsed()` to compute latencies should be executed with `cargo test -r` because latency measurements can be highly unreliable when the code is not compiled with release optimization.

Long-running validation tests are tagged with `_bench` and excluded from `./test.sh`. They can be run with `./validate.sh`, but you should NOT execute `./validate.sh` unless explicitly requested by the user.

To test the entire crate, use the `./test.sh` script.

A coverage report can be generated with `./coverage.sh`.
