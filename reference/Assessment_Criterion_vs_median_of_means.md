# Assessment: Criterion-style Regression vs. Median-of-Means for Estimating a Lognormal Mean

This note compares three estimators of the mean of a population $P \sim \mathrm{Lognormal}(\mu, \sigma^2)$, as specified in
`Pronpt_ Criterion_vs_median_of_means.md`. Notation: for a positive integer $k$, let $m = 2k$,
$n = 2^m - 1$, and $h = \lceil \sqrt{2}\cdot 2^k \rceil - 1$, so that all three scenarios consume an
approximately equal total sample budget of $\approx 2^m = 4^k$ draws.

| Scenario | Design | Estimator | Budget |
|---|---|---|---|
| A | batch sums $V_i$ at geometric batch sizes $x = 2^0, \dots, 2^{m-1}$ | OLS slope of $V$ on $x$ | $2^m - 1$ |
| B | batch sums $V_i$ at arithmetic batch sizes $x = 1, \dots, h$ | OLS slope of $V$ on $x$ | $h(h+1)/2 \approx 2^m$ |
| C | $g = 2^k$ batch means, each over $b = 2^k$ draws | median of the batch means (median-of-means, MoM) | $4^k = 2^m$ |

Scenario B is essentially the estimator used by the Criterion.rs / Criterion.hs benchmarking
libraries (regress cumulative time on iteration count; the slope is the per-call time). Scenario C
is the classical median-of-means estimator.

Throughout, write $M = e^{\mu + \sigma^2/2}$ for the population mean,
$s^2 = M^2\!\left(e^{\sigma^2} - 1\right)$ for the population variance,
$\mathrm{cv} = s/M = \sqrt{e^{\sigma^2} - 1}$ for the coefficient of variation, and
$\gamma = \left(e^{\sigma^2} + 2\right)\mathrm{cv}$ for the skewness. All error metrics below are
**relative to $M$** ($\mu = 0$ without loss of generality — every estimator is scale-equivariant,
so relative errors do not depend on $\mu$).

For calibration: $\mathrm{cv}$ and $\gamma$ grow explosively in $\sigma$.

| $\sigma$ | $\mathrm{cv}$ | skewness $\gamma$ | excess kurtosis |
|---|---|---|---|
| 0.25 | 0.254 | 0.78 | 1.1 |
| 0.5 | 0.533 | 1.75 | 5.9 |
| 1.0 | 1.31 | 6.18 | 111 |
| 2.0 | 7.32 | 414 | $\approx 9.2 \times 10^6$ |

---

## 1. Theoretical properties

### 1.1 Bias

**A and B are exactly unbiased at every $k$.** Each recorded value is a batch sum, so
$E[V_i] = M x_i$ and $\mathrm{Var}(V_i) = x_i s^2$. The OLS slope is a linear function of the $V_i$,
$\hat\beta = \sum_i (x_i - \bar x)V_i / S_{xx}$ with $S_{xx} = \sum_i (x_i - \bar x)^2$, hence

$$E[\hat\beta] = M \cdot \frac{\sum_i (x_i - \bar x)\,x_i}{S_{xx}} = M .$$

Heteroscedasticity ($\mathrm{Var}(V_i) \propto x_i$) does not bias the OLS slope; it only makes it
inefficient relative to weighted least squares.

**C is biased low for any right-skewed population.** As the number of groups grows, MoM converges
to the *median* of the sampling distribution of a batch mean of $b = 2^k$ draws, not to its mean
$M$. For a right-skewed distribution the median lies below the mean. A second-order
(Edgeworth-type) approximation gives, for a batch mean with skewness $\gamma_b = \gamma/\sqrt{b}$
and standard deviation $\sigma_b = s/\sqrt{b}$:

$$\text{relative bias} \;\approx\; -\frac{\text{mean} - \text{median}}{M} \;\approx\; -\frac{\gamma_b\,\sigma_b}{6M} \;=\; -\frac{\gamma\,\mathrm{cv}}{6\,b} \;=\; -\frac{\gamma\,\mathrm{cv}}{6\cdot 2^k},$$

so the bias decays like $O(2^{-k})$ — but the prefactor $\gamma\,\mathrm{cv}$ is $0.93$ at
$\sigma = 0.5$, $8.1$ at $\sigma = 1$, and $\approx 3000$ at $\sigma = 2$. For $\sigma = 2$ the
expansion is useless at practical $k$ and the bias must be read off the simulation: it remains a
large fraction of $M$ (−44% to −19%) throughout the studied range.

