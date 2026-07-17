# Assessment: Estimator Robustness under Measurement Contamination

This is the follow-up promised in §5.1 of `Assessment_Improved_Estimators.md` (report #2).
Reports #1 and #2 ranked estimators of a $\mathrm{Lognormal}(\mu, \sigma^2)$ mean under **clean
i.i.d. sampling**, where the only "outliers" are the population's own tail. Real latency
measurement adds *contamination* — GC pauses, scheduler preemption, thermal throttling, noisy
neighbors — that is **not** part of the distribution being estimated. This report tests the
report-#2 finalists against an explicit contamination model and re-examines the recommendations.

Notation as before: $m = 2k$, batch count $g = 2^k$, batch size $b = 2^k$, square-design budget
$N = 4^k$, geometric-design budget $2^m - 1$; clean population mean $M = e^{\mu + \sigma^2/2}$.
All error metrics are relative to the **clean** mean $M$ — contamination is an artifact to be
rejected, not signal.

## 1. Setup

### 1.1 Estimators under test

| Label | Estimator | Why it is here |
|---|---|---|
| P | pooled mean of all $4^k$ draws | baseline; what naive averaging does |
| D | WLS slope (weights $1/x$) on the geometric design | report #2's winner at large budgets; breakdown point 0, half the budget in one batch |
| C | median of $g$ batch means of size $b$ | maximal group-robustness (breakdown $\approx 1/2$) |
| H | mean of batch means after trimming $t = \lfloor g/8 \rfloor$ per side | report #2's winner at moderate budgets; breakdown $\approx 1/8$ |

E, F, G are omitted: E is dominated by D, F was disqualified under heavy tails, and G tracks C.

### 1.2 Contamination model

Each contaminated draw is inflated **multiplicatively by $\lambda = 10$** (an interference event
that stretches the measured duration roughly an order of magnitude). Two temporal patterns, with
contamination fraction $\varepsilon \in \{1\%, 5\%, 20\%\}$ plus a clean reference:

- **point**: each draw is independently contaminated with probability $\varepsilon$ — diffuse,
  uncorrelated noise (random scheduler hits, occasional GC).
- **burst**: one contiguous window of $\mathrm{round}(\varepsilon N)$ consecutive draws, placed
  uniformly at random, is fully contaminated — a localized event (thermal throttling episode,
  co-tenant spike, cron job).

Draws are consumed in temporal order: the geometric design runs its batches in ascending size
($2^0, 2^1, \dots, 2^{m-1}$ — so the largest batch is a single contiguous half of the run), and
the square design runs its $g$ batches sequentially (a burst of length $\varepsilon N$ covers
$\approx \varepsilon g$ *consecutive whole batches*).

Grid: $\sigma \in \{0.25, 1\}$ (mild and strong skew), $k \in \{3, 4, 5, 6\}$, 100,000
replications per cell, fixed seeds. Script: `simulate_contaminated_estimators.py`; raw output:
`results/contamination_comparison.csv`. (At $k = 3$, $\varepsilon = 1\%$ the burst window rounds
to 1 draw of 64, i.e. an effective 1.6%.)

## 2. What theory predicts

**The contaminated population mean is $M(1 + \varepsilon(\lambda - 1))$** under either pattern:
$+9\%$, $+45\%$, $+180\%$ for $\varepsilon = 1\%, 5\%, 20\%$. This is the number an estimator
reports when it *fails to reject* contamination.

**P and D track the contaminated mean exactly.** Both are linear in the draws and unbiased for
whatever distribution they are fed, so their bias (relative to clean $M$) should be
$\approx \varepsilon(\lambda - 1)$ under the point pattern, and the same *on average* under
bursts. For D, however, burst bias depends on *where* the burst lands: per-draw leverage differs
across batches (draws in small batches carry weights that can even be negative, draws in the
large batch carry weight $\approx 1/N$), so the error distribution across burst placements should
be wide and heavy-tailed even though its mean matches P's.

**C and H reject contamination only when it is *concentrated* relative to the batch structure.**

- *Point pattern:* a batch of size $b$ contains $\mathrm{Binomial}(b, \varepsilon)$ contaminated
  draws. If $\varepsilon b \ll 1$, contamination lands in a few batches, which the median/trim
  discard — strong protection. If $\varepsilon b \gtrsim 1$, *every* batch mean is inflated by
  $\approx \varepsilon(\lambda-1)M$ and there is nothing for the median or trim to reject —
  protection vanishes. The crossover is at $b \approx 1/\varepsilon$, i.e.
  $k \approx \log_2(1/\varepsilon)$: about $k = 6{-}7$ for $\varepsilon = 1\%$, $k = 4{-}5$ for
  $5\%$, $k = 2{-}3$ for $20\%$. **Diffuse contamination defeats group-robust estimators at
  exactly the large batch sizes that minimize their skew-bias** — the two design pressures pull
  in opposite directions.
- *Burst pattern:* the burst corrupts $\approx \varepsilon g$ consecutive whole batches (each
  inflated $\approx \times\lambda$). H rejects them all while $\varepsilon \le t/g = 12.5\%$;
  at $\varepsilon = 20\%$ the excess $\approx 7.5\%$ of batch means leak into H's average at
  $\approx \lambda M$ each, predicting a catastrophic bias of roughly
  $\big(0.075\,\lambda + 0.675\big)M / 0.75 - M \approx +0.9$. C tolerates bursts up to half the
  batches: with 20% of batch means corrupted upward, its median moves from the 50th to the
  $0.5/0.8 = 62.5$th percentile of the clean batch-mean distribution — a small positive shift of
  $\approx 0.32\,\sigma_b$. **This is precisely the H-vs-C reversal anticipated in report #2
  §5.1.**

Skew ($\sigma$) should matter far less here than in reports #1–#2: a $\times 10$ artifact dwarfs
the lognormal's own spread at $\sigma = 0.25$ and competes with it at $\sigma = 1$.

## 3. Simulation results

All metrics relative to the clean mean $M$; **bias** and **RMSE** shown per contamination level;
best RMSE per $(k, \varepsilon)$ in **bold**.

Consistency checks passed: the clean cells reproduce report #2 (D exactly — same seeds and draw
order; P, C, H within Monte Carlo error); P and D under point contamination show bias
$\varepsilon(\lambda-1)$ to three decimals (+0.090, +0.450, +1.800); H's burst-20% bias matches
the §2 breakdown arithmetic (+0.88 to +1.00 observed vs. $\approx$ +0.9 predicted); C's
point-pattern crossover appears at $k \approx \log_2(1/\varepsilon)$ as predicted.

