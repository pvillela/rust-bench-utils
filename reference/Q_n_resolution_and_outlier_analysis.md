# Why `DEFAULT_SIGFIG = 4` Is Sufficient, and Why the Remaining `Q_n`/`stdev_r` Gap Is Outlier-Driven

> **Context.** `BenchOut::rousseeuw_croux_q_ns`/`_ls` (`src/bench_out.rs`) implement the
> histogram-based Rousseeuw-Croux `Q_n` robust scale estimator described in
> `Fable-edited-Optimum_estimators_of_mean_and_median.md` §3.3-3.4. An earlier investigation in
> this repository's history found `Q_n` collapsing toward the HDR histogram's bucket-width
> quantization floor on real latency data, and fixed it by (a) recording same-bucket ("tied")
> pairs at an `E[|U1-U2|] = width/3` approximation instead of a literal `0`, and (b) raising
> `BenchCfg::DEFAULT_SIGFIG` from `3` to `4`. This document justifies (b) quantitatively, and
> separately explains why `Q_n` still reads well below the raw `stdev_r`/`stdev_ln_r` in several
> tiers of `examples/mean_and_median_estimates.rs` — establishing that this residual gap is the
> *correct* behavior of a 50%-breakdown robust estimator in the presence of outliers, not a defect.
>
> All numbers below are real output from `examples/q_resolution_analysis.rs`
> (`cargo run -r --example q_resolution_analysis --features load,_test`), which exercises the same
> target-latency/batch/sample-size tiers as `examples/mean_and_median_estimates.rs`. The workload's
> actual latencies depend on real OS/CPU scheduling, so exact figures are not bit-for-bit
> reproducible. Re-running reproduces §1's resolution findings stably; §2's per-tier outlier
> counts and ratios vary substantially from run to run, so §2's tables should be read as one
> illustrative draw — see the validation addendum (§3).

---

## 1. Why `sigfig = 4` is sufficient

### 1.1 Theory: HDR bucket resolution as a function of `sigfig`

This section establishes what `sigfig` actually buys: the precision with which an hdrhistogram
(crate `hdrhistogram` 7.5.4) can tell two nearby values apart, expressed as a fraction of their
magnitude. The bucket grid defined here is what §1.2's tie-fraction analysis and §1.4's
readout-rounding bound are both stated in terms of.

#### The bucket/sub-bucket scheme

An hdrhistogram records every value into one slot of a two-level structure. Two indices name a
slot, and both are used throughout this document:

- **Buckets**, indexed `n = 0, 1, 2, ...`, form an ever-widening sequence: bucket `n` covers a
  value range exactly `2×` as wide as bucket `n - 1`. Bucket 0 anchors the scheme, having no
  predecessor to widen from.
- **Sub-buckets**, indexed `m`, subdivide each bucket into `sub_bucket_count` equal-width slots.
  `sub_bucket_count` is fixed once at construction from `sigfig` (below) and is the *same for every
  bucket* — it is not recomputed per bucket. All values landing in one sub-bucket become
  indistinguishable: they increment a single shared count. This is the histogram's only source of
  quantization error.
- **For `n ≥ 1`, only the top half of a bucket's sub-buckets is populated** — `m` from
  `sub_bucket_half_count` to `sub_bucket_count - 1`, where `sub_bucket_half_count =
  sub_bucket_count / 2` (`hdrhistogram-7.5.4/src/lib.rs:784`). Lower indices would cover values that
  bucket `n - 1` already covers at twice the precision, so the histogram routes such values there
  instead and bucket `n`'s bottom half never receives anything. **Bucket 0 is the exception**: with
  no bucket `-1` beneath it, it uses the full index range `m = 0, ..., sub_bucket_count - 1`.

Two construction-time scalars complete the picture:

- **`lowestDiscernibleValue`** — the crate's API calls it `low`, the first argument to
  `Histogram::new_with_bounds(low, high, sigfig)` (`lib.rs:736`): the smallest value the histogram
  is required to distinguish from 0. It fixes the finest *absolute* step the histogram can take.