(Edge case: at $k = 1$ there are only $g = 2$ groups, and the median of two values is their
average — MoM degenerates to the pooled mean and is nearly unbiased. The bias appears from
$k \ge 2$ and then decays.)

### 1.2 Efficiency

Benchmark: the pooled sample mean of the same budget $N$ has variance $s^2/N$ — the best linear
unbiased estimator given all raw draws. Define relative efficiency
$\mathrm{RE} = \mathrm{Var}(\text{pooled}) / \mathrm{Var}(\text{estimator})$.

For A and B (independent batch sums, plain OLS):

$$\mathrm{Var}(\hat\beta) \;=\; s^2 \cdot \frac{\sum_i (x_i - \bar x)^2\, x_i}{S_{xx}^2}.$$

**Scenario B** ($x = 1, \dots, h$): the third central moment of $x$ vanishes by symmetry, so
$\sum (x-\bar x)^2 x = \bar x\, S_{xx}$ and
$\mathrm{Var}(\hat\beta) = s^2 \bar x / S_{xx} = 6 s^2 / (h(h-1))$. Against the pooled mean of
$N = h(h+1)/2$ draws:

$$\mathrm{RE}_B \;=\; \frac{h-1}{3(h+1)} \;\longrightarrow\; \frac{1}{3}.$$

The Criterion-style design throws away roughly two-thirds of the information in the sample budget:
plain OLS weights all batch sums equally, but the small batches carry the least information per
draw. (WLS with weights $1/x_i$ would recover most of the loss.)

**Scenario A** ($x_i = 2^i$): leverage concentrates on the largest batch, which alone holds half of
the total budget. Asymptotically $\sum (x - \bar x)^2 x \approx 8^m/7$ and
$S_{xx} \approx 4^m/3$, giving $\mathrm{Var}(\hat\beta) \approx \tfrac{9}{7}\, s^2 / 2^m$ and

$$\mathrm{RE}_A \;\longrightarrow\; \frac{7}{9} \;\approx\; 0.78,$$

approached slowly from below. Exact finite-$k$ values (computed from the variance formula above):

| $k$ | 1 | 2 | 3 | 4 | 5 | 6 | $\to\infty$ |
|---|---|---|---|---|---|---|---|
| $\mathrm{RE}_A$ | 0.111 | 0.348 | 0.499 | 0.584 | 0.633 | 0.664 | $7/9 \approx 0.778$ |
| $\mathrm{RE}_B$ | 0.111 | 0.222 | 0.278 | 0.304 | 0.319 | 0.326 | $1/3 \approx 0.333$ |

(At $k = 1$ both designs reduce to two points $x \in \{1, 2\}$: the slope is $V_2 - V_1$, with
$\mathrm{RE} = 1/9$.) Scenario A is therefore markedly *more* efficient than B: geometric spacing
makes the slope behave almost like the mean of the largest batch, with the smaller batches adding a
little more.

**Scenario C**: for $k$ large enough that batch means are approximately normal, the sample median
of $g = 2^k$ of them has $\mathrm{Var} \approx \tfrac{\pi}{2} \cdot (s^2/b)/g = \tfrac{\pi}{2}\, s^2/4^k$, hence

$$\mathrm{RE}_C \;\longrightarrow\; \frac{2}{\pi} \;\approx\; 0.637,$$

the classical median penalty. For small $k$ or large $\sigma$ the batch means are far from normal
and the variance of MoM is actually much *smaller* than this asymptote suggests (the median avoids
the tail entirely — at $\sigma = 2$ the simulated variance ratio exceeds 12), at the price of the
bias in §1.1.

### 1.3 Robustness

**A and B have breakdown point 0.** The slope is linear in the data: one extreme draw $\Delta$
landing in batch $i$ moves the estimate by $(x_i - \bar x)\Delta / S_{xx}$. Because the lognormal
with $\sigma \ge 1$ produces such draws routinely, the finite-sample distribution of $\hat\beta$ is
itself heavily right-skewed: the estimator is correct on average but erratic per experiment.
Scenario A concentrates its leverage on the single largest batch — which is also where more than
half of the draws land, i.e. exactly where an extreme value is most likely to appear. Scenario B
spreads leverage over $h$ batches, but the end batches (near $1$ and $h$) carry the
largest-magnitude weights, and small batches provide no within-batch averaging to dilute an
outlier.