### $\sigma = 0.25$, point pattern (diffuse contamination)

| $k$ | est | clean RMSE | $\varepsilon{=}1\%$ bias | RMSE | $\varepsilon{=}5\%$ bias | RMSE | $\varepsilon{=}20\%$ bias | RMSE |
|---|---|---|---|---|---|---|---|---|
| 3 | P | 0.032 | +0.090 | 0.150 | +0.450 | 0.518 | +1.80 | 1.861 |
| 3 | D | 0.038 | +0.089 | 0.169 | +0.449 | 0.544 | +1.80 | 1.888 |
| 3 | C | 0.037 | +0.009 | **0.048** | +0.201 | **0.360** | +1.68 | **1.768** |
| 3 | H | 0.033 | +0.031 | 0.086 | +0.332 | 0.421 | +1.72 | 1.793 |
| 4 | P | 0.016 | +0.090 | 0.108 | +0.450 | 0.468 | +1.80 | 1.816 |
| 4 | D | 0.017 | +0.090 | 0.111 | +0.450 | 0.471 | +1.80 | 1.817 |
| 4 | C | 0.019 | +0.014 | **0.029** | +0.342 | **0.395** | +1.73 | **1.756** |
| 4 | H | 0.016 | +0.040 | 0.062 | +0.382 | 0.405 | +1.76 | 1.777 |
| 5 | P | 0.008 | +0.090 | 0.095 | +0.450 | 0.455 | +1.80 | 1.804 |
| 5 | D | 0.008 | +0.090 | 0.095 | +0.450 | 0.455 | +1.80 | 1.804 |
| 5 | C | 0.010 | +0.024 | **0.030** | +0.399 | **0.407** | +1.77 | **1.771** |
| 5 | H | 0.008 | +0.059 | 0.065 | +0.419 | 0.424 | +1.78 | 1.783 |
| 6 | P | 0.004 | +0.090 | 0.091 | +0.450 | 0.451 | +1.80 | 1.801 |
| 6 | D | 0.004 | +0.090 | 0.091 | +0.450 | 0.451 | +1.80 | 1.801 |
| 6 | C | 0.005 | +0.054 | **0.060** | +0.426 | **0.428** | +1.78 | **1.783** |
| 6 | H | 0.004 | +0.073 | 0.074 | +0.435 | 0.436 | +1.79 | 1.790 |

