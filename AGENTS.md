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

- **Public features**: `default` (= `_bench_run` + `__core`), `busy_work` (gates `sha2`-based CPU work), `criterion` (gates criterion bench harness)
- **Helper features**: `__core` enables `basic_stats/normal` and `basic_stats/aok`. `__null` enables `basic_stats` crate. `__stats_opt` enables `basic_stats/wilcoxon`.
- **Internal features**: `_bench_run` (enables `bench_run` module), `_test_support` (enables approx_eq macros + regex), `_test_support` (Wilcoxon + AOK + regex, for friend crates). `_bench_diff` bundles what the `bench_diff` sibling crate needs.

Most tests require `_test_support` + `_bench_run`. The feature `_bench_diff` is for use by the sibling `bench_diff` crate.

## Architecture

`bench_utils` is a Rust library for measuring latency and synthesizing workloads with predictable latency. It publishes to crates.io.

**Core data flow**: `bench_run(f, n)` → warm-up → execute `f` `n` times → record each latency in an HDR histogram → produce `BenchOut` with descriptive + inferential (Student's t on log-latencies) statistics.

### Key modules

| Module | Purpose |
|--------|---------|
| `latency` | `latency(f)` function (wall-clock via `Instant`) and `LatencyUnit` enum (Milli/Micro/Nano) |
| `bench_cfg` | Global `BenchCfg` behind a `Mutex` — warmup millis, recording/reporting units, sigfig. Configured via builder pattern and `set()`. |
| `bench_out` | `BenchOut` — the result of benchmarking. Holds an HDR histogram + raw sums/sum² for latencies and ln(latencies). Methods compute mean, stdev, median, Student's t-test/CIs on log-latencies (assumes log-normal distribution). |
| `bench_run` | `bench_run`, `bench_run_with_status`, `bench_run_x`. Warm-up then execute, populating `BenchOut`. |
| `comp` | `Comp` compares two `BenchOut`s. Welch's t-test/CIs on difference of ln-means (i.e., ratio of medians). Wilcoxon rank sum behind `_test_support`. |
| `summary_stats` | `SummaryStats` struct (mean, stdev, percentiles p1 through p99, min, max). Type alias `Timing = Histogram<u64>`. |
| `fake_work` | `fake_work(Duration)` — sleeps. For validating benchmarking frameworks. |
| `busy_work` | `busy_work(u32)` — SHA-256 hashing loop. Feature-gated. Calibration functions available. |
| `test_support` | Lognormal sample generators for tests. Gated behind `#[cfg(test)]` + `_bench_run`. |

### Key design patterns

- **Stats delegation**: Inferential statistics (t-tests, CIs) are delegated to the sibling `basic_stats` crate. All results are unwrapped via `.aok()` (an extension trait from `basic_stats` that panics on error with a message).
- **Log-normal assumption**: Latency distributions are treated as approximately log-normal. Statistics on `ln(latency)` are central to the API (Student's t on one sample, Welch's t for two-sample comparison).
- **HDR histogram**: Latencies are recorded into a resizable `hdrhistogram::Histogram<u64>`, which provides quantile/percentile queries.
- **Global config**: `BenchCfg` is stored in a `static Mutex`, accessed via `BenchCfg::get()` and mutated via builder `.set()`.
- **Feature-gated API surface**: Some `BenchOut` fields and `Comp` methods are only available with specific features (`_bench_diff`, `_test_support`, `_bench_run`).

### Sibling crates

- `basic_stats` (at `../basic-stats`) — normal, Student's t, Welch's t, Wilcoxon, AOK extensions
- `bench_diff` — uses `bench_utils` with `_bench_diff` feature for paired latency comparisons

### Tests

Tests in this crate that call the `latency::latency` function or call `Duration::elapsed()` to compute letencies should be executed with `cargo test -r` because latency measurements can be highly unreliable when the code is not compiled with release optimization.

To test the entire crate, use should primarily use @test.sh script.