- **`unit_magnitude` = `floor(log2(low))`** (`lib.rs:760`): the exponent offset appearing in every
  formula below. It sets a *floor* on quantization — values differing only in their low
  `unit_magnitude` bits are never distinguished, anywhere in the histogram. That floor is not the
  main reason values share a sub-bucket, though: a sub-bucket in bucket `n` spans
  `2^(n + unit_magnitude)` values (its `equivalent_range`, below), so sharing grows with the bucket
  index and `unit_magnitude` is merely the `n = 0` case. Note that `sigfig` does not appear in that
  width — it acts one level up, by fixing *which* bucket a given value lands in: a larger
  `sub_bucket_count` places that value in a lower-indexed bucket, and hence in a narrower
  sub-bucket. `unit_magnitude` itself vanishes in this crate, where `low = 1` gives
  `unit_magnitude = 0` (see the specialization closing this section).

#### Bucket and sub-bucket boundaries

The hdrhistogram API names four quantities on this grid; §1.4 relies on all of them:

- **`lowest_equivalent(v)`** (`lib.rs:1464-1468`): the smallest value recorded into the same slot
  (same `n` and `m`) as `v` — the low, inclusive edge of `v`'s equivalent range.
- **`highest_equivalent(v)`** (`lib.rs:1475-1481`): the largest value recorded into that same slot —
  the high, inclusive edge. This is the "high point" the formulas below compute.
- **`next_non_equivalent(v)`** (`lib.rs:1500-1503`): the smallest value landing in a *different*
  slot — one unit past the top of `v`'s range, so `highest_equivalent(v) = next_non_equivalent(v) - 1`.
- **`equivalent_range(v)`** (`lib.rs:1508-1511`): the width, in raw value units, of the range of
  values collapsing into `v`'s slot — i.e. `next_non_equivalent(v) - lowest_equivalent(v)`. It
  equals `2^(n + unit_magnitude)` for `v`'s bucket `n`, so it doubles from each bucket to the next.

Three quantities fix the grid, and the fourth follows from them:

- **Width of a sub-bucket in bucket `n`** — the same for every sub-bucket of that bucket, so it
  does not depend on `m`; this is the `equivalent_range` defined above:
  `W(n) = 2^(n + unit_magnitude)`
- **Low point of sub-bucket `m` within bucket `n`**, inclusive (the crate computes it as
  `value_from_loc`, `lib.rs:1599-1606`):
  `L(n, m) = m · 2^(n + unit_magnitude)`
- **High point of sub-bucket `m` within bucket `n`**, inclusive — its low point plus its width,
  minus one, i.e. `L(n, m) + W(n) − 1`:
  `H(n, m) = (m + 1) · 2^(n + unit_magnitude) − 1`
- **High point of bucket `n`** (its overall top edge, i.e. `H(n, sub_bucket_count − 1)`):
  `H(n) = sub_bucket_count · 2^(n + unit_magnitude) − 1`

These are exact integer formulas, not approximations. Let `n` and `m` be the bucket and sub-bucket
`v` falls into. Then `lowest_equivalent(v)` is precisely `L(n, m)` — the API resolves `v` to those
two indices and returns `value_from_loc` of them (`lib.rs:1464-1468`), which *is* the low-point
formula. And `highest_equivalent(v)` is precisely `H(n, m)`, since
`highest_equivalent(v) = next_non_equivalent(v) - 1 = lowest_equivalent(v) + equivalent_range(v) - 1`
(`lib.rs:1475-1503`) reduces algebraically to the same expression.

#### How `sigfig` fixes `sub_bucket_count`

```
largest                     = 2 * 10^sigfig
sub_bucket_count_magnitude  = ceil(log2(largest))
sub_bucket_count            = 2 ^ sub_bucket_count_magnitude
```

(`lib.rs:758`, `770-772`.) `largest` is the largest value to which the histogram guarantees
single-unit resolution — the direct encoding of "`sigfig` significant decimal digits". Because
`sub_bucket_count` must be a power of two for direct indexing, it is `largest` **rounded up** to
the next one; that rounding is what makes the per-`sigfig` gain uneven, as shown below.

#### What this means for resolution

Two different quantities are easy to conflate here, so state both:

**Absolute** sub-bucket width is `W(n)` — smallest in bucket 0 and doubling with every bucket.
This is the only sense in which bucket 0 is the "finest-resolution" bucket.