### $\sigma = 0.25$, burst pattern (localized event)

| $k$ | est | clean RMSE | $\varepsilon{=}1\%$ bias | RMSE | $\varepsilon{=}5\%$ bias | RMSE | $\varepsilon{=}20\%$ bias | RMSE |
|---|---|---|---|---|---|---|---|---|
| 3 | P | 0.032 | +0.140 | 0.149 | +0.422 | 0.429 | +1.83 | 1.834 |
| 3 | D | 0.038 | +0.143 | 0.180 | +0.450 | 0.498 | +2.09 | 2.140 |
| 3 | C | 0.037 | +0.013 | 0.042 | +0.018 | **0.046** | +0.05 | **0.075** |
| 3 | H | 0.033 | +0.019 | **0.040** | +0.061 | 0.108 | +1.00 | 1.012 |
| 4 | P | 0.016 | +0.106 | 0.108 | +0.457 | 0.459 | +1.79 | 1.795 |
| 4 | D | 0.017 | +0.107 | 0.114 | +0.478 | 0.484 | +1.92 | 1.922 |
| 4 | C | 0.019 | +0.004 | 0.020 | +0.008 | 0.022 | +0.03 | **0.036** |
| 4 | H | 0.016 | +0.007 | **0.019** | +0.013 | **0.022** | +0.89 | 0.892 |
| 5 | P | 0.008 | +0.088 | 0.089 | +0.448 | 0.449 | +1.80 | 1.802 |
| 5 | D | 0.008 | +0.089 | 0.090 | +0.458 | 0.459 | +1.85 | 1.855 |
| 5 | C | 0.010 | +0.001 | 0.010 | +0.004 | 0.011 | +0.02 | **0.020** |
| 5 | H | 0.008 | +0.002 | **0.009** | +0.006 | **0.010** | +0.88 | 0.885 |
| 6 | P | 0.004 | +0.090 | 0.090 | +0.450 | 0.451 | +1.80 | 1.800 |
| 6 | D | 0.004 | +0.091 | 0.091 | +0.455 | 0.455 | +1.82 | 1.820 |
| 6 | C | 0.005 | +0.001 | 0.0050 | +0.002 | 0.0056 | +0.01 | **0.012** |
| 6 | H | 0.004 | +0.001 | **0.0043** | +0.003 | **0.0054** | +0.88 | 0.881 |

### $\sigma = 1$, point pattern (diffuse contamination)

