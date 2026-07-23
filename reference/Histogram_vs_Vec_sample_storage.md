# Replacing the HDR Histogram with a `Vec<f64>` in `BenchOut`: Pros, Cons, and Recommendation

> **Context.** `crate::BenchOut` (the result of `bench_run`, `src/bench_out.rs`) currently stores
> every recorded latency observation in a fixed-size `hdrhistogram::Histogram<u64>`, alongside six
> scalar running-moment accumulators. This document evaluates replacing that histogram with a
> growable `Vec<f64>` of raw latency samples, allocated with an initial capacity taken from a new
> `BenchCfg` field. It reasons across benchmark-duration / observation-count scenarios (with
> batching as the pivotal variable), weighs hybrid alternatives, and closes with a recommendation.
>
> All mechanical claims are traced to source; the scenario numbers are grounded in
> `analysis/bench_run_overhead.txt`; the quantization discussion builds on
> `analysis/Q_n_resolution_and_outlier_analysis.md`.

---

## 1. The proposal, and what it does and does not touch

The proposal is to swap the per-observation store — currently `hist: Histogram<u64>`
(`bench_out.rs:69`) — for a `Vec<f64>` of raw `FpSeconds`-derived samples, reserved up front from a
new `BenchCfg::initial_capacity` field (following the existing getter / `with_*` builder pattern in
`src/bench_cfg.rs`, read in `BenchOut::new`).

It is essential to be precise about what is actually in scope, because most of `BenchOut`'s
statistics **do not read the histogram at all**. Every observation is folded at record time into six
scalar accumulators — `sum`, `sum2`, `n_nz`, `sum_ln`, `sum2_ln` (plus the histogram) in
`capture_data_with_counts` (`bench_out.rs:142-162`):

- **Quantization-independent — already bypass the histogram, unaffected by the swap:** `mean`,
  `stdev`/`stdev_r`, `mean_ln_r`/`stdev_ln_r`, and every Student's-t method (`student_t`,
  `student_ln_t`, `student_ln_ci`, `student_median_ci`, `student_ln_test`, …). These build
  `SampleMoments` from the accumulators and touch the histogram only through `n_r() = hist.len()` —
  a plain count a `Vec` supplies via `.len()`.
- **Quantization-dependent — read the histogram's bucket counts / bucket geometry:** all
  `SummaryStats` percentiles p1…p99 and `min`/`max` (`summary_stats.rs:48-71`), `median_r`/`p50`
  (`bench_out.rs:262`), `iter`/`iter_with_counts` (`bench_out.rs:174-189`), the Wilcoxon `rank_sum`
  (`comp.rs:289`), and the entire **robust family**: `rousseeuw_croux_q_general` and its `_ns`/`_ls`
  wrappers, `pure_s_hat`/`s_hat`, `median_rob`, `median_rmom_estimator`,
  `median_log_space_estimator`, `mean_rob`.

So the swap only changes the quantile / robust-scale surface. The mean/stdev/Student's-t inferential
core is untouched (and would keep its accumulators regardless).

**Notation used throughout.** Two counts recur below:

- **`R` — the number of *recorded observations*:** the number of `src.next()` calls, i.e. the number
  of values handed to `capture_data`. This is what a `Vec<f64>` would store one element per. Because
  batching records one averaged value per batch, `R = ceil(executions / batch)` — not the number of
  function executions (see §4).
- **`D` — the number of *distinct occupied buckets*** in the current histogram: how many distinct
  values the robust estimators actually iterate over. It is bounded by the histogram's total bucket
  count (a few thousand at `sigfig=4`), regardless of how large `R` grows. With raw samples the
  corresponding count is `R` itself (see §3.2).

---

## 2. Pros of a raw-sample `Vec<f64>`

**2.1 Exactness — quantization disappears.** The histogram bins values into buckets whose relative
width is ~0.003–0.006 % at `sigfig=4`. Raw samples are exact, which:

- gives exact quantiles, median, `min`/`max`, and percentiles instead of bucket-edge readouts
  (`value_at_quantile` returns `highest_equivalent`, the *top* bucket edge — see
  `Q_n_resolution_and_outlier_analysis.md` §1.4);
