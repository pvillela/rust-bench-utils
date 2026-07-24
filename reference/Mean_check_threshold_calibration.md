# Calibrating the `mean_check` verdict thresholds

*Empirical study behind the `MeanVerdict` design and the `MEAN_CHECK_MILD_Z` /
`MEAN_CHECK_SIGNIFICANT_Z` cutoffs (`src/bench_out.rs`).*

- **Harness / script:** `src/test_support/mean_check_calibration.rs` — kept crate-internal because it reuses one `BenchOut` across trials via the private `reset()`.
- **Raw data:** `analysis/Mean_check_threshold_calibration_data.txt` (full run output).
- **Reproduce:**
  ```text
  cargo test -r --lib --features _ALL_NON_TEST,_test -- \
      test_support::mean_check_calibration::synthetic_study --ignored --nocapture
  ```

## Question

`BenchOut::mean_check` compares the naive arithmetic mean to the robust `mean_rob` and classifies the
disagreement. Right-tail contamination inflates the naive mean but not `mean_rob`, so a positive gap
is the signal. What statistic should the verdict key on, and at what cutoffs, so that clean data
rarely false-alarms while real contamination is detected?

## Method

This is a seeded synthetic Monte Carlo (`SplitMix64` uniforms → Box–Muller normals), designed so that
`mean()`, `mean_rob()`, and `mean_check()` run on synthetic data exactly as they would in production.

Each **trial** simulates one benchmark run:

1. Draw per-execution latencies `X ~ LogNormal(μ, σ)` (`μ` fixed; only the per-execution log-scale
   `σ` varies across the grid).
2. Group them into `g` batches of `k` draws each and average each batch, producing `g` **batch
   means** — exactly the observations the crate records when `batch = Some(k)`.
3. Optionally **contaminate** some draws or batches (patterns below); a clean trial skips this.
4. Record the `g` batch means into a real `BenchOut` (production recording unit and sigfig) and read
   back the `rel_gap` and `mean_check` verdict.

A **cell** is one point on the parameter grid `(σ, k, g)`. Within each cell we run two independent
sets of trials: **2000 clean trials** (no contamination — these calibrate the clean false-alarm
rate) and **1000 contaminated trials** *at each contamination level* (these measure detection power).
The grid is `σ ∈ {0.5, 1, 2}` (near-normal → heavy right skew), `k ∈ {1, 64, 1000}` (unbatched →
heavy batching), and `g ∈ {50, 200}` (sample size). Per cell we also report
`σ_Y = rc_q_ls`, the robust log-scale of the *recorded batch means* (distinct from the per-execution
`σ`, since batching shrinks dispersion).

Contamination has one **magnitude** and two spatial **patterns**. The contaminated fraction
`ε ∈ {1, 5, 10, 20%}` sets *how much* is hit; the pattern sets *where*:

- **point** (diffuse) — each individual draw is independently inflated with probability `ε`, so the
  spikes are scattered across all the batches;
- **burst** (concentrated) — a fraction `ε` of *whole batches* have all their draws inflated, so the
  spikes are clumped into a few batches.

**Contamination magnitude.** A contaminated draw (point) or batch (burst) is multiplied by a fixed
**inflation factor `c = 10`** — a *dimensionless* multiplier (`X → 10·X`), **not** a rate: it has no
time unit. `c = 10` is a moderate order-of-magnitude spike, representative of a scheduler-preemption
/ interrupt-scale delay landing on an otherwise-fast execution, and matches the contamination
magnitude used in `reference/analysis/Assessment_Contamination_Robustness.md`. The verdict statistic
keys on a *relative* gap, so the exact value of `c` mostly sets how sharply detection turns on with
`ε`; `c = 10` makes even `ε = 1%` a clearly visible spike without being a pathological outlier.
(The raw-data file's header records this multiplier as `LAMBDA=10`; it is this `c`, **not** the
arrival rate `λ` of Finding 4.)

## Finding 1 — the raw gap needs standardizing; `rel_gap·√g/σ_Y` does it

The clean `rel_gap` is not a constant — it is sampling noise whose scale is `σ_Y/√g`, so a fixed
`rel_gap` cutoff cannot control false alarms across batch/sample sizes. **Standardizing to
`z = rel_gap · √g / σ_Y` collapses the clean tail** across the common `σ ≤ 1` regime (the last two
rows, `σ = 2`, show where it breaks down — see Finding 3):

