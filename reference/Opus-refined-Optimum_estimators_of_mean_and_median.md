# Optimum Estimators of the Mean and Median of a Lognormal Distribution from Batched Group Means

> **Context.** In latency benchmarking the raw per-call execution time of a fast function cannot
> be measured reliably because measurement infrastructure (timer calls, loop control) consumes a
> non-trivial fraction of the wall-clock time. A common remedy is *batching*: call the target
> function $k$ times per timed block and record the block time divided by $k$. The result is a
> *group mean* $Y_j$ of $k$ IID latency observations. This report addresses the statistical problem
> of estimating the mean and median of the underlying latency distribution from those group means.
>
> **$k$ is a technical constraint, not a free statistical design choice.** Let $O$ be the fixed
> timing overhead per block and $\varepsilon$ the fractional-overhead tolerance (typically 1–5%).
> The minimum feasible batch size is
> $$k \geq k_{\min} = \left\lceil \frac{O}{\varepsilon\,\mathrm{E}[X]} \right\rceil.$$
> Because $\mathrm{E}[X]$ is unknown before benchmarking, $k_{\min}$ is found empirically (e.g.,
> doubling $k$ until the per-call estimate stabilises). The analyst receives data with a given $k$
> that may be substantially larger than 1 for sub-microsecond functions.

---

## 1. Notation and Setup

### 1.1 The Lognormal Distribution

A random variable $X$ follows a **lognormal distribution**, $X \sim \mathrm{Lognormal}(\mu, \sigma^2)$,
if $\ln X \sim \mathcal{N}(\mu, \sigma^2)$. Key properties:

| Quantity | Formula |
|---|---|
| Mean | $\mathrm{E}[X] = \exp\!\bigl(\mu + \tfrac{1}{2}\sigma^2\bigr)$ |
| Median | $\mathrm{median}(X) = \exp(\mu)$ |
| Variance | $\mathrm{Var}(X) = \bigl(\exp(\sigma^2)-1\bigr)\exp(2\mu+\sigma^2)$ |
| Coefficient of variation (CV) | $\mathrm{CV}(X)^2 = \exp(\sigma^2)-1$ |
| Mean–median relation | $\mathrm{median}(X) = \mathrm{E}[X]\,/\,\sqrt{1+\mathrm{CV}(X)^2}$ |

The distribution is right-skewed for all $\sigma > 0$. For latency benchmarks, $\sigma$ typically
ranges from $0.2$ (nearly symmetric) to $1.5$ (very heavy right tail).

### 1.2 Problem Setup

Let $X_1, \ldots, X_n$ be IID $\mathrm{Lognormal}(\mu, \sigma^2)$. Choose a batch size $k$
dividing $n$ and partition into $g = n/k$ groups of size $k$:

$$Y_i = \frac{1}{k}\sum_{j=1}^k X_{(i-1)k+j}, \qquad i = 1, \ldots, g.$$

The $Y_i$ are IID. We observe $Y_1, \ldots, Y_g$ but not the individual $X_j$. Our targets are:

- **Mean of $X$**: $\theta_\mu = \mathrm{E}[X] = \exp(\mu + \sigma^2/2)$
- **Median of $X$**: $\theta_m = \mathrm{median}(X) = \exp(\mu)$

### 1.3 Exact Moment Relations

From the IID property, for all $k$ and $\sigma^2$:

$$\mathrm{E}[Y] = \mathrm{E}[X], \qquad \mathrm{Var}(Y) = \frac{\mathrm{Var}(X)}{k}, \qquad \mathrm{CV}(Y)^2 = \frac{\exp(\sigma^2)-1}{k}.$$

These are **exact** and require no distributional approximation for $Y$.

### 1.4 Distribution of Y

The distribution of $Y$ has no closed form for $k > 1$, but two asymptotic regimes are useful:

**Regime A (small $k$, $Y$ approximately lognormal).** The Fenton–Wilkinson (FW) approximation
[1, 2] matches the first two moments of $Y$ to a lognormal $\mathrm{Lognormal}(\mu_Y, \sigma_Y^2)$:

$$\sigma_Y^2 = \ln\!\left(1 + \frac{\exp(\sigma^2)-1}{k}\right), \qquad
\mu_Y = \mu + \frac{\sigma^2 - \sigma_Y^2}{2}.$$

Note $\mu_Y + \sigma_Y^2/2 = \mu + \sigma^2/2 = \ln\mathrm{E}[X]$: the FW lognormal has the same
mean as $X$. The approximation is reliable when $\sigma_Y^2 \lesssim 0.1$ (equivalently
$\mathrm{CV}(Y) \lesssim 0.33$) and degrades as $\sigma^2/k$ grows.

**Regime B (large $k$, $Y$ approximately normal).** By the CLT,
$Y \;\dot\sim\; \mathcal{N}(\mathrm{E}[X],\,\mathrm{Var}(X)/k)$. The approximation error is
$O(k^{-1/2})$; for $\sigma \geq 1.0$, convergence is slow and $k \geq 50$ may be needed.

**Regime C (intermediate $k$).** Neither approximation is fully reliable; distribution-free and
exact-moment methods are required.

### 1.5 Regime Diagnostic

**Compute $\hat{S} = \mathrm{MAD}_\sigma(\ln Y_i)$**, the MAD-normalised estimator of $\sigma_Y$:

$$\hat{S} = \frac{\mathrm{median}_i|\ln Y_i - M|}{0.6745}, \qquad M = \mathrm{median}(\ln Y_i).$$

(The more efficient $Q_n(\ln Y_i)/d_g$ may be substituted; see §3.3.)

| $\hat{S}$ | Approximate regime | Implication |
|:---------:|-------------------|----|
| $\leq 0.3$ | $Y$ nearly normal | Symmetric methods valid; trimmed mean approximately unbiased for $\mathrm{E}[X]$ |
| $0.3$–$0.6$ | $Y$ mildly skewed | Log-space methods preferred; symmetric methods biased for $\mathrm{E}[X]$ |
| $> 0.6$ | $Y$ substantially skewed | Log-space methods required; symmetric methods severely biased for $\mathrm{E}[X]$ |

For $\sigma = 1.5$, reaching $\hat{S} \leq 0.3$ requires approximately $k \geq 250$.

An equivalent diagnostic uses $\tilde{\delta} = \tilde{\sigma}^2/k$ where
$\tilde{\sigma}^2 = \ln(1 + k\,\widetilde{\mathrm{CV}}_Y^2)$ and $\widetilde{\mathrm{CV}}_Y$ is a
robust CV estimate (§3.3). Thresholds: $\tilde{\delta} < 0.05$ (Regime A), $0.05$–$0.20$
(Regime C), $> 0.20$ with $k \geq 30$ (Regime B).

### 1.6 Sample Statistics

| Symbol | Definition | Breakdown |
|--------|-----------|:---------:|
| $\bar{Y}$ | $\frac{1}{g}\sum Y_i$ | 0% |
| $s_Y^2$ | $\frac{1}{g-1}\sum(Y_i-\bar{Y})^2$ | 0% |
| $\tilde{Y}$ | $\mathrm{median}(Y_1,\ldots,Y_g)$ | 50% |
| $Z_i$ | $\ln Y_i$ | — |
| $\bar{Z}$ | $\frac{1}{g}\sum Z_i$ | 0% |
| $s_Z^2$ | $\frac{1}{g-1}\sum(Z_i-\bar{Z})^2$ | 0% |
| $M$ | $\mathrm{median}(Z_1,\ldots,Z_g)$ | 50% |
| $\hat{S}$ | $\mathrm{MAD}_\sigma(Z_1,\ldots,Z_g) = \mathrm{median}|Z_i - M|/0.6745$ | 50% |

---

## 2. Glossary

**Bias.** $\mathrm{Bias} = \mathrm{E}[\hat\theta] - \theta$. An *unbiased* estimator has zero bias.

**Breakdown point.** The fraction of arbitrarily corrupted observations an estimator can tolerate before its output becomes arbitrarily far from the truth. The sample mean has breakdown 0; the sample median has breakdown 0.5 (the maximum possible). See [ref. 4].

**Relative efficiency.** Ratio of baseline MSE to estimator MSE; higher is better.

**Coefficient of variation (CV).** $\mathrm{CV} = \sqrt{\mathrm{Var}(X)}/\mathrm{E}[X]$.

**MAD (Median Absolute Deviation).** $\mathrm{MAD} = \mathrm{median}|X_i - \mathrm{median}(X_i)|$. Normalised by $\Phi^{-1}(0.75) \approx 0.6745$, written $\mathrm{MAD}_\sigma$, it consistently estimates $\sigma$ for a normal distribution [ref. 7]. Breakdown 50%, Gaussian efficiency 37%.

**Rousseeuw–Croux estimators ($Q_n$, $S_n$).** Robust scale estimators with 50% breakdown and Gaussian efficiencies of 82% ($Q_n$) and 58% ($S_n$). Both are consistent estimators of $\sigma$ for a normal distribution. See §3.3 and [ref. 10].

**Trimmed mean.** Arithmetic mean after discarding the lowest $\alpha_L$ and highest $\alpha_R$ fractions. Breakdown $\min(\alpha_L+\alpha_R, 0.5)$. [ref. 5]

**Winsorized mean.** Like the trimmed mean but replaces extremes with the nearest retained value rather than discarding them. [ref. 6]