- **eliminates the entire class of quantization workarounds** that the histogram forced: the
  `width/3` tied-pair approximation in `rousseeuw_croux_q_general` (`bench_out.rs:340-349`), the
  `median_equivalent` midpoint convention on inputs and readout, and the `DEFAULT_SIGFIG = 4` tuning
  that the `Q_n_resolution` investigation was devoted to justifying. With exact values, the
  same-bucket "tie fraction" that mechanically pins `Q_n` toward the quantization floor simply does
  not exist.

**2.2 Exact, tractable-in-principle robust estimators.** With sorted raw samples, `Q_n` and the
S-estimator can be computed exactly by the Croux–Rousseeuw O(n log n) selection algorithm, rather
than approximately from bucket midpoints. (See §4 for the important cost caveat.)

**2.3 Raw data unlocks future analyses.** Several open questions in this repository's analyses want
individual samples, not bucket counts: multimodality / clustering detection (the `Q_n ≪ IQR/1.349`
signature discussed in `Q_n_resolution_and_outlier_analysis.md` §2.4), Kolmogorov–Smirnov tests
(`Google_ Power_function_for_KS_test.md`), bootstrap resampling, and arbitrary re-quantiling. Raw
samples make all of these first-class instead of bucket-resolution approximations.

**2.4 Dependency and mental-model simplification.** A `Vec<f64>` is trivially easier to reason about
than HDR bucket geometry, and a full replacement could in principle drop the `hdrhistogram`
dependency — *caveat:* `comp::rank_sum` (`_experimental`) also iterates the histogram, so it would
need porting too.

**But weigh all of the above against one overriding caveat: the exactness upside is real but modest
in practice.** `sigfig=4` already suppresses quantization to the point that the residual
`Q_n`/`stdev_r` gaps are contamination-driven, not quantization artifacts
(`Q_n_resolution_and_outlier_analysis.md` §2), and batching — which fast closures should use anyway —
makes the whole question moot precisely where it would otherwise matter (§4 row 4). So the
raw-sample exactness win is smaller than it first appears, which is central to the recommendation in
§6.

---

## 3. Cons of a raw-sample `Vec<f64>`

**3.1 Memory becomes O(observations) and unbounded for `Time` runs.** The histogram is **fixed /
O(buckets)**: at `sigfig=4` with a 10 s high bound in nanoseconds and `auto(true)`, its counts array
tops out in the low single-digit MB (~2.75 MB) and is typically much smaller — **independent of how
many observations are recorded.** A `Vec<f64>` costs `8 × R` bytes where `R` is the number of
recorded observations, and for `Time`-bounded runs `R` is unbounded (`RunLength::Time` sets
`iters = usize::MAX`; the loop stops only on wall-clock, `multi/bench_run.rs`). `initial_capacity`
sets only a *floor*, not a *bound* — it cannot prevent the growth.

**3.2 Robust estimators degrade from O(D²) to O(R²).** `rousseeuw_croux_q_general` and `pure_s_hat`
form **pairwise-difference** sets and select an order statistic from them. Today they iterate over
*distinct occupied buckets* `D` (bounded by the bucket count, a few thousand) and accumulate the
pairwise differences into a scratch histogram — i.e. **the histogram's binning is what makes these
estimators tractable.** Over raw samples, the distinct-value count is `R` (up to hundreds of
millions), and a naïve pairwise computation is O(R²) — infeasible. Recovering tractability requires
*either* re-implementing the O(n log n) Croux–Rousseeuw algorithm (non-trivial, and still an
O(R log R) sort of a possibly gigabyte-scale array), *or* re-binning the raw samples into a
histogram on demand — which re-introduces exactly the structure being removed.

**3.3 Allocation churn, OOM risk, and indirect measurement perturbation.** `record_n` into a
resizable histogram is O(1) and essentially allocation-free once the range stabilizes (auto-resize
fires only a logarithmic number of times, each small). A growing `Vec` reallocates by doubling; near
the top of a large run that is a multi-hundred-MB `memcpy`, and an outright allocation failure
aborts the process. The `push` happens in the driver loop *after* the measured interval (the timed
region is inside `src.next()`), so this is **indirect** perturbation — memory pressure, page faults,
and cache eviction affecting *subsequent* measurements — not direct corruption of a single sample.
Still, it works against the tool's core job of clean latency measurement, precisely in the
long/fast runs where fidelity matters most. The fixed histogram has none of this.