```
 sigma      k      g     sigmaY   gap_p95   gap_p99     z_p95     z_p99
  0.50      1     50     0.5014    0.1102    0.1565     1.596     2.370
  0.50     64    200     0.0666    0.0065    0.0093     1.368     1.978
  0.50   1000    200     0.0168    0.0015    0.0022     1.305     1.844
  1.00     64    200     0.1584    0.0206    0.0262     1.817     2.387
  1.00   1000    200     0.0413    0.0041    0.0054     1.401     1.850
  1.00      1     50     1.0033    0.3155    0.5524     2.372     4.228
  2.00     64    200     0.4777    0.2022    0.2841     5.986     8.290   ← high-skew: standardization fails
  2.00   1000    200     0.1687    0.0480    0.0637     3.993     4.990   ← small σ_Y yet unreliable
```

For `σ ≤ 1`, clean `z_p95 ∈ [1.3, 2.37]` and `z_p99 ∈ [1.84, 4.23]` — a ~2× spread instead of the
~200× spread of the raw gaps. One z-cutoff therefore controls false alarms across batch and sample
size.

## Finding 2 — choosing the two cutoffs

Two cutoffs, **`mild_z`** and **`sig_z`**, turn the standardized gap `z` into the three-way verdict: `mild_z ≤ z < sig_z ⇒ Mild`, `z ≥ sig_z ⇒ Significant`, otherwise `Insignificant`. We pick them to keep the **clean false-alarm rate** low while still catching real contamination. Three candidate pairs were scored; the columns (each a fraction of trials, so 0.0253 = 2.53%) are:

- **`clean_FPmild`** — clean false-alarm rate for *Mild*: fraction of **clean** trials (pooled over
  all `σ ≤ 1` cells) with `z ≥ mild_z`, i.e. how often clean data wrongly reaches at least Mild.
  *Lower is better.*