**Relative** sub-bucket width — width as a fraction of the value held, and the quantity `sigfig`
actually controls — is *independent of the bucket*. Taking the sub-bucket's low edge `L(n, m)` as
the reference value, the exponent cancels:

```
W(n) / L(n, m)  =  2^(n + unit_magnitude) / (m · 2^(n + unit_magnitude))  =  1/m
```

Relative width depends only on the sub-bucket index `m`, never on `n`. Over the populated index
range of any bucket `n ≥ 1` (`m` from `sub_bucket_half_count` to `sub_bucket_count - 1`), `1/m`
therefore sweeps exactly `~2/sub_bucket_count` down to `~1/sub_bucket_count` — the same band in
every bucket, and the same band bucket 0 covers over its own top half. This is why one `sigfig`
dial sets a single relative precision across the histogram's *entire* dynamic range rather than
just near its smallest values: going from bucket `n` to `n + 1` doubles a sub-bucket's absolute
width and the value it sits at in equal measure, leaving the ratio fixed. Each unit increase in
`sub_bucket_count_magnitude` — i.e. each *doubling* of `sub_bucket_count` — halves that band.
(`sub_bucket_count` is always a power of two, so a "unit increase in `sub_bucket_count`" is not a
meaningful step; the magnitude is the dial.) This band is what "resolution" means for the rest of
this document: how finely two nearby latencies can be told apart, as a fraction of their magnitude.

The one region outside the band is **bucket 0's bottom half** (`m` from `1` to
`sub_bucket_half_count - 1`), which no higher bucket duplicates: there `1/m` grows without bound,
reaching 100% at `m = 1`. Bounding that region is exactly what `lowestDiscernibleValue` is for —
values below it get no resolution guarantee. Concretely:

| `sigfig` | `largest = 2·10^sigfig` | `sub_bucket_count_magnitude` | `sub_bucket_count` | relative sub-bucket width (any bucket, populated range) |
|---|---|---|---|---|
| 3 | 2,000 | 11 | 2,048 | ~4.9e-4 – 9.8e-4 (~0.05–0.1%) |
| 4 | 20,000 | 15 | 32,768 | ~3.1e-5 – 6.1e-5 (~0.003–0.006%) |
| 5 | 200,000 | 18 | 262,144 | ~3.8e-6 – 7.6e-6 (~0.0004–0.0008%) |

Because `sub_bucket_count` is rounded up to the next power of two, the resolution gain per
`sigfig` step is not a flat 10×: **going 3→4 is a 16× resolution gain** (`32768 / 2048 = 16`),
while going 4→5 is only an **8× gain** (`262144 / 32768 = 8`). In other words, the jump from 3 to 4
buys *more* headroom than the jump from 4 to 5 does — a first hint that 4 already captures most of
the low-hanging benefit.

#### Specialization to this crate: `unit_magnitude = 0`

The formulas above carry `unit_magnitude` symbolically because that is what the hdrhistogram source
does, but it is always **zero** here. `new_hdrhist` (`src/bench_out.rs:11-17`) builds every
histogram via `Histogram::<u64>::new_with_max(hist_high, hist_sigfig)`, and `new_with_max(high,
sigfig)` is defined as `new_with_bounds(1, high, sigfig)` (`lib.rs:712-714`) — so `low = 1` and
`unit_magnitude = floor(log2(1)) = 0`. Every formula collapses accordingly:

- `W(n) = 2^n` (the `equivalent_range` in bucket `n`)
- `L(n, m) = m · 2^n`
- `H(n, m) = (m + 1) · 2^n − 1`
- `H(n) = sub_bucket_count · 2^n − 1`

In particular, bucket 0 has an equivalent range of `2^0 = 1`: single-unit resolution for every value
below `sub_bucket_count` (32,768 recording units at `sigfig = 4`). This is the fact §1.4 depends on
when it bounds the `value_at_quantile` readout bias to zero in the "unit-resolution range".
`BenchOut`'s histograms are constructed with `high = 20_000_000` recording units
(`src/bench_out.rs:83`).

### 1.2 Empirical validation: sigfig 3 vs 4 vs 5 on identical samples

