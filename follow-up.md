# Follow-up Items

## Review `welch_ln_ci` doc comment
**DONE**
`comp.rs` — the doc for `Comp::welch_ln_ci` currently says:

> This is also the confidence interval for the difference of medians of logarithms under the above assumption.

Under the log-normal assumption, `ln(latency)` is normally distributed, and for a normal distribution the mean and median are the same value. So "difference of medians of logarithms" is technically equivalent to "difference of the means of the natural logarithms." Review whether the current phrasing is clear enough or should be revised.

## Review `fn sigfig` getter

`bench_cfg.rs` — the `sigfig()` getter was documented with a brief description. Review whether it needs more detail (e.g., explaining what significant figures mean for the HDR histogram).

## Missing getter methods for `BenchCfg`

`bench_cfg.rs` — the following fields have setter methods (builder pattern) but no public getter methods:

- `base_status_calibr` — has `with_status_calibr()` setter, no getter
- `status_millis` — has `with_status_millis()` setter, no getter

Consider adding public getter methods for consistency with `warmup_millis()`, `recording_unit()`, `reporting_unit()`, and `sigfig()`.

## Review `fn hist` accessor

`bench_out.rs` — the `hist()` accessor is gated behind `feature = "_test_support"`. It returns a reference to the raw HDR histogram. Review whether this feature gate is appropriate or if it should be made more broadly available.
