# Reconciliation Notes — `Optimum_estimators_of_mean_and_median.md`

*Date: 2026-07-07.*

This note records the reconciliation of the recommendations in the combined report
[`Optimum_estimators_of_mean_and_median.md`](Optimum_estimators_of_mean_and_median.md) (hereafter
**COMBINED**) against the two focused analyses
[`Opus-edited-Optimum_mean_estimator.md`](Opus-edited-Optimum_mean_estimator.md) (**MEAN**) and
[`Opus-edited-Optimum_median_estimator.md`](Opus-edited-Optimum_median_estimator.md) (**MEDIAN**).
It lists the edits made to COMBINED and the points where the three documents agree, differ, or use
different notation for the same object.

---

## 1. Notation crosswalk

The documents describe the same estimators with different symbols. The correspondence is:

| Concept | COMBINED | MEAN / MEDIAN |
|---|---|---|
| Grand mean of group means | $\bar{Y}$ | $m_y$ |
| Mean of $\ln Y_j$ | $\bar{Z}$ | $\text{ml}_y$ |
| Median of $\ln Y_j$ | $M$ | *(not named)* |
| Sample median of $Y_j$ | $\tilde{Y}$ | $\text{median}(Y_j)$ |
| Log-scale variance of $X$ (target) | $\sigma^2$; est. $\hat\sigma_X^2$ | $\sigma^2$; est. $\hat\sigma^2$ |
| Log-variance of $Y_j$ (FW) | $\sigma_Y^2$; est. $\tilde{S}_Z^2$ | $\sigma_w^2$; est. $\hat\sigma_w^2$ |
| Robust scale of natural-scale $Y_j$ | $\tilde\sigma_Y = Q_n(Y)/d_g$ | $\tilde\sigma_y$ |
| FW relation | $\sigma_Y^2 = \ln(1+(e^{\sigma^2}-1)/k)$ | same |

**One difference in the scale route.** COMBINED estimates the log-variance of $Y$ **directly** on
the log scale, $\tilde{S}_Z^2 = (Q_n(\ln Y)/d_g)^2$. MEAN/MEDIAN estimate it **indirectly**:
robust natural-scale variance $\tilde\sigma_y^2$ → FW map $\hat\sigma^2 = \ln(1+k\tilde\sigma_y^2/m_y^2)$
→ $\hat\sigma_w^2 = \ln(1+(e^{\hat\sigma^2}-1)/k)$. The two routes are asymptotically equivalent
(both $\to \sigma_Y^2$) but differ in finite-sample behaviour; neither document is wrong. No change
was made — the routes are noted here so a reader can map between them.

---

## 2. Estimator correspondence

| COMBINED estimator | MEAN / MEDIAN counterpart | Relationship |
|---|---|---|
| Log-space mean $\exp(M + \tilde{S}_Z^2/2)$ (§4.1) | MEAN estimator 2 $\exp(\text{ml}_y + \hat\sigma_w^2/2)$; robustified **B** | Same family (log-location + FW/scale correction → $\mathrm{E}[X]$). COMBINED uses **median** log-location; MEAN's B uses a **trimmed** log-mean (more efficient, still robust). |
| Log-space median, Eq. (3) (§5.2) | MEDIAN estimator **A** $e^{-\tilde\sigma^2/2}\,\text{median}(Y)$ | **Not identical.** Eq. (3) $= \text{median}(Y)\cdot\exp((\sigma_Y^2-\sigma^2)/2)$; estimator A omits the $+\sigma_Y^2/2$ term. Eq. (3) is consistent for $e^\mu$ at **every** $k$; A is consistent only as $k\to\infty$ and is biased low for small $k$. See §4 below. |
| RMoM, Eq. (4) (§5.3) | MEDIAN estimator **B** (mean-location MoM) $m_y e^{-\tilde\sigma^2/2}$ | Same family (robust mean-location + robust-scale moment inversion). |
| Classical BT (§5.5) | MEDIAN estimator 3 $m_y e^{-\hat\sigma^2/2}$ | Same; both flag it non-robust and clean-data-only. |
| Geometric mean $\exp(\bar{Z})$ (§5.4) | MEDIAN estimator 1 | Same; both: MLE at $k=1$, badly biased for $k>1$. |