For each tier, `q_resolution_analysis.rs` captures one real sample at `sigfig=5` (the finest of
the three, serving as a closest-to-ground-truth reference for this exercise) via
`BenchOut::iter()`, then re-records that *same* sample into fresh `BenchOut`s built at `sigfig=3`
and `sigfig=4`. This isolates the effect of bucket resolution alone — all three histograms see
identical underlying data, so differences are attributable to quantization, not to
measurement-to-measurement noise. (Re-bucketizing *up* from a captured sample can't recover
information already lost by a coarser capture; that's precisely why the base capture uses the
*finest* of the three resolutions under test.)

Also tabulated: the **same-bucket tie fraction** `Σ C(count_i,2) / C(n,2)` — the fraction of all
pairwise comparisons that land between two observations sharing an HDR bucket, computed directly
from `BenchOut::iter_with_counts()`. This is the quantity that mechanically drives floor-pinning:
`Q_n`'s target rank sits at roughly the 25th percentile of pairwise differences (`r/C(g,2)` ranges
from ≈0.251 at `n=1000` to ≈0.290 at `n=20` across these tiers), so once the tie fraction
approaches that rank, the order statistic
starts being read out of the tied-pair mass instead of a genuine cross-bucket difference.

| tier | sigfig | `rc_q_ns` | tie fraction |
|---|---|---:|---:|
| 1ns (k=100000, n=100) | 3 | 102.7ps | 0.069 |
| | 4 | 100.6ps | 0.004 |
| | 5 | 100.6ps | 0.004 |
| 10ns (k=10000, n=100) | 3 | 171.2ps | 0.083 |
| | 4 | 179.8ps | 0.006 |
| | 5 | 179.8ps | 0.006 |
| 100ns (k=1000, n=100) | 3 | 2.466ns | 0.068 |
| | 4 | 2.526ns | 0.005 |
| | 5 | 2.528ns | 0.001 |
| 1µs (k=100, n=100) | 3 | 1.096ns | 0.182 |
| | 4 | 0.822ns | 0.015 |
| | 5 | 0.813ns | 0.003 |
| 10µs (k=10, n=100) | 3 | 17.55ns | 0.215 |
| | 4 | 8.77ns | 0.017 |
| | 5 | 9.18ns | 0.004 |
| 100µs (unbatched, n=1000) | 3 | 3.049µs | 0.071 |
| | 4 | 2.983µs | 0.005 |
| | 5 | 2.979µs | 0.001 |
| 100µs (k=10, n=100) | 3 | 2.247µs | 0.023 |
| | 4 | 2.297µs | 0.001 |
| | 5 | 2.297µs | 0.0004 |
| 1ms (unbatched, n=500) | 3 | 18.52µs | 0.043 |
| | 4 | 18.21µs | 0.004 |
| | 5 | 18.19µs | 0.001 |
| 1ms (k=10, n=50) | 3 | 15.17µs | 0.009 |
| | 4 | 15.02µs | 0.000 |
| | 5 | 15.00µs | 0.000 |
| 10ms (unbatched, n=200) | 3 | 109.8µs | 0.022 |
| | 4 | 108.6µs | 0.002 |
| | 5 | 108.3µs | 0.0001 |
| 10ms (k=10, n=20) | 3 | 47.02µs | 0.042 |
| | 4 | 46.99µs | 0.000 |
| | 5 | 47.23µs | 0.000 |

**sigfig=4 vs sigfig=5** (the direct "what do we lose by not using 5" comparison): the largest
observed deviation across all 11 tiers is **4.5%** (the 10µs tier); every other tier is within
**1.1%**, several matching to 4 significant figures. Validation re-runs (§3) reproduce this
pattern — the worst tier (again 10µs, in one re-run) deviated by up to ~11%, while every tier at
100µs and above stayed within ~0.2–5%. The tie fraction at sigfig=4 stays at or below ~0.03 in
every run observed, comfortably below the ~0.25 rank threshold in every tier tested. This is
direct evidence that **sigfig=5 buys negligible additional accuracy over sigfig=4** for this
workload family.

