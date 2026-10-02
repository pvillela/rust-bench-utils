This library supports the programmatic measurement of function latencies, including descriptive and inferential statistics.

## Overview

`bench_utils` **provides** building blocks for latency benchmarking in Rust:

- Measure the wall-clock latency of closures with [`latency`].
- Run a full benchmark — warm-up, execute, collect statistics — with [`bench_run`].
- Review and analyze benchmark results with [`BenchOut`].
- Control benchmark characteristics such as warm-up duration and run status reporting frequency with [`BenchCfg`].
- Benchmark multiple closures, in parallel or interleaving their executions, with the [`duo`] and [`multi`] modules.
- Compare two benchmark results with [`Comp`], which provides statistical tests and confidence intervals.
- Create synthetic loads with [`BusyWork`].

This library **differentiates** itself by:
- Providing programmatic access to benchmarking results as well as descriptive and inferential statistics for the results. By contrast, crates like [Criterion](https://crates.io/crates/criterion) and [Divan](https://crates.io/crates/divan) focus on the generation of outputs to `stdout` and graphics, rather than APIs to programmatically access and process their outputs. As a result, additional processing of outputs from those libraries (e.g., latency comparison between two functions) may require manual work or parsing of output files.
- Supporting the benchmarking of two functions in parallel on separate threads.
- Supporting the benchmarking of multiple functions side-by-side. By executing all the target functions in each iteration, the potential effects of time-dependent noise on the results are mitigated, reulting in more reliable latency comparisons among the target functions.

## Statistical context

### Lognormal distribution

The statistical estimators and tests in this library are based on the assumption that the latency distribution of a function or closure is *approximately* log-normal. This assumption is widely supported by performance analysis theory and empirical data.

### Noise and contamination ➔ robust statistics

Realistically, performance measurements can't be assumed to be observations from a pure lognormal distribution as they are subject to contamination and distortion from the computing environment, such as time-dependent noise and spikes. For this reason, this library uses robust statistical estimators and tests for the analysis of latency data.

### Batching

In latency benchmarking, the raw per-call execution time of a fast function cannot be measured reliably because measurement infrastructure (timer calls, loop control) consumes a non-trivial fraction of the wall-clock time. A common remedy is *batching*: call the target function *k* times per timed block and record the block time divided by *k*. The result is a *group mean* of *k* independent identically distributed (IID) latency values, where each latency value is approximately lognormally distributed (subject to distortion as discussed earlier).

The benchmarking functions in this library support optional batching and the statistical functions take batching into consideration.

