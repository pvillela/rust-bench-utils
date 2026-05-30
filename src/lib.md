Utilities for measuring latency and synthesizing workloads.

# Overview

`bench_utils` provides building blocks for latency benchmarking in Rust:

- Measure the wall-clock latency of any closure with [`latency`].
- Run a full benchmark — warm-up, execute, collect statistics — with [`bench_run`].
- Review and analyze benchmark results with [`BenchOut`].
- Tweak benchmark characteristics such as warm-up duration and status reporting frequency with [`BenchCfg`].
- Benchmark multiple closures, interleaving their execution, with the [`multi`] module.
- Compare two benchmark results with [`Comp`], which provides statistical tests and confidence intervals.
- Create synthetic loads with [`BusyWork`].

This library differentiates itself by:
- Providing programmatic access to benchmarking results as well as descriptive and inferential statistics for the results. By contrast, crates like [Criterion](https://crates.io/crates/criterion) and [Divan](https://crates.io/crates/divan) focus on the generation of outputs to `stdout` and graphics, rather than APIs to programmatically access and process their outputs. As a result, additional processing of outputs from those libraries (e.g., latency comparison between two functions) may require manual work or parsing of output files.
- Supporting the benchmarking of multiple functions side-by-side. By executing all the target functions in each iteration, the potential effects of time-dependent noise on the results are mitigated, reulting in more reliable latency comparisons among the target functions.