**sigfig=3 vs sigfig=4**: mostly small (≤5%) *except* the 1µs tier (**+33%**) and the 10µs tier
(**+100%**, i.e. 2×) — exactly the tiers whose sigfig=3 tie fraction (0.182, 0.215) sits closest to
the ~0.25 rank threshold. This reproduces, mechanistically, why the earlier investigation flagged
those specific tiers as broken at `sigfig=3`, and confirms `sigfig=4`'s low tie fraction is why
raising it to 4 was sufficient to fix them. Note the sigfig=3 error is not always an inflation:
validation re-runs showed deviations in *both* directions (e.g. **−24%** in the 100µs unbatched
tier, whose sigfig=3 tie fraction ≈0.26 exceeded the target rank, so the order statistic was read
from the `width/3` tied-pair mass), which can under- or over-shoot the true difference.

### 1.3 Conclusion

`sigfig=4` pushes the same-bucket tie fraction low enough, across every tier tested, that `Q_n`'s
order statistic reads a genuine cross-bucket difference rather than the tied-pair fallback. Going
to `sigfig=5` changes the result by at most a few percent — consistent with §1.1's observation that
the 4→5 step buys less raw resolution (8×) than the 3→4 step did (16×). `sigfig=4` is therefore the
better default: it captures effectively all of the accuracy gain available from finer HDR
resolution, at 8× less memory than `sigfig=5` would cost.

### 1.4 A related readout detail: `value_at_quantile` rounding semantics

While validating the above, the readout side of the `Q_n` computation was also checked against the
hdrhistogram 7.5.4 source. For any quantile > 0, `Histogram::value_at_quantile`
(`hdrhistogram-7.5.4/src/lib.rs:1340-1369`) computes `count_at_quantile = ceil(quantile ·
total_count)`, walks the counts array to the first slot whose cumulative count reaches it, and
returns **`highest_equivalent`** of that slot's value — the *top* edge of that slot's equivalent
range (`next_non_equivalent − 1`, `lib.rs:1475-1481`) — not the slot's midpoint. (The slot here is
a single sub-bucket, §1.1; its width is one `equivalent_range`, not a whole bucket's span.)

`rousseeuw_croux_q_general` records its pairwise differences from `median_equivalent` (midpoint)
inputs, so a top-of-slot readout carries a slight systematic upward bias. Two qualifications
bound it:

- **Zero bias in the unit-resolution range.** `equivalent_range = 2^(unit_magnitude +
  bucket_index)` (`lib.rs:1508-1511`) — here simply `2^bucket_index`, since `unit_magnitude = 0` in
  this crate (§1.1) — which is 1 throughout bucket 0, i.e. for values below `sub_bucket_count`
  (32,768 units at sigfig 4), where `highest_equivalent(v) = v` exactly. This covers small pairwise
  differences, including most of the `width/3` tied-pair fallback mass.
- **Above that range**, the bias is at most one sub-bucket width (one `equivalent_range`) relative
  to the sub-bucket's bottom edge, ~half a sub-bucket relative to its midpoint — i.e.
  ≤ ~0.003–0.006% of the Q value at `sigfig=4`, matching the relative sub-bucket width tabulated in
  §1.1 and orders of magnitude below `Q_n`'s own sampling noise at these sample sizes. It cannot
  explain any effect discussed in this document.

`rousseeuw_croux_q_general` now wraps the readout as
`rc_hist.median_equivalent(rc_hist.value_at_quantile(rank_quantile))`, making the output
convention consistent with the midpoint convention used on the inputs (a no-op in the
unit-resolution range).

A second, even smaller subtlety: the exact-rank intent (`r = C(h,2)`) passes through floating
point — `ceil(rank_quantile · total_count)` can land on rank `r+1` instead of `r` when
`(r/total)·total` rounds an ulp above `r`. The effect is one adjacent order statistic among
`C(g,2)` pairwise differences; left as is.

---

## 2. Outlier analysis: is the remaining `Q_n`/`stdev_r` gap a defect?

`examples/mean_and_median_estimates.rs` shows `rc_q_ns` reading well below the raw `stdev_r` in
several tiers even at `sigfig=4`. `Q_n` has a 50% breakdown point by construction (Rousseeuw-Croux;
see `Fable-edited-Optimum_estimators_of_mean_and_median.md` §3.3) — it is *designed* to ignore up
to half the sample if that half is a contaminating tail, whereas `stdev_r` (an ordinary
sum-of-squares statistic) has 0% breakdown and is inflated by any outliers present. If the tiers
with the largest gaps are also the ones with genuine outlier contamination, the gap is expected
behavior, not a defect.