- **`clean_FPsig`** — clean false-alarm rate for *Significant*: same, but for `z ≥ sig_z`.
- **`burst5_det_mild`** — burst detection rate at `ε = 5%`: within each `σ ≤ 1` **burst** cell, the
  fraction of **contaminated** trials with `z ≥ mild_z`; the column is the **mean of those per-cell
  rates** (the data file's "mean burst@eps=5%"), i.e. how often real contamination is caught.
  *Higher is better.*

Both contamination patterns were simulated across the full grid (point results are in Finding 3);
only **burst** is used as the *tuning objective* here. That is a choice about what the cutoffs
optimize, not about what was run: burst is the regime `mean_check` is meant to catch, whereas point
contamination — once it is spread across essentially every batch (the *diffuse limit* `ε·k ≳ 1`,
defined in Finding 3) — is provably undetectable by *any* cutoff, so there is nothing for a threshold
to trade off against and including it would only distort the tuning.

```
 mild_z   sig_z  clean_FPmild   clean_FPsig  burst5_det_mild
   2.00    3.00        0.0253        0.0048            0.870
   2.50    4.00        0.0093        0.0014            0.854   ← chosen
   3.00    5.00        0.0048        0.0004            0.839
```

The cutoffs are anchored to the **worst clean cell** in the target regime (`σ ≤ 1`) rather than to
the pooled average, so the guarantee holds cell-by-cell. From the Finding 1 table, that worst cell
has the highest clean tail of any `σ ≤ 1` cell: 95th percentile `z_p95 = 2.37`, 99th percentile
`z_p99 = 4.23`. Setting a cutoff at that worst-case percentile therefore leaves at most that fraction
of clean trials above it *there*, and even fewer in every other `σ ≤ 1` cell (whose tails are lower)
— so the clean false-alarm rate is bounded in **every** cell, not merely on average:

- **`mild_z = 2.5`** sits just above the worst cell's `z_p95` (2.37), so at most ~5% of clean trials
  reach Mild even in the worst cell — and only **0.9%** pooled (`clean_FPmild = 0.0093`);
- **`sig_z = 4.0`** sits just below the worst cell's `z_p99` (4.23), so Significant fires on a bit
  over 1% of clean trials in the worst cell — **0.14%** pooled (`clean_FPsig = 0.0014`).

This costs little detection: the chosen pair still catches **85%** of burst contamination at `ε = 5%`
(`burst5_det_mild = 0.854`), rising to ~100% by `ε = 10%` (Finding 3). Loosening to `2.0/3.0` buys
only +1.6 points of detection (0.870) at ~3× the false-alarm rate; tightening to `3.0/5.0` barely
lowers false alarms while giving up more detection. `2.5 / 4.0` is the knee of that trade-off.

## Finding 3 — detection and the two blind spots

Whether contamination is caught depends sharply on **where** it lands (point vs burst) and on the
batch size `k`. Holding `σ = 0.5`, `g = 200`, `ε = 5%` fixed and varying only `k` and the pattern
makes the split vivid (median `z` and detection rate `det = fraction of contaminated trials with
verdict ≥ Mild`, from the raw data):

```
   k        point (diffuse)        burst (concentrated)
            med_z    det           med_z     det
    1        9.09   0.990           8.91    0.987
   64        0.57   0.009          83.9     1.000
 1000        0.14   0.002         332.6     0.999
```

At `k = 1` (unbatched) the two patterns are the *same experiment* — one execution per "batch", so
inflating a draw is inflating a batch — and both are caught (~0.99). As `k` grows they diverge
completely, which is the whole story of this diagnostic:

- **Concentrated (burst / group) contamination is caught, and batching *helps*:** an inflated whole
  batch is a lone outlier among the batch means, so its `z` explodes into the tens–hundreds as `σ_Y`
  shrinks with `k` (`det → 1.0`). Pooled over the `σ ≤ 1` grid, burst detection at `ε = 5%` is 0.85
  (Finding 2) and reaches ~1.0 by `ε = 10%`. (It is weakest in the small-`g`, unbatched, `σ = 1`
  cells, where even a genuine burst is only a few multiples of sampling noise.)
- **Diffuse (point) contamination becomes invisible once batched — the *diffuse limit*:** the
  product `ε·k` is the *expected number of contaminated draws per batch* (a fraction `ε` of the `k`
  draws in each batch). Once `ε·k ≳ 1` — i.e. on average at least one contaminated draw lands in
  *every* batch — all batch means are inflated together, so `mean_rob` inflates right along with the
  naive mean and the gap collapses. **This condition, `ε·k ≳ 1`, is the "diffuse limit"** referred to
  throughout: the point past which point contamination is smeared uniformly across the recorded
  observations rather than concentrated in a few. For example at `ε = 5%`, `k = 64` we have
  `ε·k = 3.2` (about 3 of every 64 draws inflated in every batch), and detection has already
  collapsed to `det = 0.009`. `mean_check` is a *group*-robustness diagnostic by design: it cannot
  see contamination that has been averaged uniformly into every batch.
- **Extreme per-execution skew (σ ≳ 1.5) breaks the standardization:** here `mean_rob` is *itself*
  biased — the log-space estimator systematically misses the true mean by an amount that, unlike
  sampling noise, does **not** shrink as `g` grows. Standardizing divides the gap by `σ_Y/√g`, i.e.
  multiplies it by `√g`, so this persistent (non-shrinking) bias gets scaled up instead of averaged
  away, and clean `z` lands far above the cutoffs even with *no* contamination. The two `σ = 2` rows
  shown in Finding 1 already have clean `z_p95 ≈ 4–6` and `z_p99 ≈ 5–8` — every `z_p99` above the
  `sig_z = 4.0` cutoff (the `z_p95` values straddle it: 5.99 above, 3.99 just below) —
  and the unbatched `σ = 2, k = 1` cells — not in that curated table but in the raw-data file — reach
  `z_p99 ≈ 14`. These are guaranteed false alarms. The failure is **not cleanly identifiable from
  `σ_Y`** (`σ=2, k=1000` has `σ_Y=0.17` yet `z_p99≈5`; `σ=0.5, k=1` has `σ_Y=0.50` yet `z_p99≈2.4`),
  so no `σ_Y` gate cleanly separates the good regime from this one — hence the documented caveat
  rather than an automatic guard.

**What the point simulations tell us, and how they shape the verdict's meaning.** The point runs are
not used to set the cutoffs, but they establish the *scope* within which a burst-tuned verdict is
meaningful — three concrete conclusions:

1. **The blind spot is real and sharp, not a tuning artifact.** Point detection collapses toward zero
   (`0.009`, `0.002`) precisely when `ε·k ≳ 1`, and no choice of `mild_z`/`sig_z` recovers it — the
   *signal itself* (the gap) is gone, because `mean_rob` is contaminated alongside the naive mean.
   Lowering the cutoffs to chase it would only inflate false alarms without adding detection. This is
   why point is excluded from the tuning rather than traded against it.
2. **The cutoffs are not biased by pattern.** Where point *is* detectable (unbatched, `k = 1`), its
   `z` distribution coincides with burst's (median `z` 9.09 vs 8.91; `det` 0.990 vs 0.987). So burst
   and point are the same detection problem exactly when both are visible; tuning on burst does not
   make the thresholds too loose or too tight for point. Burst is simply the pattern that *stays*
   visible as `k` grows, which is why it, not point, defines the useful operating envelope.
3. **How to read the resulting verdict.** A `Mild`/`Significant` verdict means "a concentrated,
   group-level right-tail excess is present" — that statement holds for burst at any `k` and for
   point only when unbatched. An `Insignificant` verdict is therefore *conditional*: it rules out
   concentrated contamination but says nothing about diffuse contamination once `ε·k ≳ 1`. The
   verdict is a one-sided guarantee — positive results are trustworthy across the tested grid;
   negative results certify only the concentrated regime, and must be paired with a small enough
   timed unit (Finding 4) before they can be read as "the mean is clean".

## Finding 4 — real interference obeys the same diffuse limit; sample size is not the lever

This section models *real* interference (scheduler preemptions / interrupts / frequency
transitions) with a small physical model. Three quantities, all in consistent time units (read them
as seconds and seconds⁻¹; only the dimensionless products below matter, so the choice of unit
cancels):

- **`λ`** — interference **arrival rate**, in *events per second* (`s⁻¹`). This is a Poisson-style
  rate, a *different quantity* from the Method section's inflation factor `c` (a dimensionless
  multiplier).