**3.4 Quantile queries need a sort.** Percentiles/median over a `Vec` need an O(R log R) sort (or
quickselect, O(R)) versus the histogram's O(buckets) `value_at_quantile`. Minor next to §3.1–3.2,
but real at large `R`.

---

## 4. Scenario analysis: duration × observations × batching

The recorded-observation count `R` is the number of `src.next()` calls, **not** the number of
function executions. Two facts set `R`:

- **Batching divides before recording.** `LatencySrc1b::next` measures a whole batch and records
  `batch_latency / batch` as **one** observation (`latency_src.rs:113`). Thus
  `R = ceil(executions / batch)` — batching shrinks the stored set linearly.
- **Run length.** `Count(n)` → `R ≈ n/batch` (predictable); `Time(d)` → `R ≈ d × recording_rate`
  (unbounded); `CountWithTimeout` = the min of the two.

From `analysis/bench_run_overhead.txt` (empty closure), the unbatched recording rate is ~15M
observations/sec (~12.5 ns/observation): 100M observations in 6.58 s. A real closure of latency `L`
records at ≈ `1/(L + ~12 ns)` per second unbatched.

| # | Closure latency | batch | run length | R (recorded obs) | `Vec` memory (8·R) | Verdict |
|---|---|---:|---|---:|---:|---|
| 1 | 1 ms | 1 | `Time(5 s)` | ~5 K | ~40 KB | trivial — `Vec` fine |
| 2 | 1 µs | 1 | `Time(5 s)` | ~5 M | ~40 MB | borderline |
| 3 | 10 ns | 1 | `Time(5 s)` | ~200 M | **~1.6–1.8 GB** | unsafe |
| 4 | 10 ns | 1000 | `Time(5 s)` | ~230 K | ~1.8 MB | fine — *below* the histogram |
| 5 | 10 ns | 1 | `Time(60 s)` | ~2–3 B | **~15–24 GB** | OOM |
| 6 | any | b | `Count(N)` | `N/b` (known) | predictable | `initial_capacity` fully applies |

**Reading the table:**

- The memory blow-up is confined to **unbatched, fast-closure, `Time`-bounded** runs (rows 3, 5).
  Everything else is bounded and cheap.
- **`Count` runs are predictable** (row 6): `R = N/batch` is known before the run, so
  `initial_capacity` reserves exactly the right amount with zero reallocation. For `Count`-shaped
  workloads a `Vec` is genuinely well-behaved.
- **Batching is the pivotal variable** (rows 3 → 4). Fast closures should be batched anyway — batching
  is the crate's primary defense against per-call measurement-overhead bias (the ~12 ns floor).
  Batch size 1000 turns a 1.8 GB `Vec` into 1.8 MB, i.e. *smaller than the histogram it would
  replace*. So in the regime where the `Vec` is dangerous, correct usage already removes the danger
  — but the tool cannot assume users batch, and it must stay safe on the unbatched `Time` run a
  novice will inevitably write.

---

## 5. Alternatives and hybrids

**(A) Histogram primary + optional raw `Vec` for the robust estimators only.** Keep the histogram for
everything it does well; additionally retain raw samples to feed exact `Q_n`/`S`. Gains exactness
where it matters, but pays *double* storage and leaves the raw `Vec` unbounded — it does not solve
§3.1/§3.2.

**(B) Bounded, reservoir-sampled `Vec<f64>` (capacity from `BenchCfg`).** Cap the retained sample at
`initial_capacity` and use reservoir sampling (uniform random retention) once full. This:

- **bounds memory** by construction (rows 3/5 become safe);
- **removes quantization** (retained values are exact);
- **keeps `Q_n`/`S` tractable** — the subsample size is capped, so even O(m²) is fine, and O(m log m)
  trivially so;
- costs only a mild variance increase: `Q_n`/median on a uniform random subsample remain consistent
  estimators.

This is the compromise that captures the exactness benefit (§2.1–2.2) while defusing every con in §3.