### 2.1 Technique: Tukey fences and Winsorizing

**Tukey fences** (Tukey, *Exploratory Data Analysis*, 1977): given the sample's own quartiles,
`IQR = p75 - p25`, values outside `[p25 - 1.5·IQR, p75 + 1.5·IQR]` are flagged as outliers. It's a
classical, distribution-agnostic rule that only depends on quantiles the sample already has, not on
an assumed parametric noise model.

**Winsorizing vs. trimming.** Given a set of flagged outliers, two classical remedies exist:
*trimming* discards them (shrinking `n`), while *Winsorizing* clamps them to the fence value
(preserving `n`). We use Winsorizing here because trimmed-sample SD's bias depends on the (unknown)
trim fraction and tail shape in a way that's awkward to reason about across tiers with wildly
different outlier counts, whereas Winsorized SD is a standard robust-scale technique in its own
right — conceptually closer to `Q_n` in spirit (both cap the influence of extreme values via order
statistics rather than assuming a parametric contamination model). Winsorized SD is *not* an
unbiased estimator of the population SD — clamping extreme values necessarily shrinks the computed
spread relative to the true (contaminated) population — but that downward bias is exactly the
point of this comparison: we want to know whether removing outliers' influence brings the
recomputed scale close to `Q_n`, not to unbiasedly re-estimate `stdev_r`.

For cross-validation, we also report **`IQR / 1.349`**, a second, threshold-free robust-σ
reference (the constant makes it a consistent estimator of σ under normality) that is a closer
conceptual sibling of `Q_n` — both are pure order-statistic constructions — whereas Winsorized SD
is a hybrid (a moment-based computation applied after an order-statistic-based clamp).

### 2.2 Computation

Per tier, on the `sigfig=4` sample expanded via `BenchOut::iter()` (the same bucket-quantized data
`Q_n` itself operates on internally — chosen for a fair like-for-like comparison; this differs
negligibly from `stdev_r()`'s exact pre-quantization sums, per §1's resolution finding):

1. `p25`/`p75` from `BenchOut::summary()`; compute the Tukey fences.
2. Flag/count observations outside the fences; Winsorize (clamp) them.
3. Compute the SD of the raw and the Winsorized sample; compute `IQR / 1.349`.
4. Compare all three against `rc_q_ns`.

### 2.3 Results

| tier (n) | outliers | `stdev_r` (raw) | `stdev_r` (Winsorized) | `IQR/1.349` | `rc_q_ns` | `Q_n`/raw | `Q_n`/Wins. | `Q_n`/(IQR/1.349) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1ns (100) | 17 | 1.271ns | 0.549ns | 0.424ns | 0.101ns | 0.079 | 0.183 | 0.237 |
| 10ns (100) | 8 | 0.539ns | 0.419ns | 0.415ns | 0.180ns | 0.333 | 0.429 | 0.433 |
| 100ns (100) | 18 | 4.636ns | 2.946ns | 2.025ns | 2.526ns | 0.545 | 0.857 | 1.247 |
| 1µs (100) | 19 | 41.44ns | 8.784ns | 6.642ns | 0.822ns | 0.020 | 0.094 | 0.124 |
| 10µs (100) | 13 | 390.9ns | 197.7ns | 162.8ns | 8.768ns | 0.022 | 0.044 | 0.054 |
| 100µs, unbatched (1000) | 3 | 7.465µs | 5.495µs | 8.037µs | 2.983µs | 0.400 | 0.543 | 0.371 |
| 100µs, k=10 (100) | 0 | 6.108µs | 6.108µs | 9.437µs | 2.297µs | 0.376 | 0.376 | 0.243 |
| 1ms, unbatched (500) | 62 | 61.76µs | 33.46µs | 29.46µs | 18.21µs | 0.295 | 0.544 | 0.618 |
| 1ms, k=10 (50) | 0 | 14.76µs | 14.76µs | 13.09µs | 15.02µs | 1.018 | 1.018 | 1.147 |
| 10ms, unbatched (200) | 16 | 161.6µs | 115.5µs | 99.49µs | 108.6µs | 0.672 | 0.940 | 1.092 |
| 10ms, k=10 (20) | 0 | 45.86µs | 45.86µs | 40.42µs | 46.99µs | 1.025 | 1.025 | 1.163 |