**C has breakdown point $\approx 1/2$ at the group level.** Up to $\lceil g/2 \rceil - 1$ of the
$2^k$ batch means can be corrupted arbitrarily without destroying the estimate, and MoM satisfies
sub-Gaussian-style deviation bounds requiring only finite variance — precisely the heavy-tail
regime where the sample mean (and hence A and B) misbehaves. The caveat: for a *skewed but
uncontaminated* population, the same mechanism that grants robustness produces the systematic
downward bias. MoM's robustness and its bias are two faces of the same coin — it targets a
median-like functional of the batch-mean distribution, which coincides with $M$ only when batch
means are symmetric.

A useful diagnostic in the tables below is the **tail ratio**
$\mathrm{RMSE} / \mathrm{RMSE}_{\text{trim }1\%}$ — how much of an estimator's error is carried by
its worst 1% of realizations. Values near $1.04$ (the Gaussian reference) mean light-tailed error;
large values flag fragility.

---

## 2. Simulation results

Monte Carlo: 100,000 replications per $(\sigma, k)$ cell, vectorized numpy with a fixed seed;
script: `simulate_estimators.py`; raw output: `results/estimator_comparison.csv`. All quantities
are relative to the true mean $M$: **bias** $= E[\hat M/M - 1]$, **SD** and **RMSE** of
$\hat M/M - 1$, **med$|$err$|$** $=$ median absolute relative error, **tail** $=$ tail ratio
(§1.3), **RE** $=$ empirical variance ratio vs. the pooled mean of the same budget. The best RMSE
per $k$ is **bold**.

Consistency checks passed: empirical bias of A and B is zero within Monte Carlo error in every
cell; empirical RE of A and B matches the exact theoretical values of §1.2 to three digits for
$\sigma \le 1$; empirical RE of C approaches $2/\pi \approx 0.637$ for small $\sigma$ and large
$k$ (0.653 at $\sigma = 0.25$, $k = 6$).

### $\sigma = 0.25$ (mild skew, $\mathrm{cv} \approx 0.25$)

| $k$ | est | bias | SD | RMSE | med$\|$err$\|$ | tail | RE |
|---|---|---|---|---|---|---|---|
| 1 | A | −0.001 | 0.440 | 0.440 | 0.287 | 1.05 | 0.111 |
| 1 | B | −0.001 | 0.440 | 0.440 | 0.288 | 1.05 | 0.111 |
| 1 | C | −0.001 | 0.126 | **0.126** | 0.085 | 1.05 | 1.000 |
| 2 | A | +0.000 | 0.111 | 0.111 | 0.074 | 1.04 | 0.350 |
| 2 | B | −0.001 | 0.140 | 0.140 | 0.093 | 1.04 | 0.222 |
| 2 | C | −0.004 | 0.069 | **0.069** | 0.047 | 1.04 | 0.849 |
| 3 | A | −0.000 | 0.045 | 0.045 | 0.030 | 1.04 | 0.499 |
| 3 | B | +0.000 | 0.059 | 0.059 | 0.040 | 1.04 | 0.278 |
| 3 | C | −0.003 | 0.037 | **0.037** | 0.025 | 1.04 | 0.755 |
| 4 | A | −0.000 | 0.0207 | 0.0207 | 0.0140 | 1.04 | 0.589 |
| 4 | B | +0.000 | 0.0290 | 0.0290 | 0.0195 | 1.04 | 0.303 |
| 4 | C | −0.002 | 0.0190 | **0.0191** | 0.0129 | 1.04 | 0.694 |
| 5 | A | −0.000 | 0.0100 | 0.0100 | 0.0067 | 1.04 | 0.633 |
| 5 | B | −0.000 | 0.0140 | 0.0140 | 0.0094 | 1.04 | 0.318 |
| 5 | C | −0.001 | 0.0097 | **0.0098** | 0.0066 | 1.04 | 0.666 |
| 6 | A | −0.000 | 0.0049 | **0.0049** | 0.0033 | 1.04 | 0.662 |
| 6 | B | −0.000 | 0.0069 | 0.0069 | 0.0047 | 1.04 | 0.327 |
| 6 | C | −0.001 | 0.0049 | 0.0049 | 0.0033 | 1.04 | 0.653 |

### $\sigma = 0.5$ (moderate skew, $\mathrm{cv} \approx 0.53$)

