# Changes in `Optimum_median_estimator.md` (supersession notes)

`analysis/Optimum_median_estimator.md` supersedes both prior documents
(`Omp-DSPro-Opus-reviewed_Optimum_median_estimator-2.md` = "Doc A" and
`ClaudeOpus48_Optimum_median_estimator-2.md` = "Doc B"). This note records what the critical review
changed relative to them.

## Corrections to the prior docs

- **Estimator 4's bias is one-sided, not a zero-crossing.** Both prior docs claimed estimator 4's
  bias "crosses through zero at a sweet spot near `k∼√n`." In fact
  `plim θ̂₄ = e^{−σ²/2}·Q₀.₅(Y)` rises monotonically from `e^{μ−σ²/2}` to `e^μ`, always `≤ e^μ` — it
  approaches zero bias *from below* and never overshoots.
- **Doc A error fixed:** estimator 4 is *not* "the only estimator approximately unbiased" at large
  `k` — estimator 3 is too; estimator 4's edge there is robustness, not uniqueness.
- **Bias vs. variance drivers separated:** bias of estimators 1/2/4 depends on `k` and `σ²` only
  (via near-normality of `Y`), while variance depends on `g = n/k`. So "`k∼√n`" is a sample-size
  balance point, not a bias threshold — a distinction both docs blurred.

## Best of each, kept

- **From Doc B:** estimator 3's unique unbiasedness-for-all-`k` (under lognormality); the identity
  `θ̂₄ = e^{−σ̂²/2}·median(Y)`; the robust-scale program; and the IQR-dominated-by-MAD debunk.
- **From Doc A:** estimator 1's variance *saturation* result (batching's cost to it is mostly bias,
  not variance) and estimator 2's `k`-independent variance scaling.

## Reframed for the `bench_utils` use case

- **Two-track recommendation:** Track A (no batching, higher-latency targets) → model-free sample
  median of `X`; Track B (batching forced, very-low-latency targets) → regime-dependent robust
  estimator.
- **Estimator 3 demoted to "simplest baseline, least robust"** (non-robust location *and* scale,
  maximal model reliance), directly addressing the concern about its fragility. The Track-B default
  is a regime switch — `Q_n`-scale MoM at small `k`, robustified estimator 4 (`Q_n` scale + median
  location) at moderate/large `k`, where forced low-latency batching lands — with a data-driven
  `ĈV_Y` switch rule.
- **Non-log-normality guard:** the mean/median correction is model-specific and its bias is
  irreducible for `k > 1`, so keep `k` minimal, run an unbatched pilot to check
  `E[X]/median(X) ≈ e^{σ²/2}`, and report the model-free `median(Y)` as a sanity value.
- **Additional-estimators section is comprehensive but robustness-filtered:** keeps the robust
  candidates and explicitly excludes the Fenton–Wilkinson corrected log-mean, bias-corrected MLE,
  and shrinkage blend (all mean/log-mean based, breakdown 0), each with a one-line reason.

## Structure

- The required summary table has the exact rows (estimators) × columns
  (`k=1`, `1<k≪√n`, `k∼√n`, `√n≪k<n`, `k=n`) structure, plus an efficiency-scaling table.
- The two prior docs are left in place as superseded.
- Short "Doc A"/"Doc B" attributions are retained where a specific prior result is adopted or
  corrected, as provenance; both labels are defined in the opening note of the main document.