### 2.4 Interpretation

**Clean tiers show `Q_n` is not mechanically pinned low — but 0 Tukey outliers does *not*
guarantee `Q_n ≈ stdev_r`.** Two of the three tiers with 0 Tukey outliers (1ms/k=10 and
10ms/k=10) have `rc_q_ns` within ~2% of the raw `stdev_r` (ratios 1.018, 1.025), demonstrating
that `Q_n` reduces to essentially the ordinary SD on a genuinely clean, unimodal sample. The third
zero-outlier tier (100µs/k=10), however, reads `Q_n/stdev_r = 0.376`, and validation re-runs (§3)
put zero-outlier tiers anywhere from **0.19 to 1.03**. The low-ratio cases carry a telltale
signature: `IQR/1.349` *exceeds* the raw `stdev_r` (9.44µs vs 6.11µs in the table's 100µs/k=10
row) — a wide-shouldered or **multi-modal** sample rather than a tailed one. When the sample
clusters into groups (e.g. from CPU-frequency or scheduler state shifting mid-run), the
~25th-percentile pairwise difference lands *within* a cluster, so `Q_n` legitimately reports
within-cluster scale while SD and IQR report across-cluster spread; a Tukey fence, which only
flags points far outside the quartiles, is blind to that structure. So the sanity check is
narrower than "zero outliers ⇒ agreement": clean *unimodal* tiers show `Q_n` tracking `stdev_r`
closely, while clustered tiers show `Q_n` below it for reasons unrelated to either quantization
(ruled out by §1) or heavy tails.

**For workloads with a substantial sample size and a clear tail (100µs/1ms/10ms unbatched), the**
**gap closes substantially once compared to a robust reference.** E.g. the 10ms unbatched tier:
`rc_q_ns/stdev_r(raw) = 0.672`, but `rc_q_ns/stdev_r(Winsorized) = 0.940` and
`rc_q_ns/(IQR/1.349) = 1.092` — both close to 1. The 1ms unbatched tier moves from `0.295` (raw) to
`0.544`/`0.618` (robust references). This directly substantiates that the raw-`stdev_r` gap in
these tiers is outlier-driven: once the outliers' influence is capped by an independent technique
(Tukey/Winsorizing), the remaining scale estimate agrees with `Q_n` reasonably well. Validation
re-runs reproduce this closure in most contaminated tiers (often to 0.8–1.2 against `IQR/1.349`),
but not reliably in the large-`n` unbatched tiers: individual runs showed the 1ms-unbatched or
100µs-unbatched tier closing only to ~0.07–0.10, with the same `Q_n ≪ IQR/1.349` multi-modality
signature discussed above — a long run gives system-state drift more opportunity to split the
sample into clusters.

**For the smallest-effort tiers (1ns–10µs), the gap only partially closes.** Even after
Winsorizing, ratios remain in the **0.02–0.5** range (vs. **0.02–0.55** raw) — Winsorizing helps,
but doesn't fully reconcile `Q_n` with either robust reference. The most plausible explanation:
at `effort` values of 1–400 (sub-microsecond target latencies), the *true* computational latency of
the workload is at or below the OS/timer measurement noise floor, so a large and possibly
multi-modal fraction of observations is affected by scheduler/cache jitter rather than a small,
cleanly-separable tail. A single-pass 1.5×IQR fence is a comparatively weak diagnostic against that
kind of pervasive contamination (and is itself subject to some masking, since the fences are
computed from a sample the contamination has already inflated) — this is a limitation of the
Tukey/Winsorizing diagnostic at this scale, not evidence that `Q_n` is misbehaving; `Q_n`'s 50%
breakdown point handles this regime by design, which is exactly why it is the estimator used in
this crate's robust median/mean corrections rather than an ad hoc trimming rule. That said,
validation re-runs (§3) show the tier boundary is not sharp: in one re-run the 100ns–10µs tiers
closed fully (0.80–1.20 against `IQR/1.349`) while a ≥100µs unbatched tier did not. Which tiers
fail to close is driven by whether the system state stayed stable (unimodal noise) or shifted
(clustered sample) during that particular run, which correlates only loosely with target latency —
though the sub-10µs tiers, sitting nearest the measurement noise floor, are the most frequently
affected.