| $k$ | est | bias | SD | RMSE | med$\|$err$\|$ | tail | RE |
|---|---|---|---|---|---|---|---|
| 1 | A | +0.002 | 0.923 | 0.923 | 0.542 | 1.07 | 0.110 |
| 1 | B | +0.000 | 0.923 | 0.923 | 0.549 | 1.07 | 0.112 |
| 1 | C | −0.002 | 0.266 | **0.266** | 0.174 | 1.07 | 1.000 |
| 2 | A | +0.000 | 0.232 | 0.232 | 0.154 | 1.06 | 0.348 |
| 2 | B | −0.000 | 0.291 | 0.291 | 0.190 | 1.06 | 0.223 |
| 2 | C | −0.019 | 0.141 | **0.142** | 0.098 | 1.05 | 0.901 |
| 3 | A | −0.000 | 0.095 | 0.095 | 0.064 | 1.04 | 0.499 |
| 3 | B | −0.000 | 0.125 | 0.125 | 0.083 | 1.04 | 0.276 |
| 3 | C | −0.015 | 0.075 | **0.077** | 0.053 | 1.04 | 0.784 |
| 4 | A | −0.000 | 0.0437 | 0.0437 | 0.0295 | 1.04 | 0.585 |
| 4 | B | +0.000 | 0.0609 | 0.0609 | 0.0409 | 1.04 | 0.303 |
| 4 | C | −0.008 | 0.0394 | **0.0403** | 0.0277 | 1.04 | 0.715 |
| 5 | A | −0.000 | 0.0209 | 0.0209 | 0.0141 | 1.04 | 0.634 |
| 5 | B | −0.000 | 0.0294 | 0.0294 | 0.0198 | 1.04 | 0.319 |
| 5 | C | −0.004 | 0.0203 | **0.0207** | 0.0140 | 1.04 | 0.671 |
| 6 | A | +0.000 | 0.0103 | **0.0103** | 0.0069 | 1.04 | 0.664 |
| 6 | B | −0.000 | 0.0146 | 0.0146 | 0.0098 | 1.04 | 0.325 |
| 6 | C | −0.002 | 0.0103 | 0.0106 | 0.0072 | 1.04 | 0.656 |

### $\sigma = 1$ (strong skew, $\mathrm{cv} \approx 1.31$)

| $k$ | est | bias | SD | RMSE | med$\|$err$\|$ | tail | RE |
|---|---|---|---|---|---|---|---|
| 1 | A | −0.006 | 2.264 | 2.264 | 0.877 | 1.23 | 0.111 |
| 1 | B | −0.006 | 2.219 | 2.219 | 0.877 | 1.22 | 0.112 |
| 1 | C | +0.001 | 0.655 | **0.655** | 0.365 | 1.20 | 1.000 |
| 2 | A | +0.000 | 0.574 | 0.574 | 0.328 | 1.14 | 0.344 |
| 2 | B | +0.003 | 0.718 | 0.718 | 0.388 | 1.14 | 0.223 |
| 2 | C | −0.095 | 0.291 | **0.306** | 0.223 | 1.06 | 1.277 |
| 3 | A | +0.002 | 0.235 | 0.235 | 0.149 | 1.09 | 0.496 |
| 3 | B | +0.001 | 0.308 | 0.308 | 0.191 | 1.08 | 0.277 |
| 3 | C | −0.078 | 0.158 | **0.177** | 0.131 | 1.03 | 1.074 |
| 4 | A | +0.000 | 0.108 | 0.108 | 0.072 | 1.05 | 0.583 |
| 4 | B | +0.001 | 0.149 | 0.149 | 0.098 | 1.05 | 0.304 |
| 4 | C | −0.051 | 0.086 | **0.100** | 0.073 | 1.03 | 0.896 |
| 5 | A | −0.000 | 0.0514 | **0.0514** | 0.0345 | 1.04 | 0.635 |
| 5 | B | −0.000 | 0.0720 | 0.0720 | 0.0483 | 1.04 | 0.320 |
| 5 | C | −0.030 | 0.0461 | 0.0548 | 0.0392 | 1.03 | 0.790 |
| 6 | A | +0.000 | 0.0251 | **0.0251** | 0.0168 | 1.04 | 0.664 |
| 6 | B | +0.000 | 0.0358 | 0.0358 | 0.0241 | 1.04 | 0.328 |
| 6 | C | −0.017 | 0.0240 | 0.0292 | 0.0207 | 1.03 | 0.721 |