---

## 3. Points of agreement (no change needed)

- **Median, $k=1$:** sample median is the robust, model-free default; geometric mean / MLE is the
  efficient option when log-normality is confirmed and data are clean.
- **Median, $k>1$, contaminated:** a robust median-location back-transform is the primary; the
  non-robust classical BT is clean-data-only.
- **Both problems:** $\bar{Y}$ / $m_y$ and $s_Y^2$ have breakdown 0; robust scale ($Q_n$ preferred
  over MAD) is the correct default; the $k=n$ ($g=1$) case is degenerate and reduces to $Y_1$.
- **FW machinery, delta-method drift, and the mean/median ratio $e^{\sigma^2/2}$** are stated
  consistently across all three.

---

## 4. Differences resolved, and the edits made to COMBINED

### 4.1 Mean at $k=1$: "never use $\bar{Y}$" was too absolute (edited)

- **Conflict.** COMBINED §4.4 said $\bar{Y}$ should **never** be a primary estimator; MEAN §7
  Track A makes the sample mean $\bar{X}$ the **default** at $k=1$, arguing its efficiency loss
  vs. the MLE is $\leq 1\%$ for typical latency $\sigma \in [0.1, 0.5]$.
- **Resolution.** Both are right in scope: $\bar{Y}$ is fragile under contamination (COMBINED's
  thesis) but unbiased and near-fully-efficient on clean data (MEAN's point). Edited COMBINED §4.4,
  §4.6 (table + default), §6.3 (added step 7), and the §7.1 table footnote to make the
  recommendation **conditional on whether contamination is expected**: robust log-space mean as the
  default when it is (the latency norm), $\bar{Y} = \bar{X}$ as an efficient primary when it is not.

### 4.2 Mean at $k=1$: LN-MLE was rejected outright (edited)

- **Conflict.** COMBINED §4.5 titled the LN-MLE "Not Recommended" for **any** application; MEAN §7
  endorses the MLE $\exp(\overline{\ln X}+s_{\ln X}^2/2)$ as the **most efficient** consistent
  estimator when log-normality is trusted.
- **Resolution.** The rejection reflects *fragility* (a single outlier enters $s_Z^2$
  exponentially), not *inefficiency*. Retitled §4.5 and rewrote it to keep the strong
  contamination warning while carving out the clean, trusted-lognormal $k=1$ niche where the MLE is
  the efficient choice. Noted that the robust primary (§4.1) is exactly this estimator with a
  median log-location and robust scale.

### 4.3 Mean, small $n$: UMVUE was missing (added)

- **Gap.** MEAN §5.E recommends the Finney / Shimizu–Iwase **UMVUE** for $n < 30$, where the MLE's
  $O(1/n)$ upward bias is non-negligible. COMBINED did not mention it.
- **Resolution.** Added the UMVUE to §4.5, §4.6, and step 7 of the §6.3 flow, scoped to the same
  clean, trusted-lognormal $k=1$ niche as the MLE.

### 4.4 Median, $k>1$: location choice by sub-regime (clarified)

- **Difference (not a contradiction).** COMBINED offers a single log-space median (Eq. 3) across
  all $k>1$ with RMoM as cross-check. MEDIAN §7 splits Track B: **mean-location** MoM (estimator B)
  at small $k$, **median-location** (estimator A) at moderate-to-large $k$.
- **Resolution.** Added a "Location choice by sub-regime" note to COMBINED §5.6 explaining the
  trade-off: at small $k$, $\text{median}(\ln Y)$ is skew-biased for $\mu_Y$, so a robust
  mean-location RMoM (using $\mathrm{E}[Y]=\mathrm{E}[X]$ exactly) can carry less bias at the cost
  of lower breakdown; at large $k$, median-location is both robust and $\approx$ unbiased.
- **Important clarification retained.** COMBINED's Eq. (3) folds in the FW $\tilde{S}_Z^2$ term and
  is therefore consistent at **every** $k$ — it is *not* the naive corrected median
  $e^{-\hat\sigma_X^2/2}\,\text{median}(Y)$ (= MEDIAN's estimator A), which omits that term and is
  biased low at small $k$. The note states this explicitly so the two are not conflated.

### 4.5 Natural-scale robust scale for HDR-histogram pipelines (added COMBINED §3.4)

- **Driver.** Latency pipelines commonly store observations in an HDR histogram (integer bins,
  constant relative precision). $Q_n$ is naturally computed from such a histogram by materialising
  the pairwise-absolute-difference distribution in a *second* histogram and reading off its lower
  quartile. This is integer-friendly on the **natural scale** (differences share the range of the
  values) but requires a fixed-point scale factor on the **log scale** (log-differences are small
  fractions), which costs precision and tuning.
- **Addition.** Added COMBINED §3.4 ("Computing the Robust Scale from an HDR Histogram"),
  documenting the natural-scale-first route as a first-class alternative to the log-scale formulas
  of §4.1/§5.2. It specifies the histogram-of-differences construction (weighting by count products
  $c_i c_j$ and $\binom{c_i}{2}$), and states three rules:
    1. **Commit to one lane.** A natural-scale $Q_n(Y)$ feeds the **exact-moment** estimators
       (RMoM §5.3 for the median; a robust location of $Y$ for the mean, since $\mathrm{E}[Y] =
       \mathrm{E}[X]$) — *not* a log-space formula. Mixing (natural-scale scale → log-space
       estimator) combines the FW approximation with the skewed-scale bias and is the worst option.
    2. **Regime switch.** $Q_n$'s constant is normal-calibrated, so the natural-scale route is
       well-calibrated only when $Y$ is near-normal. Detect via the natural-scale
       $\widetilde{\mathrm{CV}}_Y = (Q_n(Y)/d_g)/\hat\mu_Y$: $\lesssim 0.2$ ⇒ proceed; larger ⇒
       $Q_n(Y)/d_g$ underestimates $\mathrm{SD}(Y)$ (robust scale drops the tail the variance
       identity needs), so correct $d_g$ for skew or fall back to the log scale.
    3. **Resolution.** At large $k$ the $Y_j$ cluster tightly; keep histogram unit resolution fine
       relative to $\mathrm{SD}(Y)$.
- **Why this is safe.** Batching is used only for very-low-latency targets, where $k_{\min}$ is
  large and $Y$ is already near-normal — exactly where the natural- and log-scale routes coincide.
  So the convenient route and the well-calibrated route are the same one in the regime that forces
  batching. Cross-referenced from §5.3 (RMoM).

---

## 5. Deliberately unchanged

- **COMBINED's simulation tables (B.1, B.2) and the numbers derived from them** were not altered;
  the edits above are all recommendation/framing changes consistent with those tables (e.g.
  $\bar{Y}$ at $+90\%$ under contamination; log-space median as robust-best for $k>1$).
- **COMBINED's choice of median (rather than trimmed-mean) log-location** for its primary robust
  mean estimator was kept. MEAN's trimmed-log-mean variant (estimator B) is a valid, more-efficient
  alternative and is now cross-referenced (§4.5), but COMBINED's 50%-breakdown choice remains a
  defensible default and is backed by its own simulations.
- **The scale-estimate "route" was kept as COMBINED had it** (the difference is described in §1).
  All corrections need the log-scale spread of the batch means, $\sigma_Y = \mathrm{SD}(\ln Y)$, and
  the two document families compute it by applying the same two operations — *take the log* and
  *measure robust spread* — in the opposite order:
    - COMBINED (**log-scale route**): log first, then spread — $\tilde{S}_Z = Q_n(\ln Y)/d_g$,
      which targets $\sigma_Y$ directly.
    - MEAN/MEDIAN (**natural-scale route**): spread first, then transform — $Q_n(Y)$ on the raw
      group means, converted to the log scale via the FW moment map.

  Because the log and the robust-spread estimator do not commute, the two routes agree only
  asymptotically ($g\to\infty$) and differ slightly in finite samples; neither is wrong. COMBINED's
  estimator *formulas* remain written on the log-scale route (unchanged), but the **natural-scale
  route is now documented as a first-class implementation alternative** in COMBINED §3.4 — see §4.5
  below.