**M-estimator.** Minimises $\sum\rho(Y_j - \hat\mu)$ for some loss function $\rho$. The [Huber M-estimator](https://en.wikipedia.org/wiki/M-estimator) uses $\rho_c(u) = u^2/2$ for $|u| \leq c$ and $c|u|-c^2/2$ otherwise, with $c = 1.345$ giving 95% Gaussian efficiency [ref. 9].

**Geometric mean.** $\exp(\bar{Z}) = \exp\!\bigl(\frac{1}{g}\sum\ln Y_i\bigr)$. The [MLE](https://en.wikipedia.org/wiki/Log-normal_distribution#Estimation_of_parameters) of $\mathrm{median}(X)$ when $k=1$.

**Log-space estimators.** Estimators defined on $Z_i = \ln Y_i$, exploiting the FW approximation that $\ln Y$ is approximately Gaussian.

**Fenton–Wilkinson (FW) approximation.** Approximates the distribution of a sum of lognormals as lognormal by matching the first two moments (§1.4) [ref. 1, 2].

**Delta method.** Approximates $\mathrm{Var}(f(\hat\theta)) \approx [f'(\theta)]^2\mathrm{Var}(\hat\theta)$ [ref. 12].

**Jensen's inequality.** For convex $f$: $\mathrm{E}[f(X)] \geq f(\mathrm{E}[X])$. Because $\exp$ is convex, estimators of the form $\exp(M + \hat{S}^2/2)$ have a positive finite-sample bias [ref. 13].

---

## 3. Robustness in Latency Benchmarking

### 3.1 Why Robustness is the Priority

Latency benchmarks routinely violate the IID lognormal ideal:

- **GC pauses, OS interrupts, thermal throttling** produce right-tail spikes of 10–1000× the typical latency.
- **Cold-start and warm-up effects** cause clusters of elevated latencies early in a run.
- **Measurement artifacts** occasionally produce physically impossible values.

A single 10× outlier among $g = 20$ group means biases $\bar{Y}$ by approximately $+90\%$ at any $k$ or $\sigma$ (see Table B.1). Both $\bar{Y}$ and $s_Y^2$ have **breakdown point 0%**. **Every estimator in this report is assessed first by its breakdown point; efficiency under ideal lognormality is a secondary consideration.**

### 3.2 Robust Location Estimators

| Estimator | Breakdown | Gaussian efficiency | Notes |
|-----------|:---------:|:-------------------:|-------|
| Sample mean $\bar{Y}$ | 0% | 100% | Fragile; use only as a diagnostic companion. |
| Winsorized mean WM($\alpha$) | $\alpha$ | ~95% at $\alpha=0.1$ | Caps rather than removes extremes. |
| Trimmed mean TM($\alpha_L, \alpha_R$) | $\min(\alpha_L+\alpha_R, 0.5)$ | ~85–95% | Biased for $\mathrm{E}[X]$ when $Y$ is skewed (§4.2). |
| Hodges–Lehmann (HL) | 29% | 95.5% | Median of pairwise averages; $O(g^2)$ naive, $O(g\log g)$ via Monahan [ref. 5]. |
| Sample median $\tilde{Y}$ | 50% | 63.7% | Maximum breakdown; primary location for $k=1$. |

For latency data with right-tail outliers, the sample median provides the maximum achievable robustness.

### 3.3 Robust Scale Estimators

The sample standard deviation $s_Y$ has 0% breakdown. The Rousseeuw–Croux estimators [ref. 10] provide 50% breakdown at substantially higher efficiency than MAD:

**$Q_n$ (preferred):** For sample $u_1, \ldots, u_g$, let $h = \lfloor g/2 \rfloor + 1$:

$$Q_n = d_g \cdot \bigl\{|u_i - u_j| : i < j\bigr\}_{(\binom{h}{2})}$$

where $\{\cdot\}_{(r)}$ denotes the $r$-th smallest of the $\binom{g}{2}$ pairwise absolute
differences, and $d_g \to 2.2219$ as $g \to \infty$ (finite-sample correction tables in [ref. 10]).

**$S_n$:**

$$S_n = c_g \cdot \mathop{\mathrm{median}}_{i} \Bigl\{ \mathop{\mathrm{median}}_{j\neq i}|u_i - u_j| \Bigr\}, \qquad c_g \to 1.1926.$$

| Property | $Q_n$ | $S_n$ | $s_Y$ | $\mathrm{MAD}_\sigma$ |
|----------|:-----:|:-----:|:-----:|:------:|
| Breakdown | 50% | 50% | 0% | 50% |
| Gaussian efficiency | 82% | 58% | 100% | 37% |
| Computation | $O(g\log g)$ | $O(g\log g)$ | $O(g)$ | $O(g\log g)$ |

$Q_n$ is preferred throughout: it has the highest efficiency among the 50%-breakdown scale
estimators and the same interpretation as a robust standard deviation. Replacing MAD with $Q_n$
reduces the finite-sample Jensen's inequality bias in log-space estimators (§4.1, Appendix A.2)
by approximately a factor of 0.45.

**Usage conventions.** For log-space quantities we write $\tilde{S}_Z = Q_n(Z_1,\ldots,Z_g)/d_g$
as the robust estimator of $\sigma_Y$. The simpler $\hat{S} = \mathrm{MAD}_\sigma(Z_i)$ is a
valid substitute when $g \geq 30$ or when the diagnostic role (§1.5) is primary.

### 3.4 Computing the Robust Scale from an HDR Histogram (Implementation Note)

The estimators in §4–§5 are written on the **log scale**: they apply the robust scale to
$Z_i = \ln Y_i$ (e.g. $\tilde{S}_Z = Q_n(\ln Y)/d_g$). When observations are stored in an
[HDR histogram](https://en.wikipedia.org/wiki/High_dynamic_range) — constant relative precision,
integer-valued bins, as is common in latency pipelines — a **natural-scale-first** variant is often
easier to implement and, in the batching regime, statistically equivalent.

**Computing $Q_n$ from a histogram.** $Q_n$ is (up to $d_g$) the $\binom{h}{2}$-th smallest of the
$\binom{g}{2}$ pairwise absolute differences $|Y_i - Y_j|$, with $h = \lfloor g/2\rfloor + 1$ — i.e.
approximately the lower quartile of the difference distribution. That distribution can be
materialised directly: iterate the populated bins of the value histogram in a nested loop and add
$|v_i - v_j|$ to a **second histogram**, weighted by the count product $c_i c_j$ across bins (and
$\binom{c_i}{2}$ within a bin). $Q_n$ is then one quantile query on the second histogram, at cost
$O(B^2)$ in the number of populated bins $B$. The finite-sample constant $d_g$ and the target rank
$\binom{h}{2}$ both key off $g$ (the number of groups), not off the histograms.

**Why the natural scale is convenient here.** On the natural scale the differences $|Y_i - Y_j|$
share the dynamic range of the $Y_j$, so an integer-valued histogram stores them at native
resolution. On the log scale the differences are $O(\mathrm{CV}(Y))$ — small fractional values — so
an integer histogram needs a fixed-point scale factor whose choice trades range against precision.
For HDR-based pipelines the natural scale is the path of least resistance.

**Natural-scale-first route — commit to one lane.** With $\tilde\sigma_Y = Q_n(Y)/d_g$ on the
natural scale, use it in the **exact-moment** estimators, which need no log transform and no FW
approximation (§1.3):

- **Median:** RMoM (§5.3), $\hat\theta_m = \hat\mu_Y / \sqrt{1 + k\,\tilde\sigma_Y^2/\hat\mu_Y^2}$.
- **Mean:** because $\mathrm{E}[Y] = \mathrm{E}[X]$ exactly, a robust location of $Y$ already
  estimates the mean; no scale correction is required.

Do **not** convert a natural-scale $Q_n(Y)$ into $\sigma_Y^2$ to feed a log-space estimator
(§4.1, §5.2): that pairs the FW approximation with the skewed-scale bias below — the worst of both
routes. Use the natural-scale exact-moment route **or** the log-scale route (§3.3), not a mix.

**Calibration caveat (skewed $Y$).** $Q_n$'s constant $d_g \to 2.2219$ is calibrated at the normal.
When $Y$ is near-normal — the regime batching typically lands in, since $k_{\min}$ is large for
low-latency targets (§6.2) — $Q_n(Y)/d_g$ estimates $\mathrm{SD}(Y)$ well and the two routes
coincide. When $Y$ is still skewed (small $k$, large $\sigma$), $Q_n(Y)/d_g$ **underestimates**
$\mathrm{SD}(Y)$ — a robust scale discards the right tail that the variance identity relies on —
biasing the moment inversion. Detect this from a natural-scale quantity you already have:

$$\widetilde{\mathrm{CV}}_Y = \frac{Q_n(Y)/d_g}{\hat\mu_Y}.$$

If $\widetilde{\mathrm{CV}}_Y \lesssim 0.2$ ($Y$ near-normal, equivalently $\hat{S} \lesssim 0.3$;
§1.5) the natural-scale route is well-calibrated. Above that, either accept the residual bias, apply
a skewness correction to $d_g$, or fall back to the log-scale route for that case.

**Resolution note.** At large $k$ the $Y_j$ cluster tightly ($\mathrm{CV}(Y) = \mathrm{CV}(X)/\sqrt{k}$),
so keep the value histogram's unit resolution fine relative to $\mathrm{SD}(Y)$ — e.g. record block
time in fine units rather than pre-dividing by $k$ into coarse integers. This matters on either
scale but more on the log scale.

---

## 4. Estimators for the Mean of X

**Target:** $\theta_\mu = \mathrm{E}[X] = \exp(\mu + \sigma^2/2)$.

Because $\mathrm{E}[Y] = \mathrm{E}[X]$ for all $k$, any consistent estimator of the centre of the
$Y$ distribution estimates $\mathrm{E}[X]$. The key challenge is that for small $k$, the
distribution of $Y$ is strongly right-skewed, so many "centre" estimators underestimate the mean.

### 4.1 Log-Space Mean Estimator (Robust Primary for All $k$)

Let $M = \mathrm{median}(Z_1,\ldots,Z_g)$ and $\tilde{S}_Z = Q_n(Z_1,\ldots,Z_g)/d_g$ (or
$\hat{S} = \mathrm{MAD}_\sigma(Z_i)$). Define:

$$\hat\theta_\mu^{\mathrm{LS}} = \exp\!\left(M + \frac{\tilde{S}_Z^2}{2}\right). \tag{1}$$

**Derivation (Appendix A.2).** The FW approximation gives $Z_i = \ln Y_i \approx
\mathcal{N}(\mu_Y, \sigma_Y^2)$, so $M \xrightarrow{p} \mu_Y$ and $\tilde{S}_Z^2
\xrightarrow{p} \sigma_Y^2$. Since $\mu_Y + \sigma_Y^2/2 = \ln\mathrm{E}[X]$, it follows that
$M + \tilde{S}_Z^2/2 \xrightarrow{p} \ln\mathrm{E}[X]$. This argument holds for all $k$: as
$k\to\infty$, $\sigma_Y^2 \to 0$, $Z_i \to \ln\mathrm{E}[X]$, and the estimator converges to
$\mathrm{E}[X]$.

**Robustness.** Breakdown point $\approx 50\%$ (limited by both the median $M$ and $Q_n$). Under
10% contamination the bias is 2–13% depending on $k$ and $\sigma$, versus $+90\%$ for $\bar{Y}$
(Table B.1).

**Finite-sample Jensen bias (upward).** By convexity of $\exp$:

$$\mathrm{E}\!\left[\exp\!\left(M + \tfrac{\tilde{S}_Z^2}{2}\right)\right] > \exp\!\left(\mathrm{E}[M] + \tfrac{\mathrm{E}[\tilde{S}_Z^2]}{2}\right).$$

The approximate excess is $\dfrac{\pi\sigma_Y^2}{4g} + \dfrac{\sigma_Y^4}{2\,e_{\tilde{S}}\,g}$
where $e_{\tilde{S}}$ is the Gaussian efficiency (0.37 for MAD, 0.82 for $Q_n$). Using $Q_n$
instead of MAD reduces the scale-variance term by a factor of $0.37/0.82 \approx 0.45$.
This bias is negligible when $g \geq 30$ and $\tilde{S}_Z \leq 0.6$, but can reach $+30\%$ at
$k=1$, $\sigma=1.5$, $g=20$ when using MAD (Table B.1); $Q_n$ approximately halves it.

**By regime:**

| Regime | Assessment |
|--------|-----------|
| $k=1$, $\sigma \leq 0.5$, any $g$ | Excellent: $\leq 1\%$ bias, $\approx 50\%$ breakdown. |
| $k=1$, $\sigma > 1.0$, $g \leq 20$ | Substantial upward Jensen bias ($+30\%$ with MAD); use $Q_n$ and/or larger $g$. |
| $k \geq 4$, any $\sigma$, contaminated | Best available (2–9% bias vs. $+90\%$ for $\bar{Y}$). |
| Large $k$ ($\hat{S} \leq 0.3$) | Approaches $\bar{Y}$ in bias and variance; either estimator is acceptable. |

**Pros:** $\approx 50\%$ breakdown; valid for all $k$; explicitly accounts for skewness of $Y$
via the $\tilde{S}_Z^2/2$ correction term.

**Cons:** Upward Jensen bias at small $g$ with heavy tails; requires $g \geq 10$ for reliable
$Q_n$; relies on FW approximation for validity at small $k$.

### 4.2 Trimmed Mean of Y (Secondary: Approximately Symmetric Y Only)

$$\hat\theta_\mu^{\mathrm{TM}(\alpha_L, \alpha_R)} = \frac{1}{g - t_L - t_R}\sum_{i=t_L+1}^{g-t_R} Y_{(i)}, \qquad t_L = \lfloor\alpha_L g\rfloor,\; t_R = \lfloor\alpha_R g\rfloor.$$

**Bias.** The trimmed mean consistently estimates the trimmed expectation, which equals
$\mathrm{E}[X]$ only when $Y$ is symmetric. For right-skewed $Y$, discarding the right tail
removes observations that contribute substantially to the mean, causing downward bias:

| | $k=1$ | $k=4$ | $k=16$ | $k=64$ |
|---|---|---|---|---|
| $\sigma = 0.5$, 20% symmetric trim, clean | $-8.5\%$ | $-2.5\%$ | $-0.7\%$ | $-0.2\%$ |
| $\sigma = 1.5$, 20% symmetric trim, clean | $-55\%$ | $-30\%$ | $-13\%$ | $-5\%$ |
| $\sigma = 1.5$, 20% symmetric trim, 10% contam. | $-44\%$ | $-16\%$ | $-4\%$ | $\approx 0\%$ |

Right-only trimming ($\alpha_L = 0$, $\alpha_R = 0.1$) reduces but does not eliminate this bias
for skewed $Y$.

**Robustness.** Breakdown $\min(\alpha_L + \alpha_R, 0.5)$. Excellent for symmetric $Y$; the
combined bias $+$ contamination penalty is small only when $\hat{S} \leq 0.3$.

**Recommendation.** Use only when $\hat{S} \leq 0.3$ (approximately normal $Y$). **Do not use as
a primary mean estimator when $Y$ is skewed** — the downward bias can exceed 50% and is not
offset by the reduction in variance.

### 4.3 Huber M-Estimator (Secondary: Approximately Symmetric Y Only)

The Huber M-estimator $\hat\theta_\mu^H$ solves $\sum_j \psi_c((Y_j - \hat\theta_\mu^H)/\hat\sigma_{\mathrm{sc}}) = 0$ with $\psi_c(u) = \min(|u|,c)\,\mathrm{sgn}(u)$, $c=1.345$, and $\hat\sigma_{\mathrm{sc}}$ estimated via MAD. Simulation results are consistently close to the 20% trimmed mean (Table B.1). **Same restriction applies: use only when $\hat{S} \leq 0.3$.**

### 4.4 Sample Mean (Diagnostic Baseline)

$$\hat\theta_\mu^{\mathrm{SM}} = \bar{Y}.$$

Exactly unbiased for $\mathrm{E}[X]$ at all $k$, $\sigma$, $g$. Breakdown 0%.

**When contamination is possible (the default assumption for latency benchmarking): not a
primary estimator** — a single 10× outlier biases it $+90\%$ (Table B.1). Report it alongside
$\hat\theta_\mu^{\mathrm{LS}}$ as a robustness diagnostic: agreement signals clean data;
$\bar{Y} \gg \hat\theta_\mu^{\mathrm{LS}}$ flags right-tail contamination.

**When contamination can be excluded (verified-clean data), $\bar{Y}$ is a legitimate — indeed
efficient — primary at $k=1$**, where $\bar{Y} = \bar{X}$. For the $\sigma$ range typical of
latency benchmarks ($0.1$–$0.5$) its efficiency loss versus the lognormal MLE is at most $\approx 1\%$
(the relative efficiency is $(\sigma^2+\sigma^4/2)/(e^{\sigma^2}-1)$, which is $\approx 0.99$–$1.00$
there and falls only for $\sigma \gtrsim 1$). Its unbiasedness-at-all-$k$ then makes it hard to beat.
This is the one setting where robustness is not the binding concern; even so, pair it with a robust
companion for production reporting.

### 4.5 Log-Normal MLE (Efficient at $k=1$ on Clean Data; Fragile Otherwise)

$$\hat\theta_\mu^{\mathrm{LN}} = \exp\!\left(\bar{Z} + \frac{s_Z^2}{2}\right).$$

Breakdown 0%. A single outlier inflates $s_Z^2$, which enters **exponentially** via the $s_Z^2/2$
term, making this estimator **more fragile than $\bar{Y}$ under contamination** — so it is **not
recommended when contamination is possible** (the default latency assumption).

There is, however, a legitimate niche. At $k=1$ this is the exact lognormal **MLE** of
$\mathrm{E}[X]$: consistent, asymptotically efficient (attains the CRLB
$(\sigma^2+\sigma^4/2)/n$, below the sample mean's $(e^{\sigma^2}-1)/n$), and thus the **most
efficient consistent estimator when log-normality is trusted and the data are verified clean**.
Its finite-sample bias is upward, $O(1/n)$ (Finney 1941 [ref. 6]), and negligible for $n \gtrsim 100$.
The robust primary $\hat\theta_\mu^{\mathrm{LS}}$ (§4.1) is precisely this estimator with the
**median** of $\ln Y$ in place of the mean $\bar{Z}$ and a robust scale in place of $s_Z^2$ —
trading a little efficiency for $\approx 50\%$ breakdown.

**Small samples ($n < 30$).** The MLE's $O(1/n)$ upward bias is removed exactly by the **UMVUE**
(Finney 1941 [ref. 6]; Shimizu & Iwase 1981), $\exp(\bar{Z})\cdot\Psi(s_Z^2/2, n)$, which is
exactly unbiased and minimum-variance among unbiased estimators; for $n > 100$ it and the MLE are
practically identical. Use the UMVUE only in the same clean, trusted-lognormal niche.

### 4.6 Comparison and Recommendations for the Mean

| Regime | Primary | Secondary | Do not use |
|--------|---------|-----------|------------|
| $\hat{S} > 0.3$ (skewed $Y$, any $k$), contamination possible | **Log-space mean** $\exp(M + \tilde{S}_Z^2/2)$ | $\bar{Y}$ as diagnostic only | TM, Huber-M (severely biased) |
| $\hat{S} \leq 0.3$ (symmetric $Y$, large $k$), contamination possible | **Log-space mean** or $\bar{Y}$ | TM, Huber-M | LN-MLE |
| $k=1$, verified clean, log-normality trusted | **$\bar{Y}$** (unbiased, $\approx$ full efficiency) or **LN-MLE** ($n \geq 30$) / **UMVUE** ($n < 30$) | Log-space mean (robust check) | — |
| $k=n$ ($g=1$) | $Y_1$ (only option) | — | — |

**Default (contamination possible — the latency norm):** Report the log-space mean
$\exp(M + \tilde{S}_Z^2/2)$ with $Q_n$ scale as the primary robust estimator and $\bar{Y}$ as the
diagnostic companion. If $g < 30$ and $\tilde{S}_Z > 0.6$, flag possible Jensen bias and consider
increasing $g$.

**When contamination can be excluded** and the data are trusted lognormal, the efficiency-first
choice at $k=1$ is $\bar{Y} = \bar{X}$ (unbiased, $\leq 1\%$ efficiency loss for $\sigma \leq 0.5$),
or the LN-MLE / UMVUE (§4.5) if maximum efficiency under log-normality is wanted; still report the
robust log-space mean as a cross-check.

---

## 5. Estimators for the Median of X

**Target:** $\theta_m = \mathrm{median}(X) = \exp(\mu)$.

**Fundamental challenge.** When $k > 1$, $\mathrm{median}(Y) \approx \exp(\mu_Y) = \exp(\mu + (\sigma^2 - \sigma_Y^2)/2) > \exp(\mu)$. As $k \to \infty$, $\mathrm{median}(Y) \to \mathrm{E}[X] = \exp(\mu + \sigma^2/2) \gg \exp(\mu)$ for $\sigma > 0$. Simulation confirms: at $k=16$, $\sigma=1.5$, the sample median of $Y$ overestimates $\mathrm{median}(X)$ by $+158\%$ (Table B.2). **Batching degrades the direct estimability of the median;** the larger $k$ is, the more a back-transform estimator is needed, and the more residual bias that estimator carries.

### 5.1 Sample Median (k = 1 Primary)

$$\hat\theta_m^{\mathrm{Med}} = \mathrm{median}(Y_1,\ldots,Y_g).$$

At $k=1$, $Y_i = X_i$ and this is the sample median of $X$.

**Robustness.** Breakdown 50%.

**Efficiency.** ARE $\approx 63.7\%$ vs. the geometric mean under strict lognormality.

**For $k > 1$:** Estimates $\mathrm{median}(Y) \neq \mathrm{median}(X)$. Badly biased; not applicable as a median-of-$X$ estimator.

### 5.2 Log-Space Median Estimator (Robust, Any k)

Using $M = \mathrm{median}(Z_i)$ and $\tilde{S}_Z = Q_n(Z_i)/d_g$, compute:

$$\hat\sigma_X^2 = \ln\!\left(1 + k\,\bigl(\exp(\tilde{S}_Z^2) - 1\bigr)\right), \tag{2}$$

$$\hat\theta_m^{\mathrm{LS}} = \exp\!\left(M + \frac{\tilde{S}_Z^2 - \hat\sigma_X^2}{2}\right). \tag{3}$$

**Derivation (Appendix A.3).** FW gives $M \approx \mu_Y$ and $\tilde{S}_Z^2 \approx \sigma_Y^2$.
Equation (2) inverts the FW relation to recover $\hat\sigma_X^2 \approx \sigma^2$. Then:

$$M + \frac{\tilde{S}_Z^2 - \hat\sigma_X^2}{2} \approx \mu_Y + \frac{\sigma_Y^2 - \sigma^2}{2} = \mu.$$

At $k=1$: $\hat\sigma_X^2 = \tilde{S}_Z^2$, so $\hat\theta_m^{\mathrm{LS}} = \exp(M)$ — the robust
geometric mean using the median (rather than the mean) of log-values. As $g\to\infty$,
$\hat\theta_m^{\mathrm{LS}} \to \exp(\mu) = \mathrm{median}(X)$ at any fixed $k$.

**Robustness.** Breakdown $\approx 50\%$ (from $M$ and $Q_n$).

**Bias.** For $\sigma = 0.5$: $< 1\%$ at all $k$ (Table B.2). For $\sigma = 1.5$: residual bias
of 8–30% due to Jensen's inequality and FW approximation error, decreasing with $g$. Using $Q_n$
in place of MAD approximately halves the bias component from scale estimation. For very large $k$,
the denominator $\hat\sigma_X^2$ shrinks and the estimator converges toward $\exp(M) \approx
\mathrm{E}[X]$ rather than $\mathrm{median}(X)$; this regime is where RMoM (§5.3) is superior.

**By regime:**

| Regime | Assessment |
|--------|-----------|
| $k=1$, $\sigma \leq 1.0$ | Good; slightly less efficient than geometric mean; more robust. |
| $k \geq 4$, $\sigma \leq 0.5$, clean | $< 1\%$ bias; somewhat higher RMSE than classical BT. |
| $k \geq 4$, $\sigma > 1.0$ | Best robust option; 15–30% residual bias; superior under contamination. |
| Any $k$, contaminated | **Best option** — far superior to all non-robust alternatives. |

**Pros:** $\approx 50\%$ breakdown; valid across all $k$; no distributional assumptions beyond FW.

**Cons:** Residual Jensen bias for large $\sigma$ and/or small $g$; FW approximation degrades at very large $k$; $Q_n$ requires $g \geq 10$.

### 5.3 Robust Method of Moments — RMoM (Any k)

From the exact identity $\mathrm{median}(X) = \mathrm{E}[X]/\sqrt{1+\mathrm{CV}(X)^2}$, substituting robust estimators:

$$\hat\theta_m^{\mathrm{RMoM}} = \frac{\hat\mu_Y}{\sqrt{1 + k\,\tilde\sigma_Y^2/\hat\mu_Y^2}}, \tag{4}$$

where $\hat\mu_Y$ is a robust location estimator for $\mathrm{E}[Y]$ and
$\tilde\sigma_Y = Q_n(Y_1,\ldots,Y_g)/d_g$ is the robust scale (both computable directly on the
natural scale from an HDR histogram; see §3.4). Two options for $\hat\mu_Y$:

- **Trimmed mean** (10% right-trim): breakdown 10%; $\hat\mu_Y$ estimates $\mathrm{E}[Y]$ with
  little bias for the MoM formula, which was derived assuming $\hat\mu_Y \approx \mathrm{E}[Y]$.
- **Sample median** $\tilde{Y}$: breakdown 50%; but $\tilde{Y}$ estimates $\mathrm{median}(Y) \neq
  \mathrm{E}[Y]$ for $k > 1$, introducing a location bias in the formula. The bias is small when
  $\mathrm{CV}(Y)$ is small (large $k$ or small $\sigma$).

**Robustness.** Limited by $\hat\mu_Y$: 10% (trimmed mean) or 50% (median, with location bias).

**Validity.** Uses the **exact** moment relations (§1.3), not the FW approximation. Consistent
for $\exp(\mu)$ as $g \to \infty$ at any fixed $k$. Performs well at large $k$ where the
log-space median degrades.

**Efficiency.** Approximately 70–85% of the standard MoM (classical BT) on clean data.

**Pros:** Valid for all $k$; exact moment relations; no FW approximation; tunable robustness.

**Cons:** Efficiency loss vs. classical BT on clean data; requires reliable estimation of
$\mathrm{CV}(Y)^2 = \tilde\sigma_Y^2/\hat\mu_Y^2$; $O(g\log g)$ for $Q_n$.

### 5.4 Geometric Mean (k = 1 Secondary, Efficient But Fragile)

$$\hat\theta_m^{\mathrm{GM}} = \exp(\bar{Z}).$$

At $k=1$ this is the MLE of $\exp(\mu)$ and is asymptotically efficient [ref. 6]. For $k > 1$,
it estimates $\exp(\mu_Y) > \exp(\mu)$ (upward bias: $+10\%$ at $k=4$, $\sigma=0.5$; $+165\%$
at $k=16$, $\sigma=1.5$; see Appendix A.5). **Do not use as a median estimator for $k > 1$.**

Breakdown 0%. Use at $k=1$ as a secondary check when a normality test on $Z_i = \ln Y_i$ passes
($p > 0.10$); otherwise prefer the log-space median estimator $\exp(M)$.

### 5.5 Classical Back-Transform — Standard MoM (Clean Data Only)

$$\hat\theta_m^{\mathrm{BT}} = \frac{\bar{Y}}{\sqrt{1 + k\, s_Y^2/\bar{Y}^2}}.$$

Uses the exact moment identity (Appendix A.6). Breakdown 0%: a single large outlier inflates
$s_Y^2$, driving $\hat\theta_m^{\mathrm{BT}}$ toward zero. Simulation: $-82\%$ bias at $k=64$,
$\sigma=0.5$, 10% contamination (Table B.2). Has the lowest RMSE of all median estimators on
clean data with $\sigma \leq 0.5$ and $k \geq 4$.

**Recommendation.** Use only in controlled environments where contamination can be confidently
excluded, with $\sigma \leq 0.5$ and $k \geq 4$. Always verify by comparing with RMoM.

### 5.6 Comparison and Recommendations for the Median

| Regime | Primary (robust) | Secondary | Do not use |
|--------|-----------------|-----------|------------|
| $k=1$, contamination expected | **Sample median** (50% BP) | Log-space median $\exp(M)$ | GM (fragile) |
| $k=1$, clean, lognormality confirmed | **GM** (MLE) | Log-space median $\exp(M)$ | — |
| $k > 1$, contaminated or $\hat{S} > 0.3$ | **Log-space median** (Eq. 3) | **RMoM** (Eq. 4) | GM (biased), classical BT (fragile) |
| $k > 1$, large $k$ ($\hat{S} \leq 0.3$), clean | **Classical BT** (lowest RMSE) | **RMoM** | GM (biased), sample median of $Y$ |
| $k=n$ ($g=1$) | $Y_1$ only | — | — |

**Default for $k > 1$:** Report **log-space median** (Eq. 3) as the robust primary and **RMoM**
(Eq. 4) as a cross-check. Agreement signals reliable estimation; divergence prompts investigation
of outliers or FW approximation quality.

**Location choice by sub-regime (small $k$ vs. large $k$).** The log-space median (Eq. 3) uses a
**median** location on $\ln Y$; RMoM (Eq. 4) admits either a **mean** location (trimmed mean of
$Y$) or a median location. These trade off differently with $k$:

- **Small $k$ ($Y$ still skewed):** $\mathrm{median}(\ln Y)$ is itself a skew-biased estimate of
  $\mu_Y$, so a robust **mean-location** RMoM ($\hat\mu_Y$ = right-trimmed mean of $Y$) can carry
  less location bias, since $\mathrm{E}[Y] = \mathrm{E}[X]$ holds exactly at every $k$. Its cost is
  lower breakdown (the trimmed mean is 10%, not 50%), so screen for outlier batches.
- **Moderate-to-large $k$ ($Y$ near-symmetric):** $\mathrm{median}(Y) \approx \mathrm{E}[Y]$ and
  the FW correction $\to 1$, so the median-location log-space median is both robust ($\approx 50\%$
  breakdown) and $\approx$ unbiased — the preferred choice.

Note that Eq. (3) already folds in the FW $\tilde{S}_Z^2$ term, so it stays consistent at **every**
$k$; the naive corrected median $e^{-\hat\sigma_X^2/2}\,\mathrm{median}(Y)$ (which omits that term)
is biased low at small $k$ and should not be substituted for it.

---

## 6. Decision Rules

### 6.1 Regime Classification

1. Compute $Z_i = \ln Y_i$, $M = \mathrm{median}(Z_i)$, $\tilde{S}_Z = Q_n(Z_i)/d_g$.
2. Apply Table in §1.5 to classify by $\hat{S} \approx \tilde{S}_Z$.
3. If $g = 1$ ($k = n$): degenerate — report $Y_1$ and flag unquantifiable uncertainty.

### 6.2 Choosing k

**$k$ is determined by measurement overhead, not by statistical preference** (§1.2). Use $k = k_{\min}$:
- **For median estimation:** Every increment above $k_{\min}$ widens the gap between $\mathrm{median}(Y)$ and $\mathrm{median}(X)$, increasing back-transform bias. Use $k_{\min}$.
- **For mean estimation:** The log-space mean is valid at all $k$. Larger $k$ reduces $\hat{S}$ and reduces the small bias of trimmed-mean alternatives, but there is no statistical reason to increase $k$ beyond $k_{\min}$ when using the log-space mean.
- **If $\hat{S} > 0.6$ after fixing $k = k_{\min}$:** The distribution of $Y$ is still strongly skewed. Do **not** increase $k$ to reduce $\hat{S}$ — that costs groups $g$ and worsens median estimation. Instead, collect more groups (increase total $n$ at fixed $k$).

### 6.3 Decision Flow: Mean of X

```
Input: Y_1, ..., Y_g; batch size k.

1. Compute Z_i = ln Y_i, M = median(Z_i), S̃ = Q_n(Z_i)/d_g.

2. If k = n (g = 1):
       → Report Y_1. Flag: no robustness or uncertainty quantification possible.

3. Primary estimate:
       θ̂_mean = exp(M + S̃²/2)          [log-space mean, ~50% breakdown]

4. Diagnostic:
       Compute Ȳ and compare with θ̂_mean.
       • If θ̂_mean ≈ Ȳ:   data is well-behaved.
       • If Ȳ >> θ̂_mean:  right-tail contamination present; flag and investigate.

5. Jensen bias flag:
       If S̃ > 0.6 AND g < 30:
           Upward bias in θ̂_mean may be +10–30%. Consider larger g.
           Switching MAD to Q_n (if not already done) approximately halves this bias.

6. If S̃ ≤ 0.3 AND no contamination suspected:
       TM(0.1, 0.1) or Huber-M are also valid cross-checks (approximately unbiased here).

7. If k = 1 AND data verified clean AND log-normality trusted (efficiency-first):
       θ̂_mean = Ȳ = X̄                    [unbiased, ≤1% efficiency loss for σ ≤ 0.5]
       or exp(Z̄ + s²_Z/2) (LN-MLE, n ≥ 30) / UMVUE (n < 30) for max efficiency.
       Still report the log-space mean above as a robustness cross-check.
```

### 6.4 Decision Flow: Median of X

```
Input: Y_1, ..., Y_g; k; diagnostic S̃ from step 1 of §6.3.

If k = 1:
    If contamination suspected:
        → Primary:   sample median (50% breakdown)
        → Secondary: exp(M)  [log-space median, robust]
    Else (clean):
        → Primary:   exp(Z̄)  [geometric mean, MLE]
        → Secondary: exp(M)  [robust check]

Else if k = n (g = 1):
    → Report Y_1. Flag: no inference possible.

Else (k > 1):
    → Primary:   exp(M + (S̃² − σ̂²_X)/2)   where σ̂²_X = ln(1 + k(exp(S̃²)−1))
                 [log-space median, ~50% breakdown]
    → Secondary: Ỹ / sqrt(1 + k·σ̃²_Y/Ỹ²) where Ỹ = median(Y_i), σ̃_Y = Q_n(Y_i)/d_g
                 [RMoM with median location, 50% breakdown; note location bias when CV(Y) large]

    If estimates agree:   reliable.
    If estimates diverge: investigate outliers; consider larger g.

    If S̃ ≤ 0.3 AND clean environment confirmed:
        → Ȳ / sqrt(1 + k·s²_Y/Ȳ²)  [classical BT, lowest RMSE but 0% breakdown — comparison only]

Always: flag Y_i > Q3 + 3·IQR for manual investigation.
```

### 6.5 Outlier Handling

Label $Y_i$ exceeding $Q_3 + 3 \times \mathrm{IQR}$ as flagged for manual investigation. The analyst determines whether flagged observations are genuine rare events (retain) or measurement artifacts (exclude). The robust estimators above automatically downweight such observations through trimming or robust scale estimation; the labeling rule is **diagnostic, not corrective**.

---

## 7. Summary Tables

### 7.1 Mean Estimators

| Estimator | Breakdown | $k=1$, $\sigma \leq 0.5$ | $k=1$, $\sigma > 1$ | $k{=}4$–$16$, clean | $k{=}4$–$16$, contaminated | $k \geq 64$ |
|-----------|:---------:|--------------------------|---------------------|---------------------|---------------------------|-------------|
| **Log-space mean** $\exp(M+\tilde{S}_Z^2/2)$ | ~50% | ✓ 1% bias | ⚠ +30% (small $g$, MAD) | ✓ 2–5% bias | ✓ 2–9% bias | ✓ Good |
| **20% trimmed mean** | 20% | ⚠ $-8.5\%$ | ✗ $-55\%$ | ⚠ $-3$ to $-30\%$ | ✓ ($k \geq 64$) | ✓ $< 2\%$ |
| **Huber-M** | ~20% | ⚠ $-6\%$ | ✗ $-54\%$ | ⚠ $-2$ to $-26\%$ | ✓ ($k \geq 64$) | ✓ $< 2\%$ |
| **Sample mean** $\bar{Y}$ | 0% | ✓ Unbiased; fragile | ✓ Unbiased; fragile | ✓ Unbiased; fragile | ✗ $+90\%$ | ✓ Unbiased; less fragile |
| **LN-MLE** $\exp(\bar{Z}+s_Z^2/2)$ | 0% | ✗ Not recommended | ✗ Not recommended | ✗ Not recommended | ✗ Not recommended | ✗ Not recommended |

✓ = recommended; ⚠ = usable with caveats (state the caveat); ✗ = avoid.
Bias figures from Monte Carlo simulation ($g = 20$, $\mu = 0$, 10% contamination = 10% of $g$ observations scaled ×10, all log-space estimators using MAD). See Table B.1.
The ✗ entries for **LN-MLE** are for the contamination-possible default; on **verified-clean,
trusted-lognormal data at $k=1$** the LN-MLE (or UMVUE for $n<30$) is instead the *efficient*
choice (§4.5) — the ✗ reflects fragility, not inefficiency.

### 7.2 Median Estimators

| Estimator | Breakdown | $k=1$, clean | $k=1$, contaminated | $k>1$, $\sigma{\leq}0.5$, clean | $k>1$, contaminated | $k>1$, $\sigma{>}1$ |
|-----------|:---------:|-------------|---------------------|--------------------------------|---------------------|---------------------|
| **Sample median** $\mathrm{med}(Y_i)$ | 50% | ✓ Robust | ✓ Very robust | ✗ $+10$–$13\%$ bias | ✓ Robust; biased | ✗ $+158\%$ bias |
| **Log-space median** (Eq. 3) | ~50% | ✓ Robust | ✓ Best overall | ✓ $< 1\%$ bias | ✓ Best robust | ⚠ $15$–$30\%$ residual |
| **RMoM** (Eq. 4) | 10–50%† | ○ Valid | ○ Moderate | ✓ Robust | ✓ Robust | ✓ Valid, any $k$ |
| **Geometric mean** $\exp(\bar{Z})$ | 0% | ✓ MLE; efficient | ✗ $+27$–$33\%$ | ✗ Biased | ✗ Catastrophic | ✗ Useless |
| **Classical BT** $\bar{Y}/\sqrt{1+k s_Y^2/\bar{Y}^2}$ | 0% | — (= GM at $k=1$) | ✗ Fragile | ✓ Lowest RMSE | ✗ $-82\%$ | ⚠ $+14$–$35\%$ |

✓ = recommended; ○ = acceptable alternative; ⚠ = usable with caveats; ✗ = avoid. See Table B.2.
† RMoM breakdown: 10% with trimmed-mean location, 50% with median location (the latter introduces location bias for large $\mathrm{CV}(Y)$).

---

## Appendix A: Derivations

### A.1 Lognormal Moment Identities

Let $X \sim \mathrm{Lognormal}(\mu, \sigma^2)$.

**Mean.** $\mathrm{E}[X] = \mathrm{E}[e^{\ln X}] = e^{\mu+\sigma^2/2}$ (MGF of normal evaluated at 1).

**Median.** $P(X \leq m) = \tfrac{1}{2} \Leftrightarrow \ln m = \mu$, so $m = e^\mu$.

**Variance.** $\mathrm{Var}(X) = e^{2\mu+2\sigma^2} - e^{2\mu+\sigma^2} = e^{2\mu+\sigma^2}(e^{\sigma^2}-1)$.

**Mean–median.** $\mathrm{E}[X]/\mathrm{median}(X) = e^{\sigma^2/2} = \sqrt{1+\mathrm{CV}(X)^2}$.

**Moments of $Y_j$** (IID sum of $k$ copies):
$$\mathrm{E}[Y] = \mathrm{E}[X], \quad \mathrm{Var}(Y) = \mathrm{Var}(X)/k, \quad \mathrm{CV}(Y)^2 = (e^{\sigma^2}-1)/k.$$

### A.2 Derivation of Log-Space Mean Estimator

By the FW approximation (§1.4), $Z_i = \ln Y_i \approx \mathcal{N}(\mu_Y, \sigma_Y^2)$ where:
$$\mu_Y = \mu + \tfrac{\sigma^2 - \sigma_Y^2}{2}, \qquad \mu_Y + \tfrac{\sigma_Y^2}{2} = \mu + \tfrac{\sigma^2}{2} = \ln\mathrm{E}[X].$$

Since $Z_i$ is approximately normal, $M \xrightarrow{p} \mu_Y$ and $\tilde{S}_Z^2 \xrightarrow{p}
\sigma_Y^2$. Thus $M + \tilde{S}_Z^2/2 \xrightarrow{p} \ln\mathrm{E}[X]$ and $\exp(M +
\tilde{S}_Z^2/2) \xrightarrow{p} \mathrm{E}[X]$.

**Finite-sample Jensen bias.** By convexity of $\exp$:
$$\mathrm{E}\!\left[\exp\!\bigl(M + \tilde{S}_Z^2/2\bigr)\right] > \exp\!\bigl(\mathrm{E}[M] + \mathrm{E}[\tilde{S}_Z^2]/2\bigr).$$

The leading bias is approximately $\dfrac{\pi\sigma_Y^2}{4g} + \dfrac{\sigma_Y^4}{2\,e_{\tilde{S}}\,g}$,
where $e_{\tilde{S}}$ is the Gaussian efficiency of $\tilde{S}_Z$ (0.37 for MAD, 0.82 for $Q_n$).
The factor $e_{\tilde{S}}$ appears because $\mathrm{Var}(\tilde{S}_Z^2) \propto 1/(e_{\tilde{S}}\,g)$;
replacing MAD with $Q_n$ reduces this contribution by $0.37/0.82 \approx 0.45$.

### A.3 Derivation of Log-Space Median Estimator

We want $\exp(\mu)$. From the FW parametrisation: $\mu = \mu_Y + (\sigma_Y^2 - \sigma^2)/2$.

Estimating $\mu_Y$ by $M$ and $\sigma_Y^2$ by $\tilde{S}_Z^2$, invert the FW relation for $\sigma^2$:
$$\hat\sigma_X^2 = \ln\!\bigl(1 + k\,(\exp(\tilde{S}_Z^2) - 1)\bigr).$$

This is the exact inverse of $\sigma_Y^2 = \ln(1 + (e^{\sigma^2}-1)/k)$. Then:
$$\hat\theta_m^{\mathrm{LS}} = \exp\!\left(M + \frac{\tilde{S}_Z^2 - \hat\sigma_X^2}{2}\right).$$

*At $k=1$:* $\hat\sigma_X^2 = \tilde{S}_Z^2$, so $\hat\theta_m^{\mathrm{LS}} = \exp(M)$.

*Consistency:* As $g\to\infty$, $M \to \mu_Y$ and $\tilde{S}_Z^2 \to \sigma_Y^2$, so $\hat\sigma_X^2 \to \sigma^2$ and $\hat\theta_m^{\mathrm{LS}} \to \exp(\mu) = \mathrm{median}(X)$.

### A.4 Fenton–Wilkinson Approximation

For $S = \sum_{j=1}^k X_j$ with $X_j \sim \mathrm{Lognormal}(\mu, \sigma^2)$ IID, the FW method
approximates $S \sim \mathrm{Lognormal}(\mu_S, \sigma_S^2)$ by equating the first two moments:

$$\mathrm{E}[S] = k\,e^{\mu+\sigma^2/2}, \qquad \mathrm{Var}(S) = k(e^{\sigma^2}-1)e^{2\mu+\sigma^2}.$$

Matching to a lognormal gives $\mathrm{CV}_S^2 = (e^{\sigma^2}-1)/k$ and:

$$\sigma_S^2 = \ln\!\left(1 + \frac{e^{\sigma^2}-1}{k}\right), \qquad \mu_S = \ln k + \mu + \frac{\sigma^2-\sigma_S^2}{2}.$$

For $Y = S/k$: $\mu_Y = \mu_S - \ln k$ and $\sigma_Y^2 = \sigma_S^2$.

### A.5 Bias of the Geometric Mean as a Median Estimator for $k > 1$

The GM estimates $\exp(\mathrm{E}[\ln Y]) = \exp(\mu_Y)$, not $\exp(\mu)$. The FW-based bias factor:

$$B(k,\sigma^2) = \frac{\exp(\mu_Y)}{\exp(\mu)} = \exp\!\left(\frac{\sigma^2 - \sigma_Y^2}{2}\right).$$

At $k=1$: $B=1$. At $k=4$, $\sigma^2=0.25$ ($\sigma=0.5$): $B \approx 1.10$ ($+10\%$). At $k=16$, $\sigma^2=2.25$ ($\sigma=1.5$): $B \approx 2.65$ ($+165\%$). The bias grows with both $k$ and $\sigma^2$, confirming that the GM should not be used as a median estimator for $k > 1$.

### A.6 Derivation of the Classical Back-Transform

From the exact identity $\mathrm{median}(X) = \mathrm{E}[X]/\sqrt{1+\mathrm{CV}(X)^2}$, substitute:
- $\widehat{\mathrm{E}[X]} = \bar{Y}$ (unbiased);
- $\widehat{\mathrm{CV}(X)^2} = k\,s_Y^2/\bar{Y}^2$ (using $\mathrm{CV}(Y)^2 = \mathrm{CV}(X)^2/k$).

This gives $\hat\theta_m^{\mathrm{BT}} = \bar{Y}/\sqrt{1+k\,s_Y^2/\bar{Y}^2}$. Breakdown 0% because $s_Y^2$ has breakdown 0%.

### A.7 Rousseeuw–Croux $Q_n$ and $S_n$ Estimators

For sample $u_1,\ldots,u_g$, let $h = \lfloor g/2\rfloor + 1$, $r = \binom{h}{2}$:

$$Q_n = d_g \cdot \bigl\{|u_i - u_j| : i < j\bigr\}_{(r)}, \qquad S_n = c_g \cdot \mathop{\mathrm{median}}_i\!\left\{\mathop{\mathrm{median}}_{j\neq i}|u_i - u_j|\right\}.$$

Consistency factors for normal data: $d_g \to 2.2219$, $c_g \to 1.1926$ as $g\to\infty$.
Both achieve 50% breakdown. Computation: $O(g\log g)$. $Q_n$ Gaussian efficiency 82%, $S_n$ 58%.

---

## Appendix B: Empirical Validation

Monte Carlo simulation ($10^5$ replications; $\mu=0$, so true median $=1$, true mean $=
\exp(\sigma^2/2)$; 10% contamination means 10% of the $g$ group means are replaced by draws
scaled $\times 10$, simulating latency spikes). All log-space estimators use MAD for scale;
substituting $Q_n$ reduces Jensen bias by $\approx 45\%$ for the scale term (Appendix A.2).
Relative RMSE is normalised to the row-group baseline: Table B.1 to grand mean, Table B.2 to
geometric mean.

### Table B.1 — Estimators for the Mean of X

| $k$ | $g$ | $\sigma$ | Contam | Estimator | Bias% | Rel-RMSE |
|--:|--:|--:|--------|-----------|------:|---------:|
| 1 | 20 | 0.5 | none | Grand mean | +0.0 | 1.000 |
| 1 | 20 | 0.5 | none | Log-space mean | +1.0 | 1.293 |
| 1 | 20 | 0.5 | none | 20% trimmed mean | −8.5 | 1.165 |
| 1 | 20 | 0.5 | none | Huber-M | −6.4 | 1.085 |
| 4 | 20 | 0.5 | none | Grand mean | −0.0 | 1.000 |
| 4 | 20 | 0.5 | none | Log-space mean | +0.0 | 1.222 |
| 4 | 20 | 0.5 | none | 20% trimmed mean | −2.5 | 1.101 |
| 4 | 20 | 0.5 | none | Huber-M | −1.8 | 1.049 |
| 16 | 20 | 0.5 | none | Grand mean | +0.0 | 1.000 |
| 16 | 20 | 0.5 | none | Log-space mean | −0.0 | 1.216 |
| 16 | 20 | 0.5 | none | 20% trimmed mean | −0.7 | 1.076 |
| 16 | 20 | 0.5 | none | Huber-M | −0.5 | 1.036 |
| 64 | 20 | 0.5 | none | Grand mean | +0.0 | 1.000 |
| 64 | 20 | 0.5 | none | Log-space mean | −0.0 | 1.215 |
| 64 | 20 | 0.5 | none | 20% trimmed mean | −0.2 | 1.070 |
| 64 | 20 | 0.5 | none | Huber-M | −0.1 | 1.034 |
| 1 | 20 | 1.5 | none | Grand mean | +0.2 | 1.000 |
| 1 | 20 | 1.5 | none | Log-space mean | +32.6 | 2.656 |
| 1 | 20 | 1.5 | none | 20% trimmed mean | −55.3 | 0.850 |
| 1 | 20 | 1.5 | none | Huber-M | −53.7 | 0.833 |
| 4 | 20 | 1.5 | none | Grand mean | +0.0 | 1.000 |
| 4 | 20 | 1.5 | none | Log-space mean | −3.0 | 1.060 |
| 4 | 20 | 1.5 | none | 20% trimmed mean | −29.6 | 1.008 |
| 4 | 20 | 1.5 | none | Huber-M | −26.2 | 0.939 |
| 16 | 20 | 1.5 | none | Grand mean | +0.0 | 1.000 |
| 16 | 20 | 1.5 | none | Log-space mean | −4.2 | 1.019 |
| 16 | 20 | 1.5 | none | 20% trimmed mean | −13.3 | 1.057 |
| 16 | 20 | 1.5 | none | Huber-M | −11.0 | 0.977 |
| 64 | 20 | 1.5 | none | Grand mean | −0.0 | 1.000 |
| 64 | 20 | 1.5 | none | Log-space mean | −2.4 | 1.078 |
| 64 | 20 | 1.5 | none | 20% trimmed mean | −5.3 | 1.061 |
| 64 | 20 | 1.5 | none | Huber-M | −4.2 | 0.994 |
| 1 | 20 | 0.5 | 10% | Grand mean | +90.1 | 1.000 |
| 1 | 20 | 0.5 | 10% | Log-space mean | +12.8 | 0.240 |
| 1 | 20 | 0.5 | 10% | 20% trimmed mean | +0.1 | 0.129 |
| 1 | 20 | 0.5 | 10% | Huber-M | +3.4 | 0.141 |
| 4 | 20 | 0.5 | 10% | Grand mean | +90.0 | 1.000 |
| 4 | 20 | 0.5 | 10% | Log-space mean | +4.9 | 0.106 |
| 4 | 20 | 0.5 | 10% | 20% trimmed mean | +1.9 | 0.075 |
| 4 | 20 | 0.5 | 10% | Huber-M | +3.4 | 0.082 |
| 16 | 20 | 0.5 | 10% | Grand mean | +90.0 | 1.000 |
| 16 | 20 | 0.5 | 10% | Log-space mean | +2.1 | 0.050 |
| 16 | 20 | 0.5 | 10% | 20% trimmed mean | +1.5 | 0.041 |
| 16 | 20 | 0.5 | 10% | Huber-M | +2.1 | 0.044 |
| 64 | 20 | 0.5 | 10% | Grand mean | +90.0 | 1.000 |
| 64 | 20 | 0.5 | 10% | Log-space mean | +1.0 | 0.024 |
| 64 | 20 | 0.5 | 10% | 20% trimmed mean | +0.9 | 0.021 |
| 64 | 20 | 0.5 | 10% | Huber-M | +1.2 | 0.022 |
| 1 | 20 | 1.5 | 10% | Grand mean | +90.6 | 1.000 |
| 1 | 20 | 1.5 | 10% | Log-space mean | +106.9 | 1.764 |
| 1 | 20 | 1.5 | 10% | 20% trimmed mean | −43.7 | 0.214 |
| 1 | 20 | 1.5 | 10% | Huber-M | −43.2 | 0.214 |
| 4 | 20 | 1.5 | 10% | Grand mean | +89.9 | 1.000 |
| 4 | 20 | 1.5 | 10% | Log-space mean | +26.7 | 0.478 |
| 4 | 20 | 1.5 | 10% | 20% trimmed mean | −16.1 | 0.189 |
| 4 | 20 | 1.5 | 10% | Huber-M | −12.6 | 0.184 |
| 16 | 20 | 1.5 | 10% | Grand mean | +89.8 | 1.000 |
| 16 | 20 | 1.5 | 10% | Log-space mean | +8.5 | 0.229 |
| 16 | 20 | 1.5 | 10% | 20% trimmed mean | −4.0 | 0.136 |
| 16 | 20 | 1.5 | 10% | Huber-M | −0.8 | 0.139 |
| 64 | 20 | 1.5 | 10% | Grand mean | +90.1 | 1.000 |
| 64 | 20 | 1.5 | 10% | Log-space mean | +3.3 | 0.114 |
| 64 | 20 | 1.5 | 10% | 20% trimmed mean | −0.0 | 0.083 |
| 64 | 20 | 1.5 | 10% | Huber-M | +1.9 | 0.089 |
| 1 | 100 | 1.5 | none | Grand mean | +0.1 | 1.000 |
| 1 | 100 | 1.5 | none | Log-space mean | +5.4 | 1.253 |
| 1 | 100 | 1.5 | none | 20% trimmed mean | −58.3 | 1.978 |
| 1 | 100 | 1.5 | none | Huber-M | −55.4 | 1.884 |
| 16 | 100 | 1.5 | none | Grand mean | +0.0 | 1.000 |
| 16 | 100 | 1.5 | none | Log-space mean | −5.2 | 1.224 |
| 16 | 100 | 1.5 | none | 20% trimmed mean | −14.2 | 2.069 |
| 16 | 100 | 1.5 | none | Huber-M | −11.4 | 1.725 |
| 1 | 100 | 1.5 | 10% | Grand mean | +89.8 | 1.000 |
| 1 | 100 | 1.5 | 10% | Log-space mean | +53.2 | 0.624 |
| 1 | 100 | 1.5 | 10% | 20% trimmed mean | −47.7 | 0.374 |
| 1 | 100 | 1.5 | 10% | Huber-M | −45.2 | 0.357 |
| 16 | 100 | 1.5 | 10% | Grand mean | +90.0 | 1.000 |
| 16 | 100 | 1.5 | 10% | Log-space mean | +6.8 | 0.126 |
| 16 | 100 | 1.5 | 10% | 20% trimmed mean | −5.1 | 0.084 |
| 16 | 100 | 1.5 | 10% | Huber-M | −1.2 | 0.070 |

**Key observations:**

1. The grand mean is unbiased at all $(k, g, \sigma)$ on clean data, but is completely destroyed by 10% contamination ($+90\%$) regardless of $k$ or $\sigma$.
2. The log-space mean (with MAD) is the best all-rounder for $k \geq 4$: bias 2–9% even with contamination. The large upward bias at $k=1$, $\sigma=1.5$ ($+33\%$ at $g=20$, falling to $+5\%$ at $g=100$) is substantially reduced by using $Q_n$ instead of MAD.
3. **The trimmed mean and Huber-M are badly biased for the mean when $Y$ is skewed** ($-55\%$ at $k=1$, $\sigma=1.5$), and this does not improve with larger $g$ (see $g=100$ rows). They are only appropriate when $\hat{S} \leq 0.3$.
4. No single estimator dominates all regimes. The log-space mean with $Q_n$ scale is the best all-rounder when $k \geq 4$.

### Table B.2 — Estimators for the Median of X

| $k$ | $g$ | $\sigma$ | Contam | Estimator | Bias% | Rel-RMSE |
|--:|--:|--:|--------|-----------|------:|---------:|
| 1 | 20 | 0.5 | none | Geometric mean | +0.7 | 1.000 |
| 1 | 20 | 0.5 | none | Log-space median | +1.0 | 1.219 |
| 1 | 20 | 0.5 | none | Sample median | +1.1 | 1.221 |
| 1 | 20 | 0.5 | none | Classical BT | +1.1 | 1.221 |
| 4 | 20 | 0.5 | none | Geometric mean | +9.7 | 1.000 |
| 4 | 20 | 0.5 | none | Log-space median | +0.7 | 0.693 |
| 4 | 20 | 0.5 | none | Sample median | +9.6 | 1.063 |
| 4 | 20 | 0.5 | none | Classical BT | +0.3 | 0.556 |
| 16 | 20 | 0.5 | none | Geometric mean | +12.4 | 1.000 |
| 16 | 20 | 0.5 | none | Log-space median | +0.7 | 0.478 |
| 16 | 20 | 0.5 | none | Sample median | +12.3 | 1.013 |
| 16 | 20 | 0.5 | none | Classical BT | +0.2 | 0.346 |
| 64 | 20 | 0.5 | none | Geometric mean | +13.1 | 1.000 |
| 64 | 20 | 0.5 | none | Log-space median | +0.6 | 0.425 |
| 64 | 20 | 0.5 | none | Sample median | +13.1 | 1.003 |
| 64 | 20 | 0.5 | none | Classical BT | +0.2 | 0.286 |
| 1 | 20 | 1.5 | none | Geometric mean | +5.9 | 1.000 |
| 1 | 20 | 1.5 | none | Log-space median | +8.8 | 1.269 |
| 1 | 20 | 1.5 | none | Sample median | +9.7 | 1.285 |
| 1 | 20 | 1.5 | none | Classical BT | +9.7 | 1.285 |
| 4 | 20 | 1.5 | none | Geometric mean | +101.8 | 1.000 |
| 4 | 20 | 1.5 | none | Log-space median | +26.5 | 0.384 |
| 4 | 20 | 1.5 | none | Sample median | +98.2 | 0.997 |
| 4 | 20 | 1.5 | none | Classical BT | +35.1 | 0.429 |
| 16 | 20 | 1.5 | none | Geometric mean | +164.7 | 1.000 |
| 16 | 20 | 1.5 | none | Log-space median | +28.7 | 0.238 |
| 16 | 20 | 1.5 | none | Sample median | +158.0 | 0.967 |
| 16 | 20 | 1.5 | none | Classical BT | +21.8 | 0.193 |
| 64 | 20 | 1.5 | none | Geometric mean | +193.3 | 1.000 |
| 64 | 20 | 1.5 | none | Log-space median | +24.4 | 0.188 |
| 64 | 20 | 1.5 | none | Sample median | +188.1 | 0.975 |
| 64 | 20 | 1.5 | none | Classical BT | +13.4 | 0.131 |
| 1 | 20 | 0.5 | 10% | Geometric mean | +26.7 | 1.000 |
| 1 | 20 | 0.5 | 10% | Log-space median | +8.3 | 0.581 |
| 1 | 20 | 0.5 | 10% | Sample median | +8.4 | 0.583 |
| 1 | 20 | 0.5 | 10% | Classical BT | +8.4 | 0.583 |
| 4 | 20 | 0.5 | 10% | Geometric mean | +38.1 | 1.000 |
| 4 | 20 | 0.5 | 10% | Log-space median | +2.0 | 0.224 |
| 4 | 20 | 0.5 | 10% | Sample median | +13.7 | 0.414 |
| 4 | 20 | 0.5 | 10% | Classical BT | −31.0 | 0.805 |
| 16 | 20 | 0.5 | 10% | Geometric mean | +41.5 | 1.000 |
| 16 | 20 | 0.5 | 10% | Log-space median | −0.5 | 0.162 |
| 16 | 20 | 0.5 | 10% | Sample median | +14.4 | 0.361 |
| 16 | 20 | 0.5 | 10% | Classical BT | −63.7 | 1.530 |
| 64 | 20 | 0.5 | 10% | Geometric mean | +42.4 | 1.000 |
| 64 | 20 | 0.5 | 10% | Log-space median | −1.6 | 0.153 |
| 64 | 20 | 0.5 | 10% | Sample median | +14.1 | 0.337 |
| 64 | 20 | 0.5 | 10% | Classical BT | −81.6 | 1.925 |
| 1 | 20 | 1.5 | 10% | Geometric mean | +33.2 | 1.000 |
| 1 | 20 | 1.5 | 10% | Log-space median | +29.9 | 1.130 |
| 1 | 20 | 1.5 | 10% | Sample median | +31.2 | 1.149 |
| 1 | 20 | 1.5 | 10% | Classical BT | +31.2 | 1.149 |
| 4 | 20 | 1.5 | 10% | Geometric mean | +154.1 | 1.000 |
| 4 | 20 | 1.5 | 10% | Log-space median | +35.9 | 0.314 |
| 4 | 20 | 1.5 | 10% | Sample median | +124.8 | 0.847 |
| 4 | 20 | 1.5 | 10% | Classical BT | +64.7 | 0.490 |
| 16 | 20 | 1.5 | 10% | Geometric mean | +233.1 | 1.000 |
| 16 | 20 | 1.5 | 10% | Log-space median | +25.2 | 0.155 |
| 16 | 20 | 1.5 | 10% | Sample median | +177.9 | 0.773 |
| 16 | 20 | 1.5 | 10% | Classical BT | −5.2 | 0.073 |
| 64 | 20 | 1.5 | 10% | Geometric mean | +269.3 | 1.000 |
| 64 | 20 | 1.5 | 10% | Log-space median | +14.9 | 0.109 |
| 64 | 20 | 1.5 | 10% | Sample median | +200.2 | 0.746 |
| 64 | 20 | 1.5 | 10% | Classical BT | −50.8 | 0.189 |
| 1 | 100 | 1.5 | none | Geometric mean | +1.1 | 1.000 |
| 1 | 100 | 1.5 | none | Log-space median | +1.9 | 1.267 |
| 1 | 100 | 1.5 | none | Sample median | +1.9 | 1.268 |
| 1 | 100 | 1.5 | none | Classical BT | +1.9 | 1.268 |
| 4 | 100 | 1.5 | none | Geometric mean | +98.6 | 1.000 |
| 4 | 100 | 1.5 | none | Log-space median | +19.9 | 0.243 |
| 4 | 100 | 1.5 | none | Sample median | +93.0 | 0.952 |
| 4 | 100 | 1.5 | none | Classical BT | +18.2 | 0.263 |
| 16 | 100 | 1.5 | none | Geometric mean | +163.2 | 1.000 |
| 16 | 100 | 1.5 | none | Log-space median | +21.7 | 0.151 |
| 16 | 100 | 1.5 | none | Sample median | +155.4 | 0.954 |
| 16 | 100 | 1.5 | none | Classical BT | +10.1 | 0.116 |
| 1 | 100 | 1.5 | 10% | Geometric mean | +27.3 | 1.000 |
| 1 | 100 | 1.5 | 10% | Log-space median | +21.6 | 0.959 |
| 1 | 100 | 1.5 | 10% | Sample median | +21.6 | 0.960 |
| 1 | 100 | 1.5 | 10% | Classical BT | +21.6 | 0.960 |
| 4 | 100 | 1.5 | 10% | Geometric mean | +150.0 | 1.000 |
| 4 | 100 | 1.5 | 10% | Log-space median | +28.7 | 0.214 |
| 4 | 100 | 1.5 | 10% | Sample median | +118.3 | 0.798 |
| 4 | 100 | 1.5 | 10% | Classical BT | +34.1 | 0.267 |
| 16 | 100 | 1.5 | 10% | Geometric mean | +231.3 | 1.000 |
| 16 | 100 | 1.5 | 10% | Log-space median | +18.4 | 0.093 |
| 16 | 100 | 1.5 | 10% | Sample median | +174.7 | 0.757 |
| 16 | 100 | 1.5 | 10% | Classical BT | −15.2 | 0.077 |

**Key observations:**

1. At $k=1$ with clean data, the geometric mean (MLE) is most efficient; the log-space median and sample median are close but somewhat less efficient.
2. For $k > 1$, $\sigma = 0.5$, clean: the classical BT has the lowest RMSE; the log-space median is next.
3. For $k > 1$, $\sigma = 1.5$: all estimators carry residual bias of 15–30% (log-space median) or 14–35% (classical BT), persisting even at $g = 100$. **No back-transform is reliable for heavy-tailed distributions with large $k$.** Prefer $k = k_{\min}$ and collect more groups $g$.
4. Under contamination with $k > 1$: the classical BT fails catastrophically (up to $-82\%$); the log-space median is the only consistently reliable option.
5. The sample median of $Y$ is always biased for $k > 1$ and should not be used as a median-of-$X$ estimator.

---

## References

1. Fenton, L. F. (1960). The sum of log-normal probability distributions in scatter transmission systems. *IRE Transactions on Communications Systems*, 8(1), 57–67. [DOI: 10.1109/TCOM.1960.1097606](https://doi.org/10.1109/TCOM.1960.1097606). Wikipedia: [Fenton–Wilkinson method](https://en.wikipedia.org/wiki/Fenton%E2%80%93Wilkinson_method).

2. Schwartz, S. C., & Yeh, Y. S. (1982). On the distribution function and moments of power sums with log-normal components. *Bell System Technical Journal*, 61(7), 1441–1462. [DOI: 10.1002/j.1538-7305.1982.tb04353.x](https://doi.org/10.1002/j.1538-7305.1982.tb04353.x).

3. Central Limit Theorem. Wikipedia: [Central limit theorem](https://en.wikipedia.org/wiki/Central_limit_theorem).

4. Breakdown point. Wikipedia: [Robust statistics — breakdown point](https://en.wikipedia.org/wiki/Robust_statistics#Breakdown_point).

5. Monahan, J. F. (1984). Algorithm 616: Fast computation of the Hodges–Lehmann location estimator. *ACM Transactions on Mathematical Software*, 10(3), 265–270. Wikipedia: [Truncated mean](https://en.wikipedia.org/wiki/Truncated_mean). Wilcox, R. R. (2012). *Introduction to Robust Estimation and Hypothesis Testing*, 3rd ed. Academic Press.

6. Finney, D. J. (1941). On the distribution of a variate whose logarithm is normally distributed. *Supplement to the Journal of the Royal Statistical Society*, 7(2), 155–161. [DOI: 10.2307/2983663](https://doi.org/10.2307/2983663). Wikipedia: [Geometric mean](https://en.wikipedia.org/wiki/Geometric_mean).

7. Median Absolute Deviation. Wikipedia: [Median absolute deviation](https://en.wikipedia.org/wiki/Median_absolute_deviation).

8. Winsorized mean. Wikipedia: [Winsorized mean](https://en.wikipedia.org/wiki/Winsorized_mean).

9. Huber, P. J., & Ronchetti, E. M. (2009). *Robust Statistics*, 2nd ed. Wiley. Wikipedia: [M-estimator](https://en.wikipedia.org/wiki/M-estimator).

10. Rousseeuw, P. J., & Croux, C. (1993). Alternatives to the median absolute deviation. *Journal of the American Statistical Association*, 88(424), 1273–1283. [DOI: 10.1080/01621459.1993.10476408](https://doi.org/10.1080/01621459.1993.10476408). Wikipedia: [Rousseeuw–Croux estimators](https://en.wikipedia.org/wiki/Rousseeuw%E2%80%93Croux_estimators).

11. Hampel, F. R., Ronchetti, E. M., Rousseeuw, P. J., & Stahel, W. A. (1986). *Robust Statistics: The Approach Based on Influence Functions*. Wiley.

12. Delta method. Wikipedia: [Delta method](https://en.wikipedia.org/wiki/Delta_method).

13. Jensen's inequality. Wikipedia: [Jensen's inequality](https://en.wikipedia.org/wiki/Jensen%27s_inequality).

14. Lognormal distribution. Wikipedia: [Log-normal distribution](https://en.wikipedia.org/wiki/Log-normal_distribution).

15. Coefficient of variation. Wikipedia: [Coefficient of variation](https://en.wikipedia.org/wiki/Coefficient_of_variation).

16. Mean squared error. Wikipedia: [Mean squared error](https://en.wikipedia.org/wiki/Mean_squared_error).