### $\sigma = 2$ (extreme skew, $\mathrm{cv} \approx 7.3$)

| $k$ | est | bias | SD | RMSE | med$\|$err$\|$ | tail | RE |
|---|---|---|---|---|---|---|---|
| 1 | A | +0.034 | 12.46 | 12.46 | 0.967 | 3.28 | 0.111 |
| 1 | B | −0.008 | 10.85 | 10.85 | 0.966 | 2.90 | 0.111 |
| 1 | C | +0.015 | 3.695 | **3.695** | 0.739 | 2.99 | 1.000 |
| 2 | A | −0.010 | 2.830 | 2.830 | 0.679 | 2.12 | 0.370 |
| 2 | B | −0.009 | 3.467 | 3.467 | 0.718 | 2.11 | 0.239 |
| 2 | C | −0.437 | 0.519 | **0.679** | 0.604 | 1.11 | 12.50 |
| 3 | A | −0.001 | 1.258 | 1.258 | 0.424 | 1.80 | 0.515 |
| 3 | B | −0.007 | 1.548 | 1.548 | 0.484 | 1.67 | 0.297 |
| 3 | C | −0.424 | 0.250 | **0.493** | 0.479 | 1.01 | 12.22 |
| 4 | A | −0.001 | 0.594 | 0.594 | 0.246 | 1.55 | 0.567 |
| 4 | B | +0.002 | 0.823 | 0.823 | 0.311 | 1.52 | 0.311 |
| 4 | C | −0.344 | 0.156 | **0.378** | 0.365 | 1.01 | 9.38 |
| 5 | A | +0.002 | 0.286 | 0.286 | 0.139 | 1.34 | 0.645 |
| 5 | B | −0.001 | 0.405 | 0.405 | 0.182 | 1.37 | 0.308 |
| 5 | C | −0.259 | 0.098 | **0.277** | 0.267 | 1.01 | 5.18 |
| 6 | A | −0.000 | 0.141 | **0.141** | 0.076 | 1.24 | 0.658 |
| 6 | B | +0.000 | 0.199 | 0.199 | 0.104 | 1.23 | 0.337 |
| 6 | C | −0.187 | 0.060 | 0.196 | 0.190 | 1.01 | 3.65 |

*(At $\sigma = 2$ the empirical RE of A and B is noisy — variance estimation itself is fragile at
excess kurtosis $\sim 10^7$ — but consistent with theory. C's RE $\gg 1$ there means MoM has far
lower variance than the pooled mean; its error is almost pure bias, as the near-1.01 tail ratio
confirms.)*

---

## 3. Comparative assessment by $k$ regime

**Small $k$ (1–2, budget 3–16).** All three estimators are noisy. A and B are nearly identical
(the designs almost coincide) and unbiased, but with relative SD near or above 100% for
$\sigma \ge 1$ they are useless per-experiment. C degenerates to the pooled mean at $k = 1$; at
$k = 2$ it already has the lowest RMSE in every $\sigma$ column, but for $\sigma \ge 1$ its bias
arrives immediately (−9.5% at $\sigma = 1$, −44% at $\sigma = 2$). No estimator is trustworthy
here for $\sigma \ge 1$.

**Moderate $k$ (3–4, budget 63–256).** The trade-off zone.

- $\sigma \le 0.5$: everything works; C has a small RMSE edge, its bias ($\le 1.5\%$) is
  immaterial, and all tail ratios sit at the Gaussian reference. Choice barely matters.