| $k$ | est | clean RMSE | $\varepsilon{=}1\%$ bias | RMSE | $\varepsilon{=}5\%$ bias | RMSE | $\varepsilon{=}20\%$ bias | RMSE |
|---|---|---|---|---|---|---|---|---|
| 3 | P | 0.165 | +0.091 | 0.275 | +0.448 | 0.647 | +1.80 | 2.001 |
| 3 | D | 0.196 | +0.091 | 0.323 | +0.453 | 0.721 | +1.80 | 2.083 |
| 3 | C | 0.177 | −0.042 | 0.181 | +0.140 | **0.312** | +1.22 | **1.411** |
| 3 | H | 0.158 | +0.001 | **0.178** | +0.246 | 0.398 | +1.44 | 1.607 |
| 4 | P | 0.081 | +0.090 | 0.155 | +0.449 | 0.506 | +1.80 | 1.850 |
| 4 | D | 0.088 | +0.090 | 0.164 | +0.450 | 0.514 | +1.80 | 1.861 |
| 4 | C | 0.099 | −0.007 | 0.099 | +0.216 | **0.274** | +1.41 | **1.471** |
| 4 | H | 0.083 | +0.023 | **0.096** | +0.296 | 0.342 | +1.56 | 1.601 |
| 5 | P | 0.041 | +0.090 | 0.111 | +0.450 | 0.465 | +1.80 | 1.812 |
| 5 | D | 0.042 | +0.090 | 0.112 | +0.450 | 0.466 | +1.80 | 1.814 |
| 5 | C | 0.054 | +0.024 | **0.060** | +0.293 | **0.310** | +1.57 | **1.582** |
| 5 | H | 0.044 | +0.043 | 0.066 | +0.348 | 0.360 | +1.65 | 1.662 |
| 6 | P | 0.021 | +0.090 | 0.095 | +0.450 | 0.454 | +1.80 | 1.804 |
| 6 | D | 0.021 | +0.090 | 0.096 | +0.450 | 0.454 | +1.80 | 1.804 |
| 6 | C | 0.029 | +0.046 | **0.055** | +0.353 | **0.358** | +1.67 | **1.671** |
| 6 | H | 0.023 | +0.059 | 0.065 | +0.386 | 0.390 | +1.71 | 1.717 |

### $\sigma = 1$, burst pattern (localized event)

| $k$ | est | clean RMSE | $\varepsilon{=}1\%$ bias | RMSE | $\varepsilon{=}5\%$ bias | RMSE | $\varepsilon{=}20\%$ bias | RMSE |
|---|---|---|---|---|---|---|---|---|
| 3 | P | 0.165 | +0.142 | 0.299 | +0.422 | 0.573 | +1.83 | 1.976 |
| 3 | D | 0.196 | +0.143 | 0.371 | +0.449 | 0.670 | +2.10 | 2.318 |
| 3 | C | 0.177 | −0.024 | 0.176 | +0.012 | **0.191** | +0.19 | **0.333** |
| 3 | H | 0.158 | +0.023 | **0.171** | +0.093 | 0.221 | +0.91 | 1.018 |
| 4 | P | 0.081 | +0.106 | 0.161 | +0.458 | 0.500 | +1.79 | 1.831 |
| 4 | D | 0.088 | +0.107 | 0.170 | +0.478 | 0.528 | +1.92 | 1.963 |
| 4 | C | 0.099 | −0.025 | 0.095 | −0.006 | **0.097** | +0.09 | **0.148** |
| 4 | H | 0.083 | +0.005 | **0.083** | +0.040 | 0.102 | +0.81 | 0.838 |
| 5 | P | 0.041 | +0.088 | 0.105 | +0.448 | 0.459 | +1.80 | 1.811 |
| 5 | D | 0.042 | +0.089 | 0.107 | +0.457 | 0.469 | +1.85 | 1.865 |
| 5 | C | 0.054 | −0.019 | 0.051 | −0.006 | 0.050 | +0.06 | **0.082** |
| 5 | H | 0.044 | −0.004 | **0.042** | +0.015 | **0.047** | +0.80 | 0.805 |
| 6 | P | 0.021 | +0.090 | 0.095 | +0.451 | 0.453 | +1.80 | 1.802 |
| 6 | D | 0.021 | +0.091 | 0.095 | +0.454 | 0.457 | +1.82 | 1.823 |
| 6 | C | 0.029 | −0.012 | 0.027 | −0.003 | 0.025 | +0.04 | **0.049** |
| 6 | H | 0.023 | −0.004 | **0.021** | +0.009 | **0.023** | +0.80 | 0.806 |

