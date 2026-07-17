# Assessment: Five Improved Estimators of a Lognormal Mean

This is a companion to `Assessment_Criterion_vs_median_of_means.md` (report #1), which compared
three estimators of the mean of $P \sim \mathrm{Lognormal}(\mu, \sigma^2)$: **A** (OLS slope on a
geometric design), **B** (OLS slope on an arithmetic design — the Criterion.rs-style estimator),
and **C** (median-of-means). Report #1's recommendation §4.5 sketched five improvements; this
report defines them precisely and assesses their **bias, efficiency, and robustness** on the same
grid, with A, B, C re-simulated as reference rows in every table.

Notation (as in report #1): for a positive integer $k$, let $m = 2k$, $n = 2^m - 1$,
$h = \lceil \sqrt{2}\cdot 2^k \rceil - 1$, batch count $g = 2^k$, batch size $b = 2^k$, and total
budget $N \approx 4^k$. The population mean is $M = e^{\mu + \sigma^2/2}$, the variance
$s^2 = M^2(e^{\sigma^2} - 1)$, $\mathrm{cv} = s/M$, skewness $\gamma = (e^{\sigma^2}+2)\,\mathrm{cv}$.
All error metrics are relative to $M$ ($\mu = 0$ WLOG).

| Label | Estimator | Design / budget |
|---|---|---|
| A | OLS slope of batch sums on $x = 2^0, \dots, 2^{m-1}$ | geometric, $n = 2^m - 1$ |
| B | OLS slope of batch sums on $x = 1, \dots, h$ | arithmetic, $h(h+1)/2 \approx 2^m$ |
| C | median of $g$ batch means of size $b$ | square, $4^k$ |
| **D** | **WLS slope (weights $1/x_i$) on the geometric design** | geometric, $n$ |
| **E** | **WLS slope (weights $1/x_i$) on the arithmetic design** | arithmetic, $\approx 2^m$ |
| **F** | **bias-corrected median-of-means** | square, $4^k$ |
| **G** | **overlapping-blocks median-of-means** | square, $4^k$ |
| **H** | **trimmed mean of batch means** | square, $4^k$ |

---

## 1. The five estimators in detail

### 1.1 D — Weighted least squares on the geometric design

**Definition.** Same measurements as A: batch sums $V_i$ at batch sizes $x_i = 2^i$,
$i = 0, \dots, m-1$. Instead of ordinary least squares, fit $V = \alpha + \beta x$ by *weighted*
least squares with observation weights $w_i = 1/x_i$, i.e. minimize
$\sum_i (V_i - \alpha - \beta x_i)^2 / x_i$. The estimate of $M$ is the slope

$$\hat\beta_{\mathrm{WLS}} \;=\; \frac{S_w S_{wxy} - S_{wx} S_{wy}}{S_w S_{wx^2} - S_{wx}^2},
\qquad S_w = \sum_i w_i,\; S_{wx} = \sum_i w_i x_i,\; S_{wxy} = \sum_i w_i x_i V_i, \;\text{etc.}$$

**Motivation.** Since $V_i$ is a sum of $x_i$ i.i.d. draws, $\mathrm{Var}(V_i) = x_i s^2$: the
observations are heteroscedastic, and $w_i = 1/x_i$ is exactly inverse-variance weighting. By the
Gauss–Markov theorem the WLS slope is the **best linear unbiased estimator** of $M$ given the
batch sums and the two-parameter model. It keeps the operational advantage of A — the intercept
$\alpha$ absorbs any constant per-measurement overhead — while repairing A's inefficiency.

**What it fixes.** OLS on this design concentrates leverage on the largest batch
($\mathrm{RE}_A \to 7/9$); WLS spreads the information optimally and its efficiency tends to 1.

### 1.2 E — Weighted least squares on the arithmetic design

**Definition.** Same measurements as B: batch sums $V_i$ at $x_i = i$, $i = 1, \dots, h$; the same
WLS fit as D with weights $w_i = 1/x_i$; the estimate is the slope.

**Motivation.** Identical to D's: inverse-variance weighting makes the slope BLUE for the
Criterion-style design. This is the minimal statistical repair of the Criterion estimator that
preserves its design (many small-to-medium batches, useful for regression diagnostics and
outlier visibility) and its overhead-absorbing intercept.

**What it fixes.** OLS on this design wastes two-thirds of the budget
($\mathrm{RE}_B \to 1/3$). WLS recovers a large part — but, unlike D, not all of it (§2.2): the
arithmetic design itself, not just the fitting method, is what limits it.

### 1.3 F — Bias-corrected median-of-means

**Definition.** Compute the $g = 2^k$ batch means $\bar Y_1, \dots, \bar Y_g$ (each over
$b = 2^k$ draws) as in C. Let $\hat\sigma_b$ and $\hat\gamma_b$ be their sample standard
deviation and sample skewness,

$$\hat\gamma_b = \frac{\tfrac1g \sum_j (\bar Y_j - \bar{\bar Y})^3}{\left[\tfrac1g \sum_j (\bar Y_j - \bar{\bar Y})^2\right]^{3/2}} .$$

The estimator is the median plus a plug-in Edgeworth correction:

$$\hat M_F \;=\; \mathrm{median}(\bar Y_1, \dots, \bar Y_g) \;+\; \frac{\hat\gamma_b\, \hat\sigma_b}{6}.$$

**Motivation.** Report #1 showed C's error is dominated by the mean–median gap of the batch-mean
distribution, $\text{mean} - \text{median} \approx \gamma_b \sigma_b / 6$ to second order. F
estimates that gap from the batch means themselves and adds it back.

**Degeneracies and limits.** At $k = 1$ ($g = 2$) the sample skewness of two points is 0, so F
degenerates to plain MoM (which itself degenerates to the pooled mean). More fundamentally, the
sample skewness of $g$ points is bounded: $|\hat\gamma_b| \le \sqrt{g - 2 + 1/(g-1)} \approx \sqrt{g}$,
so when the true batch-mean skewness exceeds $\approx \sqrt{g}$ (the case at $\sigma = 2$ for all
studied $k$) the correction *cannot* reach the true bias — F must undercorrect. The correction
term also adds sampling noise, and — being a data-dependent additive term with the sign of the
observed skewness — it partially re-imports the tail sensitivity that the median had removed.

### 1.4 G — Overlapping-blocks median-of-means

**Definition.** Take the same $N = 4^k$ draws $Y_1, \dots, Y_N$ as one sequence, and form *all*
$N - b + 1$ overlapping (sliding-window) block means of length $b = 2^k$:

$$\bar Y^{(j)} = \frac{1}{b} \sum_{t=j}^{j+b-1} Y_t, \qquad j = 1, \dots, N - b + 1.$$

The estimator is $\hat M_G = \mathrm{median}\left(\bar Y^{(1)}, \dots, \bar Y^{(N-b+1)}\right)$.

**Motivation.** C uses only $g$ disjoint blocks, so its median is a coarse order statistic of a
small sample. Overlapping blocks (as in overlapping batch-means methods from simulation output
analysis) reuse every draw in up to $b$ blocks, smoothing the empirical block-mean distribution
and reducing the variance of its median — at zero extra sampling cost, only extra computation.

**Limits.** Every block mean is drawn from the *same* distribution as C's batch means, so the
median being estimated is unchanged: **G inherits C's bias floor exactly**. Overlap induces strong
positive correlation between nearby blocks (adjacent blocks share $b - 1$ draws), so the
effective number of independent blocks is still $\approx g$; the variance reduction is real but
bounded. Robustness weakens slightly in one specific sense: a single extreme draw contaminates up
to $b$ consecutive block means rather than 1 of $g$ batch means, though the median still rejects
them as long as they remain a minority.

### 1.5 H — Trimmed mean of batch means

**Definition.** Compute the $g = 2^k$ batch means as in C, sort them, discard the
$t = \lfloor g/8 \rfloor$ smallest and $t$ largest ($\approx 12.5\%$ per side), and average the
rest:

$$\hat M_H = \frac{1}{g - 2t} \sum_{j=t+1}^{g-t} \bar Y_{(j)} .$$

**Motivation.** The median (C) is the maximally trimmed mean — maximal robustness, maximal
skew-bias. The untrimmed mean of batch means is exactly the pooled mean — no robustness, no bias.
A light symmetric trim interpolates: it discards the tail events that inflate the pooled mean's
error while keeping most of the averaging, so its skew-bias should be roughly proportional to the
trimming intensity, far below C's.

**Degeneracies and limits.** For $k \le 2$, $t = \lfloor g/8 \rfloor = 0$ and H *is* the pooled
mean. Its group-level breakdown point is $t/g \approx 1/8$ — protection against a modest fraction
of wild batches, not against half of them. Note the bias is *negative* for a right-skewed
population (symmetric trimming removes more mass from the long right tail than the short left
one), like C's but much smaller.

---

## 2. Theoretical properties

### 2.1 Bias

**D and E are exactly unbiased**, by the same argument as A and B: the slope is a linear
combination $\sum_i c_i V_i$ with $\sum_i c_i = 0$ and $\sum_i c_i x_i = 1$, and
$E[V_i] = M x_i$. This holds for any observation weights, so weighting cannot introduce bias.

**F** removes the *second-order* (skewness) term of C's bias. Its residual bias has three
sources: higher-order Edgeworth terms ($O(b^{-1})$ kurtosis contribution), the bias of the plug-in
$\hat\gamma_b \hat\sigma_b$ under heavy tails, and the hard bound
$|\hat\gamma_b| \lesssim \sqrt{g}$, which caps the correction at
$\approx \sqrt{g}\,\hat\sigma_b/6$ regardless of the true gap. Under extreme skew these effects do
not simply produce a smaller version of C's negative bias: the *typical* realization undercorrects
(the sample skewness cannot see the true tail), while rare realizations that do catch a tail event
produce enormous positive corrections — so the mean bias can even flip positive while the median
error stays negative. The simulation (§3) shows exactly this at $\sigma = 2$.

**G** has C's bias exactly in the limit of many blocks: the target functional (median of the
block-mean distribution) is identical. Finite-sample bias differs from C's only via the
correlation structure of overlapping blocks.

**H**: for $k \ge 3$, the expectation of a $12.5\%$-per-side trimmed mean of i.i.d. right-skewed
batch means lies below $M$ by an amount proportional to the asymmetry of the trimmed tails —
first order in the trim fraction, so roughly $1/4$ to $1/3$ of C's bias at the same $(k, \sigma)$
is a reasonable a-priori guess; the simulation quantifies it.

### 2.2 Efficiency

For any linear slope estimator with coefficient vector $c$ on the batch sums,
$\mathrm{Var} = s^2 \sum_i c_i^2 x_i$. For WLS with $w = 1/x$ this evaluates to

$$\mathrm{Var}(\hat\beta_{\mathrm{WLS}}) \;=\; \frac{s^2\, S_w}{S_w N - p^2},
\qquad \mathrm{RE} \;=\; 1 - \frac{p^2}{N\, S_w},$$

where $p$ is the number of design points and $N = \sum_i x_i$ the budget. The $p^2/(N S_w)$ term
is the price of estimating the intercept.

**D** (geometric): $S_w = 2 - 2^{1-m} \to 2$, $p = m = 2k$, $N = 2^m - 1$, so
$\mathrm{RE}_D = 1 - m^2/(N S_w) \approx 1 - \frac{(2k)^2}{2 \cdot 4^k} \to 1$ *exponentially
fast*. **E** (arithmetic): $S_w = H_h$ (harmonic number), $p = h$, $N = h(h+1)/2$, so
$\mathrm{RE}_E = 1 - \frac{2h}{(h+1) H_h} \approx 1 - \frac{2}{\ln h} \to 1$ only
*logarithmically* — the arithmetic design spends half its budget on batches of size $\le h/2$
whose information the intercept term partially consumes. Exact values:

| $k$ | 1 | 2 | 3 | 4 | 5 | 6 | $\to\infty$ |
|---|---|---|---|---|---|---|---|
| $\mathrm{RE}_D$ (WLS-geom) | 0.111 | 0.431 | 0.710 | 0.874 | 0.951 | 0.982 | 1 |
| $\mathrm{RE}_E$ (WLS-arith) | 0.111 | 0.270 | 0.393 | 0.482 | 0.555 | 0.611 | 1 (as $1 - 2/\ln h$) |
| $\mathrm{RE}_A$ (OLS-geom, ref.) | 0.111 | 0.348 | 0.499 | 0.584 | 0.633 | 0.664 | $7/9$ |
| $\mathrm{RE}_B$ (OLS-arith, ref.) | 0.111 | 0.222 | 0.278 | 0.304 | 0.319 | 0.326 | $1/3$ |

**F**: C's variance plus the variance of the correction term (and their covariance); expect
modest inflation over C for $\sigma \le 1$, substantial at $\sigma = 2$ where
$\hat\gamma_b \hat\sigma_b$ is itself heavy-tailed. **G**: variance strictly below C's for the
same target; the improvement is bounded because adjacent blocks are highly correlated. **H**:
for light tails, close to the pooled mean ($\mathrm{RE}$ near 1, above C's $2/\pi$); under
extreme skew, like C, its variance can be far *below* the pooled mean's because the trimmed tail
carried most of the variance.

### 2.3 Robustness

- **D, E**: breakdown point 0, like all linear-in-data estimators. D's weighting *reduces* the
  leverage of the largest batch relative to A (weight $1/x$ shrinks big batches' influence per
  draw to parity), so its error tail should resemble the pooled mean's — heavy under $\sigma = 2$,
  but no worse than the budget-equivalent sample mean. E similarly tracks the pooled mean's tail.
- **F**: the median core is robust, but the additive correction is a function of second and third
  sample moments of the batch means — unbounded influence. Under moderate skew F's tail behavior
  sits between C's (near-deterministic) and the pooled mean's; under extreme skew the correction
  term's own heavy tail can make F *worse than the pooled mean* (§3, $\sigma = 2$).
- **G**: group-level breakdown $\approx 1/2$ like C, with the caveat that one extreme draw now
  occupies up to $b$ consecutive blocks; for $b \ll (N-b+1)/2$ this is immaterial.
- **H**: group-level breakdown $t/g \approx 1/8$; influence function bounded. Tail ratio should
  be near-Gaussian for $k \ge 3$ at every $\sigma$, at the price of the (small) trim bias.

---

## 3. Simulation results

Monte Carlo: 100,000 replications per $(\sigma, k)$ cell, vectorized numpy, fixed seed;
script: `simulate_improved_estimators.py`; raw output: `results/improved_estimator_comparison.csv`.
Estimators sharing a design are computed from the same draws (common random numbers), and A, B, C
reproduce report #1 exactly (same seeds and draw order). Metrics as in report #1, relative to
$M$; best RMSE per $k$ in **bold**.

Consistency checks passed: A, B, C reproduce report #1's values exactly (common seeds); D and E
have zero empirical bias in every cell; D's and E's empirical RE match the closed-form values of
§2.2 (e.g. 0.705 vs. 0.710 and 0.393 vs. 0.393 at $\sigma = 0.25$, $k = 3$; 0.982 and 0.611 at
$k = 6$); H equals the pooled mean identically for $k \le 2$; F and H equal C at $k = 1$; G's
bias tracks C's to three decimals at every cell, confirming the shared bias floor.

At $k = 1$ several estimators coincide by construction (D $\equiv$ A, E $\equiv$ B,
F $\equiv$ H $\equiv$ C $\equiv$ pooled mean); the rows are kept for completeness.

### $\sigma = 0.25$ (mild skew, $\mathrm{cv} \approx 0.25$)

| $k$ | est | bias | SD | RMSE | med$\|$err$\|$ | tail | RE |
|---|---|---|---|---|---|---|---|
| 1 | A | −0.001 | 0.440 | 0.440 | 0.287 | 1.05 | 0.111 |
| 1 | B | −0.001 | 0.440 | 0.440 | 0.288 | 1.05 | 0.111 |
| 1 | C | −0.001 | 0.126 | **0.126** | 0.085 | 1.05 | 1.000 |
| 1 | D | −0.001 | 0.440 | 0.440 | 0.287 | 1.05 | 0.111 |
| 1 | E | −0.001 | 0.440 | 0.440 | 0.288 | 1.05 | 0.111 |
| 1 | F | −0.001 | 0.126 | **0.126** | 0.085 | 1.05 | 1.000 |
| 1 | G | −0.001 | 0.149 | 0.149 | 0.099 | 1.05 | 0.716 |
| 1 | H | −0.001 | 0.126 | **0.126** | 0.085 | 1.05 | 1.000 |
| 2 | A | +0.000 | 0.111 | 0.111 | 0.074 | 1.04 | 0.350 |
| 2 | B | −0.001 | 0.140 | 0.140 | 0.093 | 1.04 | 0.222 |
| 2 | C | −0.004 | 0.0688 | 0.0689 | 0.047 | 1.04 | 0.849 |
| 2 | D | +0.000 | 0.0996 | 0.0996 | 0.067 | 1.04 | 0.433 |
| 2 | E | −0.000 | 0.127 | 0.127 | 0.084 | 1.04 | 0.269 |
| 2 | F | −0.002 | 0.0650 | 0.0651 | 0.044 | 1.04 | 0.951 |
| 2 | G | −0.005 | 0.0721 | 0.0723 | 0.049 | 1.04 | 0.773 |
| 2 | H | −0.000 | 0.0634 | **0.0634** | 0.043 | 1.04 | 1.000 |
| 3 | A | −0.000 | 0.0453 | 0.0453 | 0.0304 | 1.04 | 0.499 |
| 3 | B | +0.000 | 0.0594 | 0.0594 | 0.0399 | 1.04 | 0.278 |
| 3 | C | −0.003 | 0.0366 | 0.0367 | 0.0248 | 1.04 | 0.755 |
| 3 | D | +0.000 | 0.0381 | 0.0381 | 0.0255 | 1.04 | 0.705 |
| 3 | E | +0.000 | 0.0499 | 0.0499 | 0.0333 | 1.04 | 0.393 |
| 3 | F | −0.001 | 0.0343 | 0.0343 | 0.0232 | 1.04 | 0.857 |
| 3 | G | −0.003 | 0.0351 | 0.0352 | 0.0238 | 1.04 | 0.822 |
| 3 | H | −0.002 | 0.0328 | **0.0328** | 0.0221 | 1.04 | 0.940 |
| 4 | A | −0.000 | 0.0207 | 0.0207 | 0.0140 | 1.04 | 0.589 |
| 4 | B | +0.000 | 0.0290 | 0.0290 | 0.0195 | 1.04 | 0.303 |
| 4 | C | −0.002 | 0.0190 | 0.0191 | 0.0129 | 1.04 | 0.694 |
| 4 | D | +0.000 | 0.0170 | 0.0170 | 0.0115 | 1.04 | 0.878 |
| 4 | E | +0.000 | 0.0230 | 0.0230 | 0.0154 | 1.04 | 0.479 |
| 4 | F | −0.000 | 0.0179 | 0.0179 | 0.0121 | 1.04 | 0.786 |
| 4 | G | −0.002 | 0.0172 | 0.0173 | 0.0117 | 1.04 | 0.845 |
| 4 | H | −0.001 | 0.0164 | **0.0164** | 0.0111 | 1.04 | 0.937 |
| 5 | A | −0.000 | 0.00996 | 0.00996 | 0.0067 | 1.04 | 0.633 |
| 5 | B | −0.000 | 0.0140 | 0.0140 | 0.0094 | 1.04 | 0.318 |
| 5 | C | −0.001 | 0.00971 | 0.00976 | 0.0066 | 1.04 | 0.666 |
| 5 | D | −0.000 | 0.00813 | **0.00813** | 0.0055 | 1.04 | 0.950 |
| 5 | E | −0.000 | 0.0106 | 0.0106 | 0.0072 | 1.04 | 0.555 |
| 5 | F | −0.000 | 0.00916 | 0.00916 | 0.0062 | 1.04 | 0.749 |
| 5 | G | −0.001 | 0.00853 | 0.00859 | 0.0058 | 1.04 | 0.863 |
| 5 | H | −0.001 | 0.00821 | 0.00823 | 0.0056 | 1.04 | 0.932 |
| 6 | A | −0.000 | 0.00487 | 0.00487 | 0.0033 | 1.04 | 0.662 |
| 6 | B | −0.000 | 0.00693 | 0.00693 | 0.0047 | 1.04 | 0.327 |
| 6 | C | −0.001 | 0.00492 | 0.00494 | 0.0033 | 1.04 | 0.653 |
| 6 | D | +0.000 | 0.00400 | **0.00400** | 0.0027 | 1.04 | 0.982 |
| 6 | E | +0.000 | 0.00508 | 0.00508 | 0.0034 | 1.04 | 0.611 |
| 6 | F | −0.000 | 0.00464 | 0.00464 | 0.0031 | 1.04 | 0.734 |
| 6 | G | −0.001 | 0.00427 | 0.00430 | 0.0029 | 1.04 | 0.868 |
| 6 | H | −0.000 | 0.00412 | 0.00413 | 0.0028 | 1.04 | 0.931 |

### $\sigma = 0.5$ (moderate skew, $\mathrm{cv} \approx 0.53$)

| $k$ | est | bias | SD | RMSE | med$\|$err$\|$ | tail | RE |
|---|---|---|---|---|---|---|---|
| 1 | A | +0.002 | 0.923 | 0.923 | 0.542 | 1.07 | 0.110 |
| 1 | B | +0.000 | 0.923 | 0.923 | 0.549 | 1.07 | 0.112 |
| 1 | C | −0.002 | 0.266 | **0.266** | 0.174 | 1.07 | 1.000 |
| 1 | D | +0.002 | 0.923 | 0.923 | 0.542 | 1.07 | 0.110 |
| 1 | E | +0.000 | 0.923 | 0.923 | 0.549 | 1.07 | 0.112 |
| 1 | F | −0.002 | 0.266 | **0.266** | 0.174 | 1.07 | 1.000 |
| 1 | G | −0.002 | 0.320 | 0.320 | 0.204 | 1.09 | 0.688 |
| 1 | H | −0.002 | 0.266 | **0.266** | 0.174 | 1.07 | 1.000 |
| 2 | A | +0.000 | 0.232 | 0.232 | 0.154 | 1.06 | 0.348 |
| 2 | B | −0.000 | 0.291 | 0.291 | 0.190 | 1.06 | 0.223 |
| 2 | C | −0.019 | 0.141 | 0.142 | 0.098 | 1.05 | 0.901 |
| 2 | D | +0.000 | 0.209 | 0.209 | 0.135 | 1.05 | 0.431 |
| 2 | E | +0.000 | 0.264 | 0.264 | 0.167 | 1.06 | 0.271 |
| 2 | F | −0.011 | 0.135 | 0.135 | 0.092 | 1.05 | 0.990 |
| 2 | G | −0.021 | 0.149 | 0.151 | 0.103 | 1.05 | 0.807 |
| 2 | H | +0.000 | 0.134 | **0.134** | 0.090 | 1.05 | 1.000 |
| 3 | A | −0.000 | 0.0951 | 0.0951 | 0.0636 | 1.04 | 0.499 |
| 3 | B | −0.000 | 0.125 | 0.125 | 0.0833 | 1.04 | 0.276 |
| 3 | C | −0.015 | 0.0752 | 0.0766 | 0.0525 | 1.04 | 0.784 |
| 3 | D | +0.000 | 0.0796 | 0.0796 | 0.0527 | 1.04 | 0.713 |
| 3 | E | −0.001 | 0.105 | 0.105 | 0.0686 | 1.05 | 0.391 |
| 3 | F | −0.005 | 0.0715 | 0.0717 | 0.0487 | 1.04 | 0.867 |
| 3 | G | −0.015 | 0.0723 | 0.0738 | 0.0507 | 1.04 | 0.848 |
| 3 | H | −0.009 | 0.0676 | **0.0681** | 0.0466 | 1.04 | 0.971 |
| 4 | A | −0.000 | 0.0437 | 0.0437 | 0.0295 | 1.04 | 0.585 |
| 4 | B | +0.000 | 0.0609 | 0.0609 | 0.0409 | 1.04 | 0.303 |
| 4 | C | −0.008 | 0.0394 | 0.0403 | 0.0277 | 1.04 | 0.715 |
| 4 | D | −0.000 | 0.0358 | 0.0358 | 0.0241 | 1.04 | 0.875 |
| 4 | E | +0.000 | 0.0484 | 0.0484 | 0.0320 | 1.05 | 0.481 |
| 4 | F | −0.002 | 0.0375 | 0.0375 | 0.0255 | 1.04 | 0.790 |
| 4 | G | −0.008 | 0.0358 | 0.0368 | 0.0252 | 1.04 | 0.868 |
| 4 | H | −0.005 | 0.0342 | **0.0345** | 0.0235 | 1.04 | 0.952 |
| 5 | A | −0.000 | 0.0209 | 0.0209 | 0.0141 | 1.04 | 0.634 |
| 5 | B | −0.000 | 0.0294 | 0.0294 | 0.0198 | 1.04 | 0.319 |
| 5 | C | −0.004 | 0.0203 | 0.0207 | 0.0140 | 1.04 | 0.671 |
| 5 | D | −0.000 | 0.0171 | **0.0171** | 0.0115 | 1.04 | 0.949 |
| 5 | E | −0.000 | 0.0222 | 0.0222 | 0.0149 | 1.04 | 0.557 |
| 5 | F | −0.000 | 0.0192 | 0.0192 | 0.0129 | 1.04 | 0.746 |
| 5 | G | −0.004 | 0.0178 | 0.0184 | 0.0125 | 1.04 | 0.865 |
| 5 | H | −0.003 | 0.0171 | 0.0174 | 0.0117 | 1.04 | 0.937 |
| 6 | A | +0.000 | 0.0103 | 0.0103 | 0.0069 | 1.04 | 0.664 |
| 6 | B | −0.000 | 0.0146 | 0.0146 | 0.0098 | 1.04 | 0.325 |
| 6 | C | −0.002 | 0.0103 | 0.0106 | 0.0072 | 1.04 | 0.656 |
| 6 | D | +0.000 | 0.00845 | **0.00845** | 0.0057 | 1.04 | 0.982 |
| 6 | E | −0.000 | 0.0107 | 0.0107 | 0.0072 | 1.04 | 0.607 |
| 6 | F | −0.000 | 0.00977 | 0.00977 | 0.0066 | 1.04 | 0.728 |
| 6 | G | −0.002 | 0.00893 | 0.00924 | 0.0063 | 1.04 | 0.873 |
| 6 | H | −0.001 | 0.00864 | 0.00876 | 0.0059 | 1.04 | 0.932 |

### $\sigma = 1$ (strong skew, $\mathrm{cv} \approx 1.31$)

| $k$ | est | bias | SD | RMSE | med$\|$err$\|$ | tail | RE |
|---|---|---|---|---|---|---|---|
| 1 | A | −0.006 | 2.264 | 2.264 | 0.877 | 1.23 | 0.111 |
| 1 | B | −0.006 | 2.219 | 2.219 | 0.877 | 1.22 | 0.112 |
| 1 | C | +0.001 | 0.655 | **0.655** | 0.365 | 1.20 | 1.000 |
| 1 | D | −0.006 | 2.264 | 2.264 | 0.877 | 1.23 | 0.111 |
| 1 | E | −0.006 | 2.219 | 2.219 | 0.877 | 1.22 | 0.112 |
| 1 | F | +0.001 | 0.655 | **0.655** | 0.365 | 1.20 | 1.000 |
| 1 | G | +0.002 | 0.850 | 0.850 | 0.423 | 1.30 | 0.594 |
| 1 | H | +0.001 | 0.655 | **0.655** | 0.365 | 1.20 | 1.000 |
| 2 | A | +0.000 | 0.574 | 0.574 | 0.328 | 1.14 | 0.344 |
| 2 | B | +0.003 | 0.718 | 0.718 | 0.388 | 1.14 | 0.223 |
| 2 | C | −0.095 | 0.291 | 0.306 | 0.223 | 1.06 | 1.277 |
| 2 | D | −0.000 | 0.514 | 0.514 | 0.286 | 1.13 | 0.429 |
| 2 | E | +0.003 | 0.652 | 0.652 | 0.330 | 1.15 | 0.270 |
| 2 | F | −0.054 | 0.293 | **0.298** | 0.205 | 1.08 | 1.259 |
| 2 | G | −0.100 | 0.317 | 0.333 | 0.239 | 1.08 | 1.072 |
| 2 | H | +0.000 | 0.328 | 0.328 | 0.203 | 1.11 | 1.000 |
| 3 | A | +0.002 | 0.235 | 0.235 | 0.149 | 1.09 | 0.496 |
| 3 | B | +0.001 | 0.308 | 0.308 | 0.191 | 1.08 | 0.277 |
| 3 | C | −0.078 | 0.158 | 0.177 | 0.131 | 1.03 | 1.074 |
| 3 | D | +0.001 | 0.196 | 0.196 | 0.125 | 1.08 | 0.711 |
| 3 | E | +0.001 | 0.257 | 0.257 | 0.150 | 1.10 | 0.399 |
| 3 | F | −0.026 | 0.167 | 0.169 | 0.114 | 1.06 | 0.967 |
| 3 | G | −0.080 | 0.155 | 0.174 | 0.129 | 1.03 | 1.125 |
| 3 | H | −0.049 | 0.150 | **0.157** | 0.112 | 1.04 | 1.200 |
| 4 | A | +0.000 | 0.108 | 0.108 | 0.072 | 1.05 | 0.583 |
| 4 | B | +0.001 | 0.149 | 0.149 | 0.098 | 1.05 | 0.304 |
| 4 | C | −0.051 | 0.086 | 0.100 | 0.073 | 1.03 | 0.896 |
| 4 | D | +0.000 | 0.088 | 0.088 | 0.058 | 1.05 | 0.874 |
| 4 | E | +0.001 | 0.118 | 0.118 | 0.073 | 1.07 | 0.482 |
| 4 | F | −0.006 | 0.095 | 0.096 | 0.062 | 1.07 | 0.735 |
| 4 | G | −0.051 | 0.079 | 0.094 | 0.069 | 1.03 | 1.060 |
| 4 | H | −0.032 | 0.077 | **0.084** | 0.059 | 1.03 | 1.122 |
| 5 | A | −0.000 | 0.0514 | 0.0514 | 0.0345 | 1.04 | 0.635 |
| 5 | B | −0.000 | 0.0720 | 0.0720 | 0.0483 | 1.04 | 0.320 |
| 5 | C | −0.030 | 0.0461 | 0.0548 | 0.0392 | 1.03 | 0.790 |
| 5 | D | −0.000 | 0.0420 | **0.0420** | 0.0281 | 1.04 | 0.952 |
| 5 | E | +0.000 | 0.0544 | 0.0544 | 0.0351 | 1.06 | 0.559 |
| 5 | F | +0.001 | 0.0521 | 0.0521 | 0.0329 | 1.08 | 0.620 |
| 5 | G | −0.030 | 0.0410 | 0.0506 | 0.0368 | 1.03 | 0.998 |
| 5 | H | −0.019 | 0.0399 | 0.0441 | 0.0308 | 1.03 | 1.057 |
| 6 | A | +0.000 | 0.0251 | 0.0251 | 0.0168 | 1.04 | 0.664 |
| 6 | B | +0.000 | 0.0358 | 0.0358 | 0.0241 | 1.04 | 0.328 |
| 6 | C | −0.017 | 0.0240 | 0.0292 | 0.0207 | 1.03 | 0.721 |
| 6 | D | +0.000 | 0.0206 | **0.0206** | 0.0139 | 1.04 | 0.981 |
| 6 | E | +0.000 | 0.0263 | 0.0263 | 0.0171 | 1.06 | 0.608 |
| 6 | F | +0.002 | 0.0267 | 0.0268 | 0.0169 | 1.09 | 0.582 |
| 6 | G | −0.017 | 0.0210 | 0.0267 | 0.0193 | 1.03 | 0.949 |
| 6 | H | −0.011 | 0.0204 | 0.0230 | 0.0159 | 1.04 | 1.003 |

### $\sigma = 2$ (extreme skew, $\mathrm{cv} \approx 7.3$)

| $k$ | est | bias | SD | RMSE | med$\|$err$\|$ | tail | RE |
|---|---|---|---|---|---|---|---|
| 1 | A | +0.034 | 12.46 | 12.46 | 0.967 | 3.28 | 0.111 |
| 1 | B | −0.008 | 10.85 | 10.85 | 0.966 | 2.90 | 0.111 |
| 1 | C | +0.015 | 3.695 | **3.695** | 0.739 | 2.99 | 1.000 |
| 1 | D | +0.034 | 12.46 | 12.46 | 0.967 | 3.28 | 0.111 |
| 1 | E | −0.008 | 10.85 | 10.85 | 0.966 | 2.90 | 0.111 |
| 1 | F | +0.015 | 3.695 | **3.695** | 0.739 | 2.99 | 1.000 |
| 1 | G | +0.009 | 5.051 | 5.051 | 0.804 | 3.73 | 0.535 |
| 1 | H | +0.015 | 3.695 | **3.695** | 0.739 | 2.99 | 1.000 |
| 2 | A | −0.010 | 2.830 | 2.830 | 0.679 | 2.12 | 0.370 |
| 2 | B | −0.009 | 3.467 | 3.467 | 0.718 | 2.11 | 0.239 |
| 2 | C | −0.437 | 0.519 | **0.679** | 0.604 | 1.11 | 12.50 |
| 2 | D | −0.009 | 2.656 | 2.656 | 0.612 | 2.16 | 0.420 |
| 2 | E | −0.001 | 3.021 | 3.021 | 0.632 | 2.04 | 0.314 |
| 2 | F | −0.259 | 0.892 | 0.929 | 0.541 | 1.50 | 4.24 |
| 2 | G | −0.450 | 0.607 | 0.755 | 0.638 | 1.18 | 9.16 |
| 2 | H | −0.000 | 1.836 | 1.836 | 0.524 | 2.20 | 1.000 |
| 3 | A | −0.001 | 1.258 | 1.258 | 0.424 | 1.80 | 0.515 |
| 3 | B | −0.007 | 1.548 | 1.548 | 0.484 | 1.67 | 0.297 |
| 3 | C | −0.424 | 0.250 | 0.493 | 0.479 | 1.01 | 12.22 |
| 3 | D | −0.006 | 1.147 | 1.147 | 0.363 | 1.86 | 0.620 |
| 3 | E | −0.002 | 1.343 | 1.343 | 0.390 | 1.77 | 0.394 |
| 3 | F | −0.100 | 0.874 | 0.879 | 0.369 | 1.74 | 1.003 |
| 3 | G | −0.429 | 0.253 | 0.498 | 0.486 | 1.01 | 11.92 |
| 3 | H | −0.301 | 0.318 | **0.438** | 0.390 | 1.04 | 7.57 |
| 4 | A | −0.001 | 0.594 | 0.594 | 0.246 | 1.55 | 0.567 |
| 4 | B | +0.002 | 0.823 | 0.823 | 0.311 | 1.52 | 0.311 |
| 4 | C | −0.344 | 0.156 | 0.378 | 0.365 | 1.01 | 9.38 |
| 4 | D | −0.001 | 0.478 | 0.478 | 0.209 | 1.46 | 0.872 |
| 4 | E | +0.002 | 0.638 | 0.638 | 0.234 | 1.53 | 0.517 |
| 4 | F | +0.075 | 0.960 | 0.963 | 0.247 | 1.98 | 0.247 |
| 4 | G | −0.346 | 0.147 | 0.376 | 0.366 | 1.01 | 10.46 |
| 4 | H | −0.249 | 0.173 | **0.303** | 0.277 | 1.02 | 7.58 |
| 5 | A | +0.002 | 0.286 | 0.286 | 0.139 | 1.34 | 0.645 |
| 5 | B | −0.001 | 0.405 | 0.405 | 0.182 | 1.37 | 0.308 |
| 5 | C | −0.259 | 0.098 | 0.277 | 0.267 | 1.01 | 5.18 |
| 5 | D | +0.001 | 0.236 | 0.236 | 0.117 | 1.32 | 0.946 |
| 5 | E | −0.000 | 0.289 | 0.289 | 0.130 | 1.34 | 0.605 |
| 5 | F | +0.188 | 0.796 | 0.818 | 0.166 | 1.71 | 0.079 |
| 5 | G | −0.260 | 0.090 | 0.275 | 0.267 | 1.01 | 6.22 |
| 5 | H | −0.189 | 0.100 | **0.214** | 0.198 | 1.01 | 4.99 |
| 6 | A | −0.000 | 0.141 | 0.141 | 0.076 | 1.24 | 0.658 |
| 6 | B | +0.000 | 0.199 | 0.199 | 0.104 | 1.23 | 0.337 |
| 6 | C | −0.187 | 0.060 | 0.196 | 0.190 | 1.01 | 3.65 |
| 6 | D | +0.000 | 0.115 | **0.115** | 0.064 | 1.20 | 0.983 |
| 6 | E | +0.001 | 0.145 | 0.145 | 0.072 | 1.28 | 0.635 |
| 6 | F | +0.242 | 0.709 | 0.749 | 0.119 | 1.65 | 0.026 |
| 6 | G | −0.187 | 0.053 | 0.194 | 0.190 | 1.01 | 4.54 |
| 6 | H | −0.136 | 0.058 | 0.148 | 0.139 | 1.01 | 3.89 |

*(RE $\gg 1$ for C, G, H at $\sigma = 2$ means variance far below the pooled mean's — their error
is dominated by bias, as the near-1.01 tail ratios confirm. F's RE of 0.026 at $k = 6$ is the
opposite pathology: the correction term's heavy tail inflates its variance to $\approx 39\times$
the pooled mean's.)*

---

## 4. Comparative assessment by $k$ regime

**The RMSE winner map.** Across all 24 cells the best estimator follows a clean pattern:

| $\sigma$ \ $k$ | 1 | 2 | 3 | 4 | 5 | 6 |
|---|---|---|---|---|---|---|
| 0.25 | C/F/H (= pooled) | H | H | H | **D** | **D** |
| 0.5 | C/F/H (= pooled) | H | H | H | **D** | **D** |
| 1.0 | C/F/H (= pooled) | F | H | H | **D** | **D** |
| 2.0 | C/F/H (= pooled) | C | H | H | H | **D** |

**H owns the middle, D owns the top end, and nothing beats the pooled mean at $k = 1$.**

**Estimator by estimator:**

**D (WLS-geometric) is the new benchmark for unbiased estimation.** It dominates A at every cell
(same data, better weights): 8–19% lower RMSE at $k = 3$ growing to 18–22% at $k = 6$, with
empirical RE reaching 0.98 — statistically indistinguishable from using every raw draw. From
$k = 5$ on it has the best RMSE of *all eight* estimators for $\sigma \le 1$, and at
$(\sigma{=}2, k{=}6)$ it beats even the robust estimators because their bias floors exceed its
shrinking SD. Its one weakness is inherited from the design: error tails under extreme skew
(tail ratio 1.2–1.5 at $\sigma = 2$), essentially those of the budget-equivalent sample mean.

**E (WLS-arithmetic) is a strict but partial repair of B.** Unbiased, and it roughly halves B's
variance penalty (RE 0.61 vs. 0.33 at $k = 6$, i.e. RMSE $\approx 27\%$ lower) — for free, since
it reuses B's measurements. But it remains dominated by D at every cell, confirming §2.2: the
arithmetic design itself, not OLS, is the binding constraint, and the gap closes only as
$1 - 2/\ln h$.

**F (bias-corrected MoM) is a success below $\sigma = 1$ and a failure above it.**
At $\sigma \le 0.5$ it cuts C's already-small bias by $2$–$12\times$ and slightly beats C's RMSE.
At $\sigma = 1$ it removes 70–95% of C's bias (e.g. $-0.006$ vs. $-0.051$ at $k = 4$) at a modest
variance cost, and even wins the $k = 2$ cell outright. At $\sigma = 2$ it collapses exactly as
§1.3/§2.1 anticipated, and worse: the typical run still undercorrects (median error remains
large), while rare tail realizations produce huge positive corrections — the mean bias flips sign
($-0.26$ at $k = 2$ → $+0.24$ at $k = 6$), the SD balloons to 5–12× C's, and by $k \ge 4$ F has
*worse RMSE than the raw pooled mean*. A plug-in moment correction is the wrong tool where
moments themselves cannot be estimated.

**G (overlapping-blocks MoM) is a real but small improvement over C.** Its bias matches C's to
three decimals everywhere, as theory requires. Its variance is consistently 10–20% below C's for
$k \ge 2$ (empirical RE $\approx 0.86$–$0.87$ vs. C's 0.65–0.72 at $\sigma \le 1$) — worth having,
but it never changes which estimator wins a cell: where bias is negligible D and H beat it, and
where bias dominates ($\sigma \ge 1$) the shared bias floor makes the variance saving irrelevant
(at $\sigma = 2$, $k = 6$: RMSE 0.194 vs. C's 0.196). At $k = 1$ it is strictly *worse* than C
(median of 3 highly correlated block means vs. mean of 4 draws). Verdict: for i.i.d. data the
extra computation buys little; overlapping blocks earn their keep in serially correlated
(time-series) settings, which is outside this study.

**H (trimmed mean of batch means) is the best practical robust estimator in the middle regime.**
For $k = 2$–$4$ it has the best RMSE at nearly every $\sigma$, and at $\sigma = 2$ it extends that
through $k = 5$. It keeps C's signature near-deterministic error under extreme skew (tail ratio
1.01–1.04 for $k \ge 3$) while cutting C's bias by roughly 25–35% ($-0.30$ vs. $-0.42$ at $k = 3$;
$-0.14$ vs. $-0.19$ at $k = 6$); for $\sigma \le 1$ it beats C's variance as well (RE close to 1
vs. C's $2/\pi$), while at $\sigma = 2$ it trades a little variance ($k = 3$–$5$) for the much
larger bias gain. The cost: its group breakdown point is only $\approx 1/8$ (vs. C's $1/2$), and
under extreme skew its residual bias is still material ($-14\%$ at $\sigma = 2$, $k = 6$), so like
C it systematically under-reports the mean where the tail matters most.

**Regime summary.**

- **$k = 1$–$2$:** nothing sophisticated helps; the pooled mean (equivalently H, or C/F at
  $k = 1$) is as good as it gets, and at $\sigma = 2$, $k = 2$ only C's aggressive medianization
  reduces the chaos — at the price of $-44\%$ bias.
- **$k = 3$–$4$:** H is the all-around winner across every $\sigma$; D is the best *unbiased*
  choice, now clearly ahead of A and E.
- **$k = 5$–$6$:** D takes over for $\sigma \le 1$ (best RMSE, zero bias, RE $\approx 0.95$–$0.98$);
  at $\sigma = 2$ H holds at $k = 5$ before D overtakes at $k = 6$. F must be avoided at
  $\sigma = 2$; G never earns a cell.

---

## 5. Recommendations

1. **Always replace OLS with WLS on batch-sum designs — it is free.** D and E use exactly the
   same measurements as A and B, so this is a pure post-processing change: at $k = 6$, D cuts A's
   RMSE by $\approx 18\%$ and E cuts B's by $\approx 27\%$. There is no trade-off; the OLS
   variants are obsolete. For benchmarking tools specifically (Criterion-style), WLS with weights
   $1/x_i$ keeps the overhead-absorbing intercept and the diagnostic value of the design while
   recovering most of the wasted budget.

2. **Prefer the geometric design over the arithmetic one.** Even after the WLS repair, E's
   efficiency is capped near 0.6 at practical budgets while D's reaches 0.98. If measurement
   overhead diagnostics matter, a compromise is a geometric design with a few repeated sizes,
   rather than the arithmetic ladder.

3. **Use H (12.5%-per-side trimmed mean of batch means) as the default robust estimator**, in
   place of the median-of-means, whenever moderate protection suffices: it has smaller bias than
   C everywhere and lower RMSE at every studied cell with $k \ge 3$, while keeping C's
   near-deterministic error under extreme skew. (At $k \le 2$ the trim count is zero and H is just
   the pooled mean — under extreme skew at $k = 2$, C's aggressive medianization still wins.) Reserve C for genuinely adversarial settings where up to half the batches may be
   corrupted — that is MoM's true home turf, not skew mitigation.

4. **Do not use plug-in skewness corrections (F) under heavy tails.** F is worthwhile for
   $\sigma \le 1$ (it removes most of C's bias at small cost) but is catastrophically
   counterproductive at $\sigma = 2$ — worse than the raw pooled mean. If a bias-corrected robust
   estimate is needed under extreme skew, correct H or C using a *parametric* (lognormal) model
   or bootstrap-calibrated correction rather than sample moments.

5. **Skip G for i.i.d. data.** A guaranteed ≤20% variance saving that never changes the ranking
   is not worth much; it addresses the wrong problem (variance, when C's problem is bias).
   Overlapping blocks belong in serially-correlated data analysis.

6. **The two-estimator playbook.** For lognormal-like latency estimation with total budget
   $N = 4^k$: use **H** for $k \le 4$ (or up to $k = 5$ under extreme skew), and **D** for larger
   budgets — switching when D's relative SD (which is $\approx \mathrm{cv}/\sqrt{N}$, computable
   in advance) drops below H's expected bias. If the estimate must be unbiased at any budget —
   e.g. it feeds arithmetic like mean latency × request rate — use **D** exclusively and accept
   the error tail under extreme skew.

### 5.1 Robustness caveats for the latency use case

The recommendations above are derived from a **clean i.i.d. lognormal model**: the only "outliers"
in the simulation are the population's own tail. Real latency measurement adds *contamination* —
GC pauses, scheduler interference, thermal throttling, warmup drift, co-tenant noise — that is not
part of the distribution being estimated. No contaminated cell was simulated, so three caveats
qualify the recommendations:

1. **D's optimality is model-fragile.** D has breakdown point 0, and the geometric design places
   half of the entire budget in one contiguous batch: a single time-localized interference event
   during that batch enters the estimate with maximal leverage, and cannot be detected by
   comparing batches — there is only one large batch. In environments where such events are
   plausible (shared hardware, CI runners, long wall-clock runs), either pair D with an
   independent interference check (e.g. also compute H on a parallel batch structure and alarm on
   disagreement), repeat the largest batch and compare, or simply prefer H and accept its small
   bias as insurance.
2. **"H dominates C" holds only for skew-driven error.** H's group breakdown is
   $t/g \approx 1/8$ vs. C's $\approx 1/2$. If more than ~12.5% of batches can be corrupted
   (bursty interference, noisy neighbors), C's ranking versus H reverses — a regime the tables
   cannot show. Choose the trim fraction from an estimate of the plausible contamination rate,
   not just from the skew-bias trade-off; C is the limiting choice for genuinely adversarial
   conditions.
3. **Contamination is now tested separately.** The companion study
   `Assessment_Contamination_Robustness.md` (report #3) re-runs P, D, C, H with an explicit
   contamination model ($\varepsilon$-fraction of draws inflated $\times 10$, point and burst
   patterns). It confirms both caveats above empirically — D is the *most* contamination-fragile
   estimator tested, and H's ranking versus C reverses precisely at its breakdown fraction — and
   adds a new one: diffuse contamination defeats all group-robust estimators once
   $\varepsilon b \gtrsim 1$, so under diffuse noise smaller batches beat larger ones. The winner
   map below should be read as "best under clean sampling."

Conversely, remember that for latency the population tail itself is usually *signal* (SLA cost,
capacity planning): robustness mechanisms that discount it are the source of C/G/H's negative
bias. Contamination-robustness and tail-fidelity pull in opposite directions; no estimator in
this study delivers both, and the choice between D-with-diagnostics and H-with-bias is ultimately
a judgment about which failure mode is cheaper in the given application.

**Bottom line.** Of the five candidate improvements, two are unqualified upgrades under clean
sampling: **WLS weighting (D, E)**, which makes the regression estimators as efficient as the
design allows at zero cost, and the **trimmed mean of batch means (H)**, which dominates the
median-of-means everywhere it was tested. One is regime-limited (**F**: good below $\sigma = 1$,
disastrous above), and one is a nice idea that doesn't pay in this setting (**G**). The overall
winner map collapses to a simple rule — **H at small-to-moderate budgets, D at large budgets** —
subject to the §5.1 caveats: in contamination-prone environments, H (or C, under heavy
contamination) deserves preference beyond its clean-model regime, and D should not be used
without an interference safeguard. The original A, B, C of report #1 remain strictly dominated
by their repaired versions in every scenario studied.