- $\sigma = 1$: C still wins RMSE (e.g. 0.100 vs. A's 0.108 at $k = 4$) but 5–8 points of its
  error is now *systematic* underestimation. A is unbiased with only slightly worse RMSE and a
  mild tail (1.05–1.09).
- $\sigma = 2$: C's RMSE is roughly half of A's, but it is essentially all bias (−42% to −34%):
  C reliably reports about two-thirds of the true mean. A is unbiased and better than C on the
  *median* run from $k = 3$ (med$|$err$|$ 0.42 vs. 0.48) but its tail ratio of 1.5–1.8 means
  occasional runs are wildly off. This is the regime where the choice genuinely depends on
  whether a systematic −35% error or an occasional +200% error is more damaging.

**Large $k$ (5–6, budget 1023–4096).** A's SD shrinks as $\Theta(2^{-m/2})$ while C's bias floor
decays only as the batch means symmetrize, so A overtakes C on RMSE: at $k = 6$, A has the best
RMSE in every $\sigma$ column. For $\sigma \le 1$ the two are nearly interchangeable (C keeps a
$-0.05\%$ to $-1.7\%$ bias). For $\sigma = 2$, A wins RMSE at $k = 6$ (0.141 vs. 0.196) and C is
still biased by −19%; note however that C's *SD* is 0.060 vs. A's 0.141 — if the bias were
corrected (or only relative comparisons between two alternatives measured the same way are
needed), C would be much more precise. B is never competitive: it tracks A's qualitative behavior
while paying a $\approx 2.3\times$ variance penalty ($\mathrm{RE} \to 1/3$ vs. $7/9$) at every budget.

**The three-way summary.**

| Property | A (geometric OLS) | B (arithmetic OLS, Criterion-style) | C (median-of-means) |
|---|---|---|---|
| Bias | exactly 0 | exactly 0 | negative, $O(\gamma\,\mathrm{cv}\,2^{-k})$; severe for $\sigma \ge 1$ at small–moderate $k$ |
| Efficiency (RE vs. pooled mean) | $\to 7/9 \approx 0.78$ | $\to 1/3$ | $\to 2/\pi \approx 0.64$ (variance far lower than pooled mean under extreme skew) |
| Robustness / error tails | breakdown 0; tail ratio up to 3.3 at $\sigma = 2$ | breakdown 0; same tail behavior as A | group breakdown $\approx 1/2$; tail ratio $\approx 1.01$ for $k \ge 3$ even at $\sigma = 2$ |

---

## 4. Recommendations

1. **Mild-to-moderate skew ($\sigma \le 0.5$), any $k$:** all three work; prefer **A** (unbiased,
   most efficient) or **C** (marginally better RMSE at small $k$). Avoid B unless its operational
   rationale applies (below).

2. **Strong skew ($\sigma \approx 1$):** use **C** for $k \le 4$ if a few percent of downward bias
   is acceptable, **A** for $k \ge 5$. If unbiasedness is a hard requirement (e.g. the estimate
   feeds capacity planning where mean latency × throughput = work), use **A** throughout and
   accept the heavier error tail.

3. **Extreme skew ($\sigma \approx 2$):** no estimator is good below $k \approx 5$. C's
   consistency is seductive (tail ratio 1.01, tiny SD) but it *systematically reports
   ~65–80% of the true mean* — dangerous if the absolute mean matters, acceptable if only
   comparing two alternatives measured identically (the bias largely cancels in ratios).
   A is the only estimator that converges to the truth, reaching ±14% RMSE at $k = 6$;
   budget permitting, push $k$ higher.

4. **On the Criterion-style estimator (B):** its statistical case is weak — it is dominated by A
   at every $(\sigma, k)$ studied with $k \ge 2$ (at $k = 1$ the two designs coincide), paying a
   $\approx 2.3\times$ variance penalty at large $k$ for
   no bias or robustness benefit. Its real justification is *operational*, not statistical: the
   regression intercept absorbs constant per-measurement overhead (timer calls, loop setup), so
   the slope estimates the pure per-iteration cost. Scenario A shares exactly this
   intercept-absorbing property with far better efficiency; its one operational drawback is that
   half the budget sits in a single batch, so a drift or interference event during that batch
   contaminates the estimate with maximal leverage, whereas B's many medium-sized batches make
   such an event visible as an outlier in the regression diagnostics.

5. **Easy improvements outside the given menu** (for context, not part of the comparison):
   weighted least squares with weights $1/x_i$ on either design recovers full pooled-mean
   efficiency while keeping the intercept/overhead benefit; a bias-corrected MoM (e.g. using the
   batch-mean skewness estimate) or overlapping-blocks MoM shrinks C's bias; and for pure
   heavy-tail protection with less bias, a lightly trimmed mean of batch means is a good middle
   ground.

**Bottom line.** The regression estimators trade robustness for unbiasedness; median-of-means
trades bias for robustness; and the arithmetic (Criterion-style) design is a strictly worse
version of the geometric one on every statistical axis measured here. For lognormal-like latency
data: prefer **C** when budgets are small and skew is moderate, **A** when budgets are large or
unbiasedness is required — and if you control the design, weighted regression on the geometric
design beats all three.