- **`D`** — mean **duration** of one interference event, in *seconds*.
- **`U`** — wall-clock **duration of one timed unit**, in *seconds*: a batch's total duration when
  batching, or a single execution's latency when not. This is what the crate feeds to one `Instant`
  pair, so it is the window an interference event can land in.

An event lands in a given unit with probability ≈ `λ·U` (dimensionless: rate × window). So real
interference contaminates a timed unit exactly as the synthetic point/burst models do, and its
**concentration** is set by `U`, not by batching:

- fraction of units hit ≈ `λ·U` (a longer window is a bigger target — this plays the role of `ε`);
- relative inflation of a hit unit ≈ `D/U` (a fixed-length stall is a smaller *fraction* of a longer
  unit, so it shrinks as `U` grows);
- their product — the naive-mean bias — ≈ `λ·D`, the interference **duty cycle** (dimensionless,
  rate × duration), roughly *independent of `U`*.

**How this maps onto the synthetic burst model.** The Method section's two knobs are exactly these
first two quantities, one timed unit = one batch:

- contaminated fraction `ε` ↔ hit probability `λ·U`;
- excess inflation `c − 1` (a hit unit becomes `c×`) ↔ `D/U` (a hit unit becomes `(1 + D/U)×`), i.e.
  `c ↔ 1 + D/U`;
- and the biases agree: synthetic `ε·(c − 1)` ↔ Poisson `λU · D/U = λ·D`.

The essential difference is what is held fixed. In the study, `ε` and `c` are set **independently and
kept constant** (`c = 10`, `ε` swept). In reality both are tied to `U`: lengthening the unit
**raises** the hit fraction (`ε ≈ λU ↑`) while **lowering** the per-hit inflation (`c − 1 ≈ D/U ↓`),
the two trading off to keep `λD` fixed. So one physical interference process is not a single `(ε, c)`
point but a **family** of them, swept out as `U` grows — and the study, fixing `c` and varying `ε`,
does **not** trace that physical `U`-curve; its cells are individual points on the `(ε, c)` plane.

What carries over is not a numeric match but the pair of **regimes** the mapping exposes:
**concentrated** (`ε = λU ≪ 1`: few units hit, each a large spike) versus **diffuse**
(`ε = λU ≳ 1`: most units hit, each mildly). These are the same two regimes the synthetic
point/burst results separate in Finding 3, and the detectability discussion below is stated in their
shared terms — it is *not* a reading of the burst table along `U`.

So the mean's contamination is ~duration-independent, but its detectability is not:

- **short units (`λU ≪ 1`):** rare, dramatic spikes → concentrated → `mean_rob` resists → gap →
  **detected** (the 100 µs spot-check row);
- **long units (`λU ≳ 1`):** most units hit, each mildly → diffuse → `mean_rob` inflates with the
  naive mean → **gap collapses → undetected**. This is the same `ε·k ≳ 1` diffuse limit as for point
  contamination; both higher latency and heavier batching raise `U` toward it. An `Insignificant`
  verdict at long units therefore does **not** certify a clean mean — the diagnostic has lost
  resolution. The lever is a *smaller* `U` (smaller batch), not more data.

**Does sample size (`g`) help?** It never moves the regime boundary — `λU` is per-unit, so adding
units at fixed `U` leaves the contaminated *fraction* unchanged — and its value depends on the regime:

- **Concentrated regime → yes.** A fixed concentrated contamination gives a fixed `rel_gap`
  (a population bias), so `z = rel_gap·√g/σ_Y` grows like `√g` while clean `z` stays `O(1)`
  (clean `rel_gap ~ σ_Y/√g`). More samples thus detect *smaller / rarer* concentrated interference
  **without** raising the false-alarm rate — sample size is free detection power (precisely why the
  statistic standardizes by `√g`).
- **Diffuse regime → no.** `mean_rob` is contaminated too, so `rel_gap ≈ 0` regardless of `g`; there
  is no gap to recover however long you run. The obstacle is identifiability, not statistical power —
  reduce `U` instead of collecting more data.

## Spot-check on real `BusyWork` latencies

Running `mean_check` on real measured batch means (harness `busywork_spotcheck`,
`--features …,load`), clean + 5%-burst-injected:

| target | k | g | σ_Y | clean_gap | clean_z | clean verdict | burst_z | burst verdict |
|---:|---:|---:|---:|---:|---:|:---|---:|:---|
| 1µs | 100 | 200 | 0.0265 | 0.0210 | 11.17 | Significant | 189.84 | Significant |
| 1ms | 1 | 200 | 0.0131 | 0.0014 | 1.47 | Insignificant | 474.27 | Significant |

- The **1ms unbatched** run matches the synthetic clean model *quantitatively*: at its small
  `σ_Y ≈ 0.013` the comparable small-`σ_Y` synthetic clean cells (Finding 1) sit at `z_p95 ≈ 1.3–1.4`
  and `z_p99 ≈ 1.84`, and the measured `clean_z = 1.47` lands right between them — clean sampling
  noise, `Insignificant`, exactly as predicted. Injecting a 5% burst into that *same* run moves it
  from `clean_z = 1.47` (Insignificant) to `burst_z = 474` — over 100× the `sig_z = 4.0` cutoff — so
  it flips to `Significant` with an enormous margin. On real measured data the diagnostic thus stays
  quiet when the run is clean and fires hard when contamination is injected.
- The **1µs / batch-100** run flags `Significant` (`clean_gap = 2.1%`, `clean_z = 11`). The batch is
  timed as one ~100 µs wall-clock interval (one `Instant` pair, then ÷100), so an interference event
  (scheduler preemption, interrupt) that lands in that window inflates the whole batch mean. **This
  susceptibility is set by the timed unit's wall-clock duration, not by batching** — a single 100 µs
  execution would be exactly as exposed; batching's only role here is to average out the *per-execution*
  jitter of the 100 tiny executions (hence the small `σ_Y ≈ 0.027`), which is orthogonal to
  whole-window contamination. With `σ_Y` near-normal the log-space model bias is negligible
  (`σ_Y²/2 ≈ 3.5e-4`), so the 2.1% gap is not model bias — it is ~2–3 inflated batch means out of 200
  (concentrated / group-level contamination), exactly the regime `mean_check` targets, so the flag is
  the diagnostic working. Against the synthetic clean model this row is a clear outlier: at
  `σ_Y ≈ 0.027` the synthetic clean cells cap `z_p99 ≈ 1.85`, whereas this real run gives
  `z = 11.2` — roughly 6× the clean ceiling. That gap *is* the point: the excess over the synthetic
  clean prediction is genuine group-level contamination in the real data, not clean sampling noise.
  (The clean **1ms / unbatched** row differs from this one in timed-unit *duration*, 1 ms vs 100 µs —
  not in batching; a fixed ~1 ms event is a ×10 spike on a 100 µs unit but only ×2 on a 1 ms unit, so
  shorter units show sharper right-tail contamination. Single-run spot-check — illustrative, not
  conclusive.)

## Decision (implemented)

- **Statistic:** the verdict keys on `std_gap = rel_gap · √g / σ_Y` (`MeanVerdict::from_std_gap`);
  `MeanCheck` now also exposes `std_gap` and `sigma_y`.
- **Cutoffs:** `MEAN_CHECK_MILD_Z = 2.5`, `MEAN_CHECK_SIGNIFICANT_Z = 4.0`.
- **High-dispersion caveat (documented, not gated):** at extreme per-execution skew the verdict can
  over-flag; this cannot be reliably detected from `σ_Y` alone, so no threshold constant is exposed —
  the caveat is documented on `MeanVerdict` and `MeanCheck::sigma_y` is reported as a partial signal.
  Batching (which lowers `σ_Y` and normalizes the batch means) is the fix.
