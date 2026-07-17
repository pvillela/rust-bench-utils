# Reconciliation Notes — Robust Estimators of Mean Latency

*Date: 2026-07-17.*

This note reconciles the recommendations about **robust estimators of the mean** of a lognormal
latency distribution in two independently developed documents:

- [`Assessment_Contamination_Robustness.md`](Assessment_Contamination_Robustness.md) (hereafter
  **R3**, report #3 of the Assessment chain #1–#3) — tests estimators **P** (pooled mean),
  **D** (WLS slope on the geometric design), **C** (median of batch means), and **H** (trimmed
  mean of batch means) under an explicit draw-level contamination model: each contaminated draw
  inflated ×10, in *point* (diffuse) or *burst* (localized) patterns, at fractions
  $\varepsilon \in \{1\%, 5\%, 20\%\}$.
- [`Fable-edited-Optimum_estimators_of_mean_and_median.md`](Fable-edited-Optimum_estimators_of_mean_and_median.md)
  (hereafter **FABLE**) — fixed batch size $k = k_{\min}$ set by timing overhead, estimators
  $\bar{Y}$ (grand mean), **LS** (log-space mean $\exp(M + \tilde\sigma_Y^2/2)$), trimmed
  mean / Huber-M, and LN-MLE, with a *group-level* contamination model (10% of group means ×10).

**Scope: mean estimation only.** FABLE's median-of-$X$ recommendations (§5) are out of scope, as
is R3's clean-data winner map inherited from report #2 except where it touches the mean
recommendations. Edits made to both documents as part of this reconciliation are listed in §5;
pre-edit copies are preserved as
[`Assessment_Contamination_Robustness-original.md`](Assessment_Contamination_Robustness-original.md)
and
[`Fable-edited-Optimum_estimators_of_mean_and_median-original.md`](Fable-edited-Optimum_estimators_of_mean_and_median-original.md).

---

## 1. Notation and estimator crosswalk

The documents study the same objects under different symbols:

| Concept | R3 (and reports #1–#2) | FABLE |
|---|---|---|
| Batch (group) size | $b$ (square design: $b = 2^k$) | $k$ (fixed at $k_{\min}$) |
| Number of batches (groups) | $g$ | $g$ |
| Batch (group) means | $\bar Y_1, \dots, \bar Y_g$ | $Y_1, \dots, Y_g$ |
| Clean population mean | $M = e^{\mu + \sigma^2/2}$ | $\theta_\mu = \mathrm{E}[X] = e^{\mu+\sigma^2/2}$ |
| Pooled/grand mean | **P** | $\bar{Y}$ (§4.4) — identical (mean of batch means = pooled mean) |
| Median of batch means | **C** | $\tilde{Y} = \mathrm{median}(Y_i)$ — not offered as a mean estimator (see §3, C2) |
| Trimmed mean of batch means | **H** (symmetric $\lfloor g/8\rfloor$ per side) | TM($\alpha_L, \alpha_R$) (§4.2), same family |
| WLS slope, geometric design | **D** | *(no counterpart — FABLE has no multi-size design)* |
| Log-space mean $\exp(M + \tilde\sigma_Y^2/2)$ | *(no counterpart — never tested in R3)* | **LS** (§4.1), the robust primary |

Caution on the letter $k$: in R3 it indexes the design ($g = b = 2^k$, budget $N = 4^k$); in
FABLE it *is* the batch size. This note always says "batch size" explicitly.

The contamination models also differ in kind, and this difference drives most of §3:

| | R3 | FABLE (Appendix B) |
|---|---|---|
| Unit contaminated | individual draws (×10) | whole group means (×10) |
| Patterns | point (diffuse) and burst (consecutive draws) | equivalent to R3's *burst/concentrated* regime: entire groups corrupted |
| Fractions | 1%, 5%, 20% of draws | 10% of groups |

---

## 2. Points of agreement (no changes needed)

1. **Clean-vs-contaminated is the primary decision axis for the mean.** R3 rec 1 ("decide based
   on the environment, not the estimator table") and FABLE §4.6 ("the clean/contaminated split is
   the main driver for the mean") say the same thing, quantitatively backed on both sides: a
   linear estimator is biased by $\approx \varepsilon(\lambda-1)$ — $+9\%$ at 1% contamination
   (R3 §2), $+90\%$ at 10% (FABLE Table B.1).
2. **Linear estimators are disqualified as primaries under contamination but retained as
   diagnostics.** R3 recs 4 and 5(a) (estimator-disagreement gap $\hat M_P - \hat M_H$ as the
   primary alarm) correspond exactly to FABLE §4.4 (report $\bar{Y}$ alongside LS;
   $\bar{Y} \gg$ LS flags right-tail contamination).
3. **Size the trim to the threat, not to clean-model RMSE.** R3 rec 2 (H survives bursts up to
   its trim fraction $t/g$, then fails hard; set the trim from worst-case interference duty
   cycle) matches FABLE's breakdown accounting (Table B.1 note 6: a 10%-breakdown location is
   "exactly consumed" by 10% contamination).
4. **Prefer small batch sizes.** Convergent conclusion from different premises: R3 rec 3 derives
   it from diffuse-artifact rejection ($\varepsilon b \lesssim 1$ required); FABLE §6.2 derives
   $k = k_{\min}$ from overhead calibration plus median-estimation cost. Each document's argument
   independently reinforces the other's rule (see C3).

---

## 3. Conflicts resolved

### C1 — "H is the default robust estimator" (R3) vs. "never use a trimmed mean when $Y$ is skewed" (FABLE)

- **Conflict.** R3's bottom line makes H (trim ≥ plausible burst fraction) *the* default latency
  mean estimator. FABLE §4.2/§4.6 forbids trimmed means (and Huber-M) as mean estimators outside
  the near-normal band $\hat{S} \leq 0.3$, because a symmetric-centre estimator is biased low for
  $\mathrm{E}[X]$ when $Y$ is right-skewed (to $-55\%$ at batch size 1, $\sigma = 1.5$).
- **Resolution — not a real contradiction; the studies test disjoint robust-estimator sets and
  agree numerically where they overlap.** H's skew bias is fully visible in the Assessment chain
  itself: report #2 measures it at $-5\%$ ($\sigma = 1$, $g = b = 8$) to $-30\%$ ($\sigma = 2$,
  $g = b = 8$) — the same phenomenon, same sign, same order as FABLE's TM rows at comparable
  $Y$-skew. R3 accepts that bias as the price of burst rejection *because its candidate set
  contains no skew-correcting robust estimator*; FABLE has one (LS) and therefore restricts
  TM/H to the band where its bias is negligible.
- **Reconciled rule** (now reflected in both documents), keyed to FABLE's $\hat{S}$ bands
  (§1.5), for contamination that is *concentrated relative to groups* (see C3):
  - $\hat{S} \leq 0.3$ ($Y$ near-normal — the typical regime when batching forces a large
    $k_{\min}$): **H / TM with trim ≥ plausible burst fraction** is primary. Both documents
    already endorse this (R3 rec 2; FABLE §4.6 near-normal contaminated row).
  - $\hat{S} > 0.3$ ($Y$ skewed): **LS** is primary — it corrects the skew that H cannot
    (see C2). H remains usable if its skew bias (FABLE §4.2 table) is acceptable and the trim
    covers the threat.
  - Adversarial (up to ~half the groups corruptible): **C** — maximal group breakdown, accepting
    its skew bias; equivalently view LS as C with that bias removed (C2).

### C2 — LS is a skew-corrected C; it plausibly dissolves R3's C-vs-H dial (open gap)

Since $\exp(M) = \exp(\mathrm{median}(\ln Y_i)) = \mathrm{median}(Y_i)$ (the exponential is
monotone; exact for odd $g$, and for even $g$ equal up to the midpoint convention — $\exp(M)$ is
the geometric rather than arithmetic midpoint of the two central values), FABLE's log-space mean
is

$$\hat\theta_\mu^{\mathrm{LS}} = \mathrm{median}(Y_i)\cdot e^{\tilde\sigma_Y^2/2}
\;=\; \text{(R3's C)} \times e^{\tilde\sigma_Y^2/2},$$

i.e. **C with a multiplicative skew correction** built from the 50%-breakdown scale $Q_n(\ln Y)$.
Analytically it therefore has C's group-level breakdown (both $M$ and $Q_n$ tolerate up to ~half
the groups corrupted) *without* C's downward mean–median bias — the very bias that motivated H in
report #2 and that keeps the C-vs-H trim dial alive in R3 rec 2.

**Prediction:** under R3's burst model LS should track C's near-clean RMSE at every
$\varepsilon \le 50\%$ of batches while removing C's skew bias, dominating both C and H outside
the near-normal band. **This is untested** — R3 never simulated LS, and FABLE's Appendix B never
simulated draw-level point/burst patterns. Recorded as the open gap in §6; both documents now
cross-reference it rather than resolving the C-vs-H choice as if LS did not exist.

### C3 — FABLE's "best available under contamination" claims are burst-scoped

- **Conflict.** FABLE §4.1/§4.6/§7.1 present LS as the robust primary "under contamination"
  without qualification. R3 §3–§4 demonstrates that *diffuse* (point) contamination at
  $\varepsilon b \gtrsim 1$ inflates **every** batch mean by $\approx \varepsilon(\lambda-1)M$,
  so any statistic of the batch means — median, trimmed mean, and equally $M$ and $Q_n(\ln Y)$,
  hence LS — converges to the *contaminated* mean. Group-level robustness is only meaningful for
  contamination that is concentrated relative to the group structure.
- **Resolution.** FABLE's contamination model (whole group means ×10) is precisely R3's
  concentrated/burst regime, so its conclusions are correct *in that regime* and its claims now
  carry the scope condition inherited from R3: group-robust protection requires
  $\varepsilon k \lesssim 1$, working rule $k \lesssim 1/(5\varepsilon)$ (R3 rec 3). If
  $k_{\min}$ exceeds $1/(5\varepsilon)$ for a plausible diffuse fraction, no estimator choice
  helps materially — that is a measurement-hygiene (or explicit mixture-modeling) problem.
- **Bonus alignment.** This gives FABLE §6.2's "use $k = k_{\min}$, never raise $k$" rule a
  second, independent justification: raising the batch size not only costs groups, it erodes
  diffuse-artifact rejection. Conversely, FABLE's LS removes the *other* pressure R3 identified
  toward large batches (skew-bias reduction for C/H), so the two documents jointly resolve R3
  §4.3's "the design pressures conflict" in favour of small batches + skew-correcting estimator.

### C4 — D (guarded regression estimator) vs. fixed-$k$ $\bar{Y}$

- **Difference (not a contradiction).** R3 retains D (with an interference guard) for
  verifiably clean environments at large budgets; FABLE never considers multi-size designs
  because its framework fixes one batch size $k_{\min}$ and handles timing overhead by
  calibration rather than by a regression intercept.
- **Resolution.** These are two overhead-handling strategies, not competing statistics. Under
  contamination both frameworks converge on the same playbook: group-robust estimator on the
  group means, linear estimator demoted to a diagnostic. On verified-clean data,
  R3 rec 4 (compute H alongside D, alarm on disagreement) and FABLE §4.6/§6.3 (report LS
  alongside $\bar{Y}$, alarm on disagreement) are the same guard pattern. No edits needed
  beyond cross-references.

### C5 — Diagnostics: complementary, with one incompatibility to flag

FABLE's diagnostics are a subset of R3's rec 5 suite:

| R3 rec 5 | FABLE counterpart |
|---|---|
| (a) estimator-disagreement gap | $\bar{Y}$-vs-LS comparison (§4.4, §6.3) — same alarm |
| (b) batch-mean outlier count | $Q_3 + 3\,\mathrm{IQR}$ flag (§6.5) + $\hat{S}$ bands (§1.5) — partial |
| (c) longest-run statistic | *(absent)* |
| (d) aggregation-scale variance ratio | *(absent)* |
| (e) cross-scale location disagreement | *(absent — needs small-batch probes)* |
| (f) design-consistency check for D | n/a (no regression design) |

The burst detectors (c) and (d) need the group means **in temporal order**, and the diffuse
detector (e) needs a second, smaller batch size. FABLE's §3.4 HDR-histogram pipeline stores
group means in a histogram, which **destroys temporal order**; retaining the plain sequence of
$g$ group means alongside the histogram costs $O(g)$ memory and restores (c)/(d). FABLE now
notes this (§3.4) and points to R3 rec 5 from its decision flow (§6.3).

In the reverse direction, R3 rec 5 now closes with a **fallback ladder for histogram-only
pipelines** (added 2026-07-17 as part of this reconciliation): (a), (b), and (e) survive without
ordering — they are histogram quantiles/counts plus a second batch size — while (c)/(d) are lost
and can be substituted by streaming accumulators computed at collection time (lag-1
autocovariance for (d); a running longest-run counter or CUSUM for (c)) or by per-epoch
histograms; failing those, the trim is sized conservatively to the flagged fraction $n_+/g$ (or
C is used) and excision is forgone. The robust estimators themselves never needed temporal
order — only the burst *diagnostics* and excision do — and the diffuse limit
$\varepsilon k \gtrsim 1$ is unaffected either way.

---

## 4. Reconciled decision rule for the mean

Combining both documents (batch size $k$, $g$ groups, group means $Y_i$, $\hat{S}$ = robust
log-scale spread per FABLE §1.5):

1. **Environment verifiably clean** (isolated hardware, no GC in measured path):
   $\bar{Y}$ at $k = k_{\min}$ — unbiased, fully efficient (FABLE §4.4). If a multi-size design
   is in use (overhead absorbed by intercept instead of calibration), D is the equivalent
   choice at large budgets (R3/report #2), **with** the disagreement guard. Either way report a
   robust companion (LS or H) and alarm on divergence.
2. **Contamination plausible, concentrated/bursty** (interference events spanning whole groups;
   plausible corrupted-group fraction $\varepsilon_g$ bounded):
   - $\hat{S} \leq 0.3$: **H / trimmed mean with trim $\geq \varepsilon_g$** per side (or
     right-only).
   - $\hat{S} > 0.3$: **LS** $= \exp(M + \tilde\sigma_Y^2/2)$ (skew-corrected, ~50% group
     breakdown). H acceptable if its §4.2 skew bias is tolerable and the trim covers
     $\varepsilon_g$.
   - $\varepsilon_g$ potentially approaching 1/2 (adversarial): **C** $= \mathrm{median}(Y_i)$,
     accepting its skew bias — or LS, whose burst behaviour is predicted to match C's (§3 C2,
     unverified).
3. **Diffuse per-draw contamination plausible** (fraction $\varepsilon$): group-level
   robustness requires $\varepsilon\, k \lesssim 1$ ($k \lesssim 1/(5\varepsilon)$ as a working
   rule). If $k_{\min}$ violates this, no estimator materially helps — fix the pipeline or model
   the mixture explicitly (R3 rec 3). Detect via small-batch probe blocks (R3 rec 5(e)).
4. **Always:** keep $k = k_{\min}$; retain the $g$ group means *in temporal order* (not only a
   histogram); run the disagreement gap (a), outlier count (b), longest-run (c), and
   variance-ratio (d) diagnostics — they distinguish clean / bursty / diffuse at $O(g)$ cost and
   determine which row above applies.

---

## 5. Edits made to the two documents

**To `Fable-edited-Optimum_estimators_of_mean_and_median.md`** (mean sections only; median
sections untouched):

1. §4.1 (Robustness paragraph): added the concentrated-contamination scope caveat and the
   $\varepsilon k \lesssim 1$ condition for diffuse contamination, citing R3.
2. §4.6 (table and defaults): contaminated rows now carry the "concentrated relative to groups"
   proviso; the near-normal contaminated default now says to size the TM/Huber trim to the
   plausible corrupted-group fraction (R3 rec 2).
3. §6.2 (Choosing $k$): added diffuse-artifact rejection as a second argument for $k = k_{\min}$,
   with the $k \lesssim 1/(5\varepsilon)$ working rule and the escalation path when $k_{\min}$
   exceeds it.
4. §6.3 (decision flow): added a note under the contaminated branch pointing to R3's rec 5
   diagnostics for classifying clean/bursty/diffuse, and to the diffuse limitation.
5. §3.4 (HDR-histogram note): added the retain-temporal-order paragraph (C5), with a pointer to
   R3 rec 5's order-free fallback ladder for pipelines that cannot keep the sequence.
6. References: added R3 (and this note) as cross-references.

**To `Assessment_Contamination_Robustness.md`:**

1. §5 rec 2: added a pointer noting the untested log-space mean
   $\mathrm{median}(\bar Y_j)\, e^{\tilde\sigma_Y^2/2}$ (FABLE §4.1) as a candidate that
   analytically combines C's burst breakdown with corrected skew bias, potentially replacing the
   C-vs-H trim dial when batch means are skewed; explicitly marked untested in this study's
   model, with a reference to FABLE and this note (§6 below).
2. §5 rec 5: added a closing fallback paragraph for pipelines that do **not** retain the batch
   means in temporal order (e.g. histogram-only storage): which diagnostics survive order-free
   ((a), (b), (e)), which are lost ((c), (d), excision), the substitution ladder (retain the
   sequence; streaming lag-1 autocovariance / run counter at collection time; epoch histograms),
   and the conservative estimator fallback when none is available (see C5).
3. No simulation table, result, or other recommendation text was altered — R3 remains the
   record of what was actually tested.

---

## 6. Open gap (deliberate; not resolved analytically)

**LS under R3's draw-level contamination model.** The prediction in C2 — LS matches C's
burst robustness at every burst fraction up to ~50% of batches while removing C's and H's skew
bias — is analytic, not empirical. A follow-up simulation should add to R3's grid
($\sigma \in \{0.25, 1\}$, $k \in \{3,\dots,6\}$, point + burst, $\varepsilon \in \{1,5,20\%\}$):

- **LS** $= \mathrm{median}(\bar Y_j)\,e^{\tilde\sigma_Y^2/2}$ with $\tilde\sigma_Y = Q_n(\ln \bar Y_j)$
  (finite-sample $d_g$ from FABLE Appendix C);
- optionally a right-trimmed-location variant, and MAD-vs-$Q_n$ scale sensitivity;
- the same clean cells, to verify LS's clean-model RMSE against H's (the cost side of the
  trade).

Two specific failure modes to watch: (i) under 20% bursts the corrupted batch means enter the
*right* tail, which shifts $M$ upward by the same $\approx 0.32\,\sigma_b$ as C **and** inflates
$Q_n(\ln \bar Y)$, so LS's error may exceed C's by the correction factor's response — the
simulation should report the two factors separately; (ii) at $\hat{S} \leq 0.3$ the correction
$e^{\tilde\sigma_Y^2/2} \approx 1$ and LS ≈ C, so H's efficiency advantage in that band (report
#2) is expected to survive — LS is a candidate to replace the *C end* of the dial, not H's
near-normal home turf.

---

## 7. Deliberately unchanged

- **All simulation tables in both documents** (R3 §3; FABLE Appendix B) and every number derived
  from them.
- **R3's clean-data winner map** ("H at moderate budgets, D at large budgets", inherited from
  report #2) — it concerns a design-freedom setting FABLE does not address; C4 records why this
  is not a conflict.
- **FABLE's median-of-$X$ sections** (§5, §6.4, Table B.2) — out of scope.
- **R3's estimator-disagreement diagnostic suite** (rec 5) — imported by reference into FABLE,
  not duplicated there.
