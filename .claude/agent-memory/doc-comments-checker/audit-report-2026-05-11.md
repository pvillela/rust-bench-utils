---
name: "Doc Comment Audit 2026-05-11"
description: "Full audit results for all doc comments in bench_utils crate, including missing docs, inaccurate parameters, stale references, and copy-paste bugs"
type: reference
---

# Doc Comment Audit Report: bench_utils

Audit date: 2026-05-11

## Summary

| Metric | Count |
|--------|-------|
| Total Rust source files audited | 10 (in src/) |
| Fully documented public items | ~44 |
| Missing doc comments (critical) | 23 |
| Missing doc comments (minor) | 2 |
| Inaccurate/stale documentation | 14 |
| Broken intra-doc links | 3 |
| Doc test examples | 0 (none found in src/) |

## Detailed Findings

### Critical -- Missing Documentation

These items are `pub` and have no doc comment (`///`), making them invisible to users browsing rustdoc.

#### `BenchCfg` struct (bench_cfg.rs:56-65)
No doc comment on the crate's main configuration struct. Users cannot understand what fields it holds or how to construct/use it.

#### `BenchCfg` getter methods (bench_cfg.rs:94-107)
All four getters lack docs:
- `recording_unit()` at line 94
- `reporting_unit()` at line 98
- `conversion_factor()` at line 102
- `sigfig()` at line 106

#### `BenchCfg` builder/setter methods (bench_cfg.rs:116-144)
Six builder methods and one setter lack docs:
- `with_recording_unit()` at line 116
- `with_reporting_unit()` at line 121
- `with_sigfig()` at line 126
- `with_status_calibr()` at line 131
- `with_status_millis()` at line 136
- `set()` at line 141

#### `BenchCfg` calibration methods (bench_cfg.rs:146-177)
- `executions_per_milli()` at line 146
- `status_count()` at line 174

#### `RunLength` estimation methods (bench_cfg.rs:31-53)
Two methods have regular `//` comments instead of `///` doc comments:
- `estimated_count()` at line 31
- `estimated_time()` at line 43

#### `BenchCfg::get()` free function (bench_run.rs:21)
The primary way users access the global benchmark config -- no doc comment.

#### `Comp` accessor methods (comp.rs:46-51)
- `out_f1()` at line 46
- `out_f2()` at line 50

#### Feature-gated `BenchOut` accessors (bench_out.rs:248-282)
Six methods behind feature gates with no doc comments:
- `hist()` at line 250 (`#[cfg(feature = "_test_support")]`)
- `sum()` at line 256 (`#[cfg(feature = "_bench_diff")]`)
- `sum2()` at line 261 (`#[cfg(feature = "_bench_diff")]`)
- `n_ln()` at line 267 (`#[cfg(feature = "_bench_diff")]`)
- `sum_ln()` at line 273 (`#[cfg(feature = "_bench_diff")]`)
- `sum2_ln()` at line 279 (`#[cfg(feature = "_bench_diff")]`)

### Warnings -- Inaccurate or Stale Documentation

#### `bench_run_x` -- parameter names mismatch (bench_run.rs:63-76)
Doc says parameters `warmup_execs` and `exec_count` but the actual signature has `warmup_millis: u64` and `exec_run_length: RunLength`. The doc also fails to document the `execs_per_second: f64` parameter entirely.

#### `bench_run_x`, `bench_run`, `bench_run_with_status` -- broken intra-doc link (bench_run.rs:67,106,132)
All three doc comments link to `` [`get_warmup_millis`] `` which does not exist as a function. The correct reference is `BenchCfg::warmup_millis()`.

#### `bench_run` -- parameter name mismatch (bench_run.rs:110-112)
Doc says parameter is `exec_count` but actual signature has `exec_run_length: RunLength`.

#### `bench_run_with_status` -- parameter name mismatch (bench_run.rs:136-138)
Doc says parameter is `exec_count` but actual signature has `exec_run_length: RunLength`. Also says `header` argument is `exec_count` but the code passes a local variable of that name derived from `exec_run_length.estimated_count()`.

#### `calibrate_busy_work` -- parameter name mismatch (busy_work.rs:24)
Doc says `target_micros` but actual parameter is `target_latency: Duration`. Since the parameter is a `Duration` (not a microsecond count), the doc is misleading.

#### `calibrate_busy_work_x` -- parameter name mismatch (busy_work.rs:30-31)
Same issue: doc says `target_micros` but parameter is `target_latency: Duration`.

#### `Comp::welch_ln_p` -- copy-paste bug (comp.rs:124)
Doc says `median(latency(f1)) / median(latency(f1))` -- both sides reference `f1`. Should be `f1 / f2`.

#### `Comp::welch_ln_test` -- copy-paste bug (comp.rs:176)
Same issue: `median(latency(f1)) / median(latency(f1))` should be `f1 / f2`.

#### `Comp::welch_ln_ci` -- confusing phrasing (comp.rs:143)
Doc says "confidence interval for the difference of medians of logarithms." This is confusing; it should say "difference of the means of the natural logarithms" since it refers to `mean(ln(latency(f1))) - mean(ln(latency(f2)))`.

#### `Comp::welch_ln_df` -- grammar (comp.rs:117)
Doc says "this statistics" -- should be "this statistic".

#### `Comp::wilcoxon_rank_sum_test` -- double word (comp.rs:233)
Doc says "for for" -- duplicate word.

#### `BenchOut::stdev_ln` -- missing preposition (bench_out.rs:147)
Doc says "natural logarithms latencies" -- should be "natural logarithms of latencies" or "of the natural logarithms of latencies."

#### Feature-gated `Comp` methods missing feature mention (comp.rs:215-237)
Four `_test_support`-gated `Comp` methods (wilcoxon_rank_sum_w, wilcoxon_rank_sum_z, wilcoxon_rank_sum_p, wilcoxon_rank_sum_test) have doc comments but do not mention they require feature `_test_support`.

### Minor Issues

#### `SummaryStats` struct fields not documented (summary_stats.rs:22-37)
All 14 `pub` fields lack individual doc comments. The struct-level doc provides a summary, but rustdoc will show fields without descriptions.

#### `LatencyUnit` variants not documented (latency.rs:13-17)
The three variants (`Milli`, `Micro`, `Nano`) have no doc comments. Their meaning is self-evident but convention varies.

#### `RunLength` variants not documented (bench_cfg.rs:10-15)
Three variants (`Count`, `Duration`, `CountWithTimeout`) have no doc comments. The latter's tuple fields are particularly opaque.

### No Doc Tests Found

No `rust` code blocks were found inside `///` doc comments in any source file. The only doc-test-style example is in `examples/calibration.rs` (a doc comment instructing how to run the example, not a doc test).

## Summary of Proposed Fixes

### Most impactful fixes (in priority order):

1. Add doc comments to `BenchCfg` struct + all its methods (13 items)
2. Fix parameter name mismatches in `bench_run_x`, `bench_run`, `bench_run_with_status` docs
3. Fix broken intra-doc links (`get_warmup_millis` -> `BenchCfg::warmup_millis()`)
4. Fix `target_micros` -> `target_latency` in `calibrate_busy_work` / `calibrate_busy_work_x` docs
5. Fix copy-paste bug in `Comp::welch_ln_p` and `Comp::welch_ln_test` docs (`f1/f1` -> `f1/f2`)
6. Add doc comments to feature-gated `BenchOut` accessors
7. Add doc comments to `BenchCfg::get()`, `Comp::out_f1()`, `Comp::out_f2()`
8. Add feature requirement mentions to `_test_support`-gated `Comp` methods