Crucially, (B) also has **no adverse impact on the stability of individual latency measurements** —
and is arguably better on that axis than either the unbounded `Vec` or the current histogram:

- The reservoir insert/replace runs in the driver loop *after* `src.next()` returns, **outside the
  timed interval** (the measured region is inside `latency`/`latency_n`), so it can never corrupt the
  sample it is deciding whether to keep — the same "indirect only" situation as §3.3, with a far
  smaller effect.
- With a fixed capacity reserved once (`Vec::with_capacity` at construction/warmup), the main run does
  **zero allocation**: none of the doubling-`memcpy` spikes, OOM risk, or memory-pressure growth of an
  unbounded `Vec` (§3.3), and unlike the histogram (which auto-resizes occasionally) **no resize
  events at all**.
- Its only added per-iteration cost is a constant RNG draw + branch — a few ns, *uniform* across
  iterations, and outside the timed span. It adds a constant offset to inter-sample spacing rather
  than injecting variance (and stability is harmed by the *variability* of between-sample work, not
  its mean); it costs a little throughput, not accuracy.
- The one second-order effect — random-index writes evicting cache lines — is a wash versus the
  histogram, whose `record_n` already does a data-dependent write into a multi-MB counts array each
  observation. Sizing `initial_capacity` to keep the buffer cache-resident (e.g. ~100 K samples
  ≈ 800 KB, below the ~2.75 MB counts array) makes (B) gentler here than the status quo.

**(C) Mode-dependent store:** `Vec` for `Count`/`CountWithTimeout` (predictable `R`), histogram for
`Time`. Delivers exactness on bounded runs at no memory risk, but bifurcates the internal API and the
statistics' behavior by run mode — a meaningful complexity and testing cost.

---

## 6. Recommendation

**Keep the HDR histogram as `BenchOut`'s primary store. Do not adopt an unbounded `Vec<f64>` as a
wholesale replacement.**

The rationale, in order of weight:

1. **Bounded, workload-agnostic safety is a core property of a benchmarking tool.** The histogram's
   fixed few-MB footprint holds for *any* closure and *any* run length. A plain `Vec` forfeits that
   exactly on unbatched fast-closure `Time` runs (§4 rows 3, 5) — reaching multi-GB or OOM — which
   are among the most natural things a user will run.
2. **The histogram's binning is load-bearing for the robust estimators**, not incidental storage:
   it is what keeps `Q_n`/`S` tractable (§3.2). Replacing it with raw samples means either a
   substantial algorithmic rewrite or re-binning on demand.
3. **The exactness upside is real but modest in practice.** `sigfig=4` already suppresses
   quantization to the point that the remaining robust-estimator gaps are contamination-driven
   (`Q_n_resolution` §2), and batching — which fast closures should use anyway — makes the whole
   memory question moot precisely where it would otherwise bite (§4 row 4).

**If quantization-free robust estimation is a goal worth pursuing, adopt alternative (B): a bounded,
reservoir-sampled `Vec<f64>` whose capacity comes from a new `BenchCfg` field, as an optional
companion to the histogram — not a replacement for it.** That bounds memory by construction, removes
quantization from the estimators that care, keeps `Q_n`/`S` cheap on a capped subsample, and leaves
the histogram (and the already-histogram-free Student's-t core) intact. The `BenchCfg::initial_capacity`
field the proposal introduces is exactly the right knob — it just belongs on a *bounded* buffer, not
an unbounded one.

---

## See also

- `analysis/Q_n_resolution_and_outlier_analysis.md` — why `sigfig=4` is sufficient and why the
  residual `Q_n`/`stdev_r` gap is contamination-, not quantization-, driven.
- `analysis/bench_run_overhead.txt` — the measured recording rates underpinning §4.
- `analysis/Fable-edited-Optimum_estimators_of_mean_and_median.md` §3.3-3.4 — the histogram-based
  `Q_n`/`S` recipe whose tractability §3.2 discusses.
- Source: `src/bench_out.rs` (store, robust family), `src/summary_stats.rs` (percentiles),
  `src/bench_cfg.rs` (config/builder), `src/multi/bench_run.rs` + `src/multi/latency_src.rs`
  (record path and batching).