### 2.5 Conclusion

The large `rc_q_ns`/`stdev_r` gaps seen in `mean_and_median_estimates.rs` are consistent with
genuine contamination — Tukey-flaggable outliers and, more broadly, clustered/multi-modal
structure that Tukey fences don't flag — not a residual defect in `rousseeuw_croux_q_general`:
clean unimodal tiers show `Q_n` tracking raw `stdev_r` closely; most contaminated tiers show the
gap closing substantially once compared against independent robust references (Winsorized SD,
`IQR/1.349`); and where the gap persists (in different tiers on different runs), the
`Q_n ≪ IQR/1.349` signature indicates a clustered, multi-modal sample — a regime `Q_n`'s 50%
breakdown point is designed for — rather than histogram quantization, which §1 rules out.

---

## 3. Validation addendum (2026-07-20)

The conclusions above were re-checked against two fresh runs of `q_resolution_analysis.rs` on the
same machine class. Key observations:

- **§1 reproduces stably.** `rc_q_ns` at sigfig=4 vs 5 was *identical* in most tiers in both runs;
  the largest deviations were 5.9% and 11.1% (both in the 10µs tier), with every ≥100µs tier
  within ~0.2–5%. Sigfig=4 tie fractions peaked at 0.020/0.028; sigfig=3 tie fractions reached
  0.23–0.45 in the low-latency tiers, with distortions of +100% (10ns, 1µs) and −24%
  (100µs unbatched, tie fraction 0.26 — above the target rank). `DEFAULT_SIGFIG = 4` stands.
- **§2's per-tier detail is run-dependent.** Outlier counts varied widely (e.g. the 100µs
  unbatched tier: 3, then 195, then 150 across the three recorded runs). Zero-Tukey-outlier tiers
  showed `Q_n/stdev_r(raw)` of 0.671 (10ms/k=10, run 1) and 0.185 (100µs/k=10, run 2) alongside
  values near 1 — refuting an earlier draft's claim that zero-outlier tiers always track raw
  `stdev_r`. In every low-ratio zero-outlier case, `IQR/1.349 > stdev_r` and `Q_n ≪ IQR/1.349`
  (multi-modality signature, §2.4).
- **Winsorized/IQR closure is the norm but not universal.** Run 2 closed the 100ns/1µs/10µs tiers
  to 0.80–1.20 against `IQR/1.349`, while its 100µs-unbatched tier (150 outliers, n=1000) closed
  only to 0.097; run 1's 1ms-unbatched tier closed only to 0.088. Non-closure was not confined to
  sub-10µs tiers.
- **Readout rounding checked and corrected.** The validation also confirmed hdrhistogram's
  `value_at_quantile` returns the top bucket edge (`highest_equivalent`) rather than the bucket
  midpoint — a ≤ ~0.006% upward bias at sigfig=4 (zero in the unit-resolution range); see §1.4.
  `rousseeuw_croux_q_general` now reads out `median_equivalent` instead. The tables in this
  document predate that correction; its effect is far below run-to-run noise.

Net verdict: §1's resolution conclusion and §2's bottom line (the gap is contamination-driven,
`Q_n` is behaving as a 50%-breakdown estimator should) are confirmed; the *stable* cross-run
diagnostic is the `Q_n` vs `IQR/1.349` comparison, not the Tukey outlier count of any single run.

---

## See also

- `Fable-edited-Optimum_estimators_of_mean_and_median.md` §3.3-3.4 — the `Q_n`-from-histogram
  recipe this analysis validates.
- `examples/q_resolution_analysis.rs` — the reproducible source of every number in this document.
- `examples/mean_and_median_estimates.rs` — the example whose `rc_q_ns/stdev_r`,
  `rc_q_ls/stdev_ln_r` columns motivated this investigation.