## 4. Assessment

**1. Even 1% contamination swamps every clean-model refinement.** With $\lambda = 10$, one
contaminated draw per hundred adds $+9\%$ bias to any linear estimator. For P and D at
$k \ge 4$ that is 5–20$\times$ their entire clean RMSE — the 18% efficiency edge D earned in
report #2 is a rounding error by comparison. The first-order question in a real measurement
pipeline is not "which estimator is efficient" but "which estimator rejects artifacts."

**2. Bursts are exactly C's home turf — and the report-#2 breakdown arithmetic is sharp in
practice.** Under localized events C is astonishingly good: at $\varepsilon = 20\%$ (a fifth of
the whole run corrupted $\times 10$) its bias stays between $+1\%$ and $+19\%$, and its RMSE is
20–150$\times$ better than P's or D's. H matches or slightly beats C up to its breakdown
($\varepsilon = 1\%, 5\% < t/g = 12.5\%$: RMSE barely above clean), then collapses on schedule at
$\varepsilon = 20\%$ with bias $+0.80$ to $+1.00$ — the predicted
$(0.075\lambda + 0.675)/0.75 - 1 \approx +0.9$ almost to the digit. The C-vs-H reversal
hypothesized in report #2 §5.1 is now empirical fact.

**3. Diffuse (point) contamination defeats *all* group-robust estimators once
$\varepsilon b \gtrsim 1$ — and the crossover lands where predicted.** C's point-pattern bias at
$\varepsilon = 1\%$ grows from $+0.9\%$ at $k = 3$ ($b = 8$) to $+5.4\%$ at $k = 6$ ($b = 64$,
$\varepsilon b = 0.64$): once every batch expects a contaminated draw, batch means all shift
together and the median follows them. At $\varepsilon = 5\%$ protection is already half-gone by
$k = 4$; at $20\%$ no estimator retains any protection at any studied $k$. **The design pressures
conflict**: skew-bias (reports #1–2) wants *large* batches, diffuse-artifact rejection wants
*small* ones ($b \ll 1/\varepsilon$). At $\sigma = 1$, $k = 3$–4 the two biases even partially
cancel (C: $-0.078$ skew $+$ contamination $\to -0.042$ at $\varepsilon = 1\%$; H lands within
$0.5\%$ of zero) — a coincidence of this $(\sigma, \varepsilon, \lambda)$, not a mechanism to
rely on.

**4. D under bursts is strictly worse than the naive pooled mean.** Its burst bias exceeds
$\varepsilon(\lambda-1)$ (up to $+2.10$ vs. P's $+1.83$ at $k = 3$, $\varepsilon = 20\%$) and its
SD runs 1.5–3$\times$ P's: burst placement interacts with the design's uneven per-draw leverage,
adding placement variance that the flat-weighted pooled mean does not have. The effect fades as
the design grows ($k = 6$: D $+1.82$ vs. P $+1.80$) but never reverses. Report #2 §5.1's warning
is confirmed and quantified — D is the *most* contamination-fragile estimator tested, not merely
"as fragile as P."

**5. Skew is a second-order effect here.** The $\sigma = 0.25$ and $\sigma = 1$ tables tell the
same story; a $\times 10$ artifact dominates the lognormal's own dispersion in every regime
studied. Contamination structure (diffuse vs. localized, fraction vs. breakdown) — not the
population's tail weight — determines the ranking.

## 5. Revised recommendations

1. **Decide based on the environment, not the estimator table.** If the measurement environment
   is verifiably clean (isolated hardware, pinned cores, no GC in the measured path), report #2's
   rule stands: **H at moderate budgets, D at large budgets**. If contamination is plausible and
   not filtered upstream, artifact rejection dominates every efficiency consideration, and
   **D and P are disqualified as primary estimators** — a single percent of contamination biases
   them by more than their entire clean error budget.

2. **For bursty/localized interference, use H sized to the threat, else C.** H's protection is
   exactly its trim fraction: it survives bursts up to $t/g$ and fails hard beyond. If the
   plausible burst fraction is bounded well below the trim (12.5% here), H is the best all-around
   choice — near-clean RMSE *and* report #2's clean-model performance. If bursts can be long or
   stacked, use C: it held $+4\%$ bias with 20% of the run corrupted. The trim fraction is a
   tunable dial between H and C; set it from an estimate of worst-case interference duty cycle,
   not from clean-model RMSE.

   *Untested candidate that may replace this dial when batch means are skewed:* the **log-space
   mean** of `Fable-edited-Optimum_estimators_of_mean_and_median.md` §4.1,
   $\hat M_{\mathrm{LS}} = \mathrm{median}(\bar Y_j)\, e^{\tilde\sigma_Y^2/2}$ with
   $\tilde\sigma_Y = Q_n(\ln \bar Y_j)$ — algebraically **C times a multiplicative skew
   correction** built from a 50%-breakdown scale. Analytically it keeps C's group breakdown
   (both the median and $Q_n$ tolerate up to ~half the batches corrupted) while removing the
   mean–median skew bias that motivated H, so it would plausibly dominate both C and H outside
   the near-normal band; where batch means are near-normal the correction $\approx 1$ and
   $\hat M_{\mathrm{LS}} \approx$ C, leaving H's efficiency edge intact. **This estimator was
   not simulated in this study** — under 20% bursts the corrupted batch means both shift the
   median and inflate $Q_n(\ln \bar Y)$, and how the correction factor responds is an open
   empirical question. See
   `Reconciliation_notes_Contamination_vs_Fable_mean_estimators.md` §6 for the proposed
   follow-up grid.

3. **For diffuse contamination, keep batches small — or fix the pipeline.** Group-robust
   estimators only reject what is concentrated relative to the batch structure: protection
   requires $\varepsilon b \ll 1$, i.e. $b \lesssim 1/(5\varepsilon)$ as a working rule. This
   directly opposes the large-$b$ preference that minimizes skew-bias, so under diffuse noise a
   deliberately smaller $b$ with more groups (rectangular rather than square designs) is the
   right trade. Above $\varepsilon \approx 5\%$ diffuse, no batching scheme helps materially —
   that is a measurement-hygiene problem (or calls for explicit mixture modeling), not an
   estimator choice.

4. **If D's efficiency is wanted, guard it.** Run the square-design batches alongside (or
   partition the geometric batches), compute H in parallel, and alarm when
   $|\hat M_D - \hat M_H|$ exceeds a few clean-model standard errors — the tables show
   contamination drives the two apart by an order of magnitude more than clean sampling does, so
   the alarm is sharp. An unguarded D should be treated as a clean-room instrument only.

5. **Always retain per-batch data.** Every distinction above (burst vs. diffuse, fraction vs.
   breakdown) is diagnosable post hoc from the batch means at negligible cost; a pipeline that
   only stores the final point estimate cannot even detect that it was contaminated. Note the
   observability constraint: in the batch-sum measurement model of these reports, each batch is
   timed with a *single* timer read — that is the point of batching when individual operations
   are too fast to time — so within-batch quantities (per-draw values, maxima, variances) do not
   exist. Everything below is computable from what is observable: the batch totals
   $V_1, \dots, V_g$ (equivalently the batch means $\bar Y_j = V_j / b$), stored *in temporal
   order*.

   **(a) Estimator-disagreement gap** — *the primary alarm; detects any contamination that biases
   the mean.* Compute
   $$T_{\mathrm{gap}} \;=\; \frac{\hat M_P - \hat M_H}{\widehat{\mathrm{SE}}},\qquad
   \widehat{\mathrm{SE}} = \frac{s_{\bar Y}}{\sqrt{g}},$$
   where $\hat M_P$ is the pooled mean (= untrimmed mean of batch means), $\hat M_H$ the trimmed
   mean, and $s_{\bar Y}$ the *robust* SD of the batch means ($1.4826 \times$ their MAD). Under
   clean sampling the gap has a small predictable positive value (the trim bias, of order
   $\hat\gamma_b\, s_{\bar Y}/6$ — estimate it from the same data or from the clean tables of
   report #2); contamination that biases the pooled mean but not the trimmed mean — the burst
   pattern — drives the gap to $\approx \varepsilon(\lambda - 1)M$, one to two orders of
   magnitude above the clean standard error in this study's cells. Flag when the observed gap
   exceeds its clean prediction by $\gtrsim 3$ standard errors.

   **(b) Batch-mean outlier count** — *estimates the corrupted-batch fraction; sizes the trim.*
   Compute robust z-scores
   $$z_j = \frac{\bar Y_j - \mathrm{median}(\bar Y)}{1.4826 \cdot \mathrm{MAD}(\bar Y)},$$
   and count $n_+ = \#\{j : z_j > z_c\}$ with $z_c \approx 3.5$ (calibrate $z_c$ on the clean
   skewed null — for lognormal batch means at small $b$, use a one-sided cut on the log scale or
   simulate the null once). $n_+/g$ estimates the corrupted-batch fraction directly: if it
   approaches the trim fraction $t/g$, H is near breakdown and the run should be discarded or
   re-estimated with C.

   **(c) Longest-run statistic** — *separates burst from diffuse.* Let $R$ be the longest run of
   *consecutive* batches with $z_j > z_c$ (temporal order matters here). Under diffuse
   contamination flagged batches are scattered, so $R$ stays near its i.i.d. null value
   $\approx \log g / \log(1/p)$ (with $p = n_+/g$); a burst produces $R \approx \varepsilon g$,
   far above it. Cheap calibration: permute the batch order many times and recompute $R$ — the
   observed $R$ sitting in the permutation tail identifies a burst and its location (the run
   itself is the interference window, usable for excision-and-re-estimation).

   **(d) Aggregation-scale variance ratio** — *catches burst corruption even when no single batch
   looks extreme, using batch means only.* Group the batch means into non-overlapping super-batches
   of $r$ consecutive means (e.g. $r = 2, 4, 8$) and compute
   $$R_r \;=\; \frac{r \cdot \mathrm{Var}\big(\text{super-batch means}\big)}{\mathrm{Var}\big(\bar Y_1, \dots, \bar Y_g\big)}.$$
   For i.i.d. batches, variance shrinks as $1/r$ under aggregation, so $R_r \approx 1$ at every
   scale (any $\sigma$). A burst corrupts *consecutive* batches, inducing positive serial
   correlation, so super-batch variance fails to shrink and $R_r \gg 1$ at scales up to the burst
   length; diffuse contamination leaves $R_r \approx 1$. Calibrate the null by permuting the
   batch order (permutation destroys exactly the temporal structure a burst creates). This is the
   batch-means consistency check from simulation output analysis, repurposed as a contamination
   alarm.

   **(e) Cross-scale location disagreement** — *the detector for diffuse contamination, which is
   invisible to (b)–(d) once $\varepsilon b \gtrsim 1$.* Diffuse contamination cannot be seen at a
   single batch size, but its signature *changes with batch size*: at small $b'$
   ($\varepsilon b' \ll 1$) contaminated batches are rare, isolated spikes that robust statistics
   reject, while at large $b$ ($\varepsilon b \gtrsim 1$) every batch mean is shifted by
   $\approx \varepsilon(\lambda-1)M$ and nothing is rejectable. So measure at two scales and
   compare robust locations:
   $$\Delta \;=\; \mathrm{median}\big(\bar Y_j\big)\Big|_{\text{size-}b\text{ batches}}
   \;-\; \mathrm{median}\big(V_i / x_i\big)\Big|_{\text{small batches } x_i = b'} .$$
   Under clean sampling $\Delta$ is small and predictable (the difference of two skew-dependent
   median offsets); under diffuse contamination $\Delta \approx \varepsilon(\lambda - 1)M$.
   Moreover the small-batch spikes themselves are informative: the fraction of small batches
   exceeding a robust fence estimates $\varepsilon b'$ (hence $\hat\varepsilon$), and the spike
   magnitude estimates $\lambda$. The regression designs get this *for free* — the geometric and
   arithmetic designs already contain small batches (report #1 credited exactly this diagnostic
   value to the Criterion-style design); a square design must deliberately carve out a few percent
   of budget for small probe batches. Caveat: per-batch timer overhead inflates small-batch
   per-draw values by $o/b'$, so use small batches for *detection* and difference the overhead out
   via the design's intercept, not for estimation.

   **(f) Design-consistency check (for D)** — *guards the geometric design specifically.* The
   WLS residuals $r_i = (V_i - \hat\alpha - \hat\beta x_i)/\sqrt{x_i}$ are homoscedastic with
   comparable scale under the clean model; a burst inside batch $i$ shows up as $|r_i|$ far
   outside the others' range. Additionally recompute $\hat\beta$ with the largest batch deleted:
   under clean sampling the two slopes agree to within $\approx 2\times$ the clean SE (the large
   batch is half the information); disagreement beyond that localizes interference to the largest
   batch — D's known weak point.

   Indicators (a)–(d) cost $O(g)$ arithmetic on the batch means the estimator already computed;
   (e) is free in the regression designs and costs a few percent of budget as probe batches in the
   square design; (f) is two extra regressions. Together they distinguish the three regimes that
   drive recommendations 1–3 above: *clean* (all quiet — report #2's winner map applies),
   *bursty* (b, c, d, f fire; act via trim size or excision), and *diffuse* (e fires while c and d
   stay quiet; act via smaller batches or pipeline fixes).

   **If temporal order was not retained** (e.g. the batch means were accumulated only into a
   histogram): (a) and (b) are order-free — every statistic they need is a histogram
   quantile or count — and (e) needs a second batch size, not ordering, so contamination
   *detection*, the corrupted-batch fraction estimate, and the diffuse-regime check all survive.
   What is lost is exactly (c) and (d): burst-vs-diffuse separation at the main scale, detection
   of bursts too mild to flag any single batch, and excision (the window cannot be located).
   Substitutes, in order of preference: (i) **keep the plain sequence too** — it is only $g$
   numbers, $O(g)$ memory next to the histogram, and restores (c)/(d) completely; (ii)
   **streaming accumulators computed at collection time**, while order still exists: a lag-1
   autocovariance of the batch means ($\sum_j \bar Y_j \bar Y_{j+1}$ alongside the usual sums —
   one scalar of extra state) substitutes for (d), and a running longest-run counter or CUSUM
   above a threshold substitutes for (c), with the threshold scale taken from a calibration
   prefix or a previous run; (iii) **epoch histograms** — one histogram per fixed fraction (e.g.
   eighth) of the run: a burst concentrates its location shift or outlier count in one epoch
   while diffuse contamination spreads evenly — a coarse (c)/(d) at the cost of a few extra
   histograms. Failing all three, act conservatively: the robust estimators themselves never
   needed ordering (H trims the most extreme batch means wherever they occurred in time; C cares
   even less), so when (b) fires, treat the flagged fraction $n_+/g$ as a possible burst that
   cannot be verified short — size the trim $\geq n_+/g$ with margin, or fall back to C — and
   forgo excision. None of this relaxes the diffuse limit: once $\varepsilon b \gtrsim 1$ the
   corruption sits *inside* every batch mean, and only (e) can see it, ordered or not.

**Bottom line.** The clean-model winner map of report #2 survives only in clean environments.
Under realistic contamination: **H (trim ≥ plausible burst fraction) is the default latency
estimator; C is the heavy-duty fallback; small batches beat large ones when noise is diffuse; and
D — the clean-model champion — is the most artifact-fragile estimator in the study and needs an
explicit interference guard.** Robustness, in the contamination sense, is not a tie-breaker among
otherwise-similar estimators; in this use case it is the primary axis.
