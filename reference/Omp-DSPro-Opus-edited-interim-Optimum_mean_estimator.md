# Optimum Mean Estimator Under Batching of Lognormal Latency Data

*This document analyses estimators of the population **mean**
$\mathbb{E}[X] = e^{\mu+\sigma^2/2}$ from batched latency measurements Y. Batching does **not** hide the mean — the
grand mean $m_y$ is exactly $\bar{X}$ and is unbiased at every $k$. The estimation
problem is therefore about **robustness and efficiency**, not about recovering a lost
quantity.*

---

## 1. Setup and Notation

Let $X_1, \ldots, X_n \sim \text{Lognormal}(\mu, \sigma^2)$ be IID. Population quantities:

| Quantity | Expression |
|----------|-----------|
| **Mean** (the target $\theta$) | $\mathbb{E}[X] = \exp(\mu + \sigma^2/2)$ |
| **Median** | $\exp(\mu)$ |
| **Variance** | $\text{Var}(X) = \exp(2\mu + \sigma^2)(\exp(\sigma^2) - 1)$ |
| **Squared CV** (coefficient of variation) | $\text{CV}^2(X) = \exp(\sigma^2) - 1$ |
| **Log-scale** | $\mathbb{E}[\ln X] = \mu,\quad \text{Var}(\ln X) = \sigma^2$ |

Let $k \mid n$ ($k$ divides $n$). Partition the observations into $g = n/k$ groups of size $k$ and form the group
means, the only quantities we observe:

$$Y_j = \frac{1}{k} \sum_{i \in \text{group } j} X_i, \qquad j = 1, \ldots, g.$$

From the $Y_j$ compute:

$$
m_y = \frac{1}{g}\sum_{j} Y_j, \qquad
s_y^2 = \frac{1}{g-1}\sum_{j}(Y_j - m_y)^2, \qquad
\text{ml}_y = \frac{1}{g}\sum_{j} \ln(Y_j),
$$

and the derived quantities

$$
\text{var}_x = k \cdot s_y^2, \qquad
\hat{\sigma}^2 = \ln\!\Big(1 + \frac{\text{var}_x}{m_y^2}\Big), \qquad
\hat{\sigma}^2_w = \ln\!\Big(1 + \frac{e^{\hat{\sigma}^2} - 1}{k}\Big).
$$

Here $\hat{\sigma}^2$ estimates the log-scale variance $\sigma^2 = \text{Var}(\ln X)$, while
$\hat{\sigma}^2_w$ — the **Fenton–Wilkinson log-variance of $Y_j$** — estimates
$\text{Var}(\ln Y_j) \approx \ln(1 + (e^{\sigma^2}-1)/k)$ under the Fenton–Wilkinson
approximation (the sum of $k$ IID lognormals is approximately lognormal with the same first
two moments; see ref. 1 — Wilkinson's contribution circulated informally and is traditionally
credited in the name). At $k=1$, $\hat{\sigma}^2_w = \hat{\sigma}^2$; as $k \to \infty$,
$\hat{\sigma}^2_w \to 0$.

**Exact moment identities (hold for every $k$, by linearity and within-group independence):**

$$
\mathbb{E}[Y_j] = \mathbb{E}[X] = e^{\mu+\sigma^2/2}, \qquad
\text{Var}(Y_j) = \frac{\text{Var}(X)}{k}, \qquad
\text{CV}^2(Y_j) = \frac{e^{\sigma^2}-1}{k}.
$$

Consequently $m_y = \bar{X}_n$ exactly (the grand mean of all $n$ observations), and —
writing $\xrightarrow{p}$ for convergence in probability, with $\operatorname{plim}$ its probability limit —

$$
m_y \xrightarrow{p} e^{\mu+\sigma^2/2}, \quad
\text{var}_x \xrightarrow{p} \text{Var}(X), \quad
\hat{\sigma}^2 \xrightarrow{p} \sigma^2, \quad
\hat{\sigma}^2_w \xrightarrow{p} \ln\!\Big(1 + \frac{e^{\sigma^2}-1}{k}\Big).
$$

### The four candidate estimators (target $\theta = e^{\mu+\sigma^2/2}$)

| # | Name | Formula |
|---|------|---------|
| 1 | Sample mean (grand mean) | $\hat\theta_1 = m_y = \bar{X}$ |
| 2 | Corrected geometric mean (FW) | $\hat\theta_2 = \exp(\text{ml}_y + \hat\sigma^2_w/2)$ |
| 3 | Corrected median | $\hat\theta_3 = \text{median}(Y_1,\ldots,Y_g) \cdot \exp(\hat\sigma^2_w/2)$ |
| 4 | Trimmed / winsorized mean of $Y_j$ | $\hat\theta_4 = \text{trimmed- or winsorized-mean}(Y_1,\ldots,Y_g)$ |

---

## 2. Why Batching Does Not Hide the Mean (the crux)

**This is the fundamental structural difference from median estimation.** For the median,
batching hides $e^\mu$ because $Y_j$ converges to $\mathbb{E}[X]$, not to $e^\mu$. For the
mean, the batch mean $Y_j$ already carries $\mathbb{E}[X]$ as its expectation:

$$\mathbb{E}[Y_j] = \mathbb{E}[X]\quad\text{for every }k.$$

The grand mean $m_y$ is therefore **exactly unbiased** at every $k$, and no model is needed
to recover the target from $m_y$. The role of the lognormal model is limited to (a) improving
efficiency at small $k$ and (b) enabling robust alternatives that correct a robust location
(median, trimmed mean) back to the mean.

**Implication.** For $k \ge 1$, *every* estimator considered here is consistent for
$\mathbb{E}[X]$ — the sample mean by construction, the corrected estimators by design. The
honest goals are (a) robustness to outliers and departures from log-normality, and
(b) efficiency, especially at small $k$ where the sample mean is suboptimal for lognormal data.

**Two independent axes vary with $k$:**

- **Efficiency of $m_y$ relative to the lognormal MLE** (estimator 2 at $k=1$) depends only
  on $\sigma^2$, not on $k$ — the MLE's advantage grows with $\sigma^2$.
- **Robustness of corrected estimators** improves as $k$ grows: $Y_j$ becomes more symmetric
  (CLT), so robust location estimators (median, trimmed mean) become unbiased for
  $\mathbb{E}[X]$ *without* correction, and the correction factor $\to 1$.

---

## 3. Distribution of $Y_j$ as $k$ Varies

| Regime | Shape of $Y_j$ | Groups $g=n/k$ |
|--------|----------------|----------------|
| $k=1$ | Exactly $\text{Lognormal}(\mu,\sigma^2)$; $\ln Y_j\sim N(\mu,\sigma^2)$ | $g=n$ (many) |
| $k$ small (FW holds) | Right-skewed, less so than $X$; well approximated as lognormal (Fenton–Wilkinson) | many |
| $k$ intermediate | Transition: FW degrading, CLT emerging — neither lognormal nor normal | moderate |
| $k$ large (CLT dominant) | Approximately $N\!\big(\mathbb{E}[X],\ \text{Var}(X)/k\big)$ | few |
| $k=n$ | Single value $Y_1=\bar X_n$; $m_y$ still unbiased but no variance estimate possible | $g=1$ |

The regime boundaries are determined by $k$ and $\sigma^2$ — **not** by $n$. The shape of
$Y_j$ depends only on $\text{CV}(Y_j) = \sqrt{(e^{\sigma^2}-1)/k}$, which is a function of
$k$ and $\sigma^2$ alone. "Small $k$" means the FW lognormal approximation is accurate —
typically $k \lesssim 10$–$20$, depending on $\sigma^2$ (the approximation degrades faster
for larger $\sigma^2$). "Large $k$" means the CLT has made $Y_j$ approximately normal —
roughly when $\text{CV}(Y_j) \lesssim 0.2$, i.e., $k \gtrsim 25(e^{\sigma^2}-1)$. The
number of groups $g = n/k$ (rightmost column) is a separate axis that governs estimation
variance, not the shape of $Y_j$.

The **Fenton–Wilkinson approximation**, invoked at several points below, models the sum (or mean) of
a few IID lognormals as *itself* lognormal with the same first two moments; it is accurate for small
$k$ and degrades as $Y_j$ turns Gaussian. Under FW:

$$Y_j \overset{\text{approx}}{\sim} \text{Lognormal}\!\big(\mu_w,\ \sigma^2_w\big),\qquad
\mu_w = \mu + \frac{\sigma^2 - \sigma^2_w}{2},\qquad
\sigma^2_w = \ln\!\Big(1 + \frac{e^{\sigma^2} - 1}{k}\Big).$$

This parameterisation is chosen so that $\exp(\mu_w + \sigma^2_w/2) = \exp(\mu + \sigma^2/2) =
\mathbb{E}[X]$ exactly.

A useful drift formula (**delta method** — a first-order Taylor expansion of $\ln(\cdot)$ about
$\mathbb{E}[Y_j]$, valid once $Y_j$ concentrates):

$$\mathbb{E}[\ln Y_j] \approx \mu + \frac{\sigma^2}{2} - \frac{e^{\sigma^2}-1}{2k}.$$

At $k=1$ the exact value is $\mathbb{E}[\ln Y_1] = \mu$, not $\mu + \sigma^2/2 - (e^{\sigma^2}-1)/2$;
the delta method is inaccurate there unless $\sigma^2 \ll 1$. As $k\to\infty$,
$\mathbb{E}[\ln Y_j] \to \mu + \sigma^2/2 = \ln\mathbb{E}[X]$, so the geometric mean
$\exp(\text{ml}_y)$ converges to the target $\mathbb{E}[X]$.

---

## 4. Analysis of the Four Estimators

### 4.1 Estimator 1 — $\hat\theta_1 = m_y$ (sample mean)

**Unbiased at every $k$:** $\mathbb{E}[m_y] = \mathbb{E}[Y_j] = \mathbb{E}[X] =
e^{\mu+\sigma^2/2}$, by linearity of expectation. This is the most natural estimator and the
baseline against which all alternatives are judged.
**Efficiency at $k=1$ (relative to the lognormal MLE):** for lognormal data, the sample mean
is not fully efficient. The asymptotic variance of the MLE (estimator 2 at $k=1$, §4.2) is
smaller than $\text{Var}(X)/n$ for $\sigma^2 > 0$. The efficiency gap grows with $\sigma^2$;
for typical latency benchmarks ($\sigma \approx 0.1$–$0.5$), the sample mean's ARE vs. the MLE
is approximately $0.99$–$1.00$ — a loss of at most 1%. At $\sigma = 1.0$ the ARE is $\approx
0.87$; at $\sigma = 1.5$ it is $\approx 0.56$; at $\sigma = 2.0$ it is $\approx 0.22$.
(These values follow from the CRLB ratio $\text{ARE} = (\sigma^2 + \sigma^4/2)/(e^{\sigma^2}-1)$.)
The key practical point: **for the $\sigma$ range encountered in latency benchmarking,
$m_y$ is essentially fully efficient.**

As $k$ grows, $Y_j$ becomes less skewed (CLT), and the MLE's advantage over $m_y$ shrinks
further — at large $k$, $Y_j$ is near-normal and $m_y$ is approximately the MLE.

**Robustness:** poor. The sample mean has **breakdown point 0** — a single extreme $Y_j$ (or
a single extreme $X_i$ at $k=1$) can drive it arbitrarily far. This is the primary motivation
for the robust alternatives below, especially when measurements may contain outliers (cold-start
effects, GC pauses, scheduler interference).

**At $k=n$ (degenerate):** $m_y = \bar{X}_n$ is still unbiased for $\mathbb{E}[X]$, but no
internal estimate of $\text{Var}(X)$ or $\sigma^2$ is available ($s_y^2$ is undefined).
External calibration or a pilot run is needed to estimate the variance.

### 4.2 Estimator 2 — $\hat\theta_2 = \exp(\text{ml}_y + \hat\sigma^2_w/2)$ (corrected geometric mean)

**$k=1$:** $\hat\sigma^2_w = \hat\sigma^2$, so $\hat\theta_2 = \exp(\overline{\ln X} +
s^2_{\ln X}/2)$ — the lognormal **maximum-likelihood estimator (MLE)** of the mean.
(*Note:* the formula uses the unbiased sample variance $s^2_{\ln X}$ with divisor $n-1$; the
exact MLE of $\sigma^2$ uses divisor $n$; the two are asymptotically equivalent and the
difference is negligible for $n > 30$.)
The MLE works by estimating the lognormal parameters from the log-transformed data
($\hat\mu = \overline{\ln X}$, the sample mean of $\ln X_i$; $\hat\sigma^2 = s^2_{\ln X}$,
the sample variance of $\ln X_i$) and plugging them into the mean formula
$\mathbb{E}[X] = \exp(\mu + \sigma^2/2)$. Because the $\ln X_i$ are exactly normal,
these parameter estimates are fully efficient, making the MLE more efficient than the
raw sample mean $m_y$.
Consistent and asymptotically efficient (attains the Cramér–Rao lower bound
$(\sigma^2+\sigma^4/2)/n$ relative to $\theta^2$, which is less than the sample mean's
$(e^{\sigma^2}-1)/n$). Finite-sample
bias $\mathbb{E}[\hat\theta_2] \approx e^{\mu+\sigma^2/2} \cdot
\exp(\frac{\sigma^4+2\sigma^2}{4n})$ (upward, $O(1/n)$; Finney 1941). For typical
benchmark sample sizes ($n \gg 100$), this bias is negligible. For small samples
($n < 30$) where the MLE bias may matter, the **UMVUE** (estimator E, §5) removes it
entirely — it is exactly unbiased at every $n$ and minimum-variance among unbiased
estimators.

**$k>1$:** the geometric mean $\exp(\text{ml}_y)$ alone targets $e^{\mathbb{E}[\ln Y_j]}$,
which drifts from $e^\mu$ (the median) at $k=1$ toward $\mathbb{E}[X]$ as $k\to\infty$.
The correction $\exp(\hat\sigma^2_w/2)$ bridges the gap — under the Fenton–Wilkinson
approximation, $\exp(\mathbb{E}[\ln Y_j] + \sigma^2_w/2) = \mathbb{E}[X]$ exactly.
Thus $\hat\theta_2$ is approximately unbiased for all $k$, with the FW approximation error
as the only source of systematic bias.
**Variance — a saturation result.** With $\text{Var}(\ln Y_j) \approx \sigma^2_w$ (FW),

$$\text{Var}(\text{ml}_y) = \frac{\text{Var}(\ln Y_j)}{g} \approx \frac{k}{n} \cdot
\ln\!\Big(1 + \frac{e^{\sigma^2}-1}{k}\Big),$$

which rises **monotonically** from $\sigma^2/n$ (at $k=1$) and **saturates** at
$(e^{\sigma^2}-1)/n$ — it does *not* grow linearly in $k$. The additional variance from
estimating $\hat\sigma^2_w$ (via $\hat\sigma^2$) grows with $k$ as $g$ shrinks — because
$s_y^2$ has relative standard error $\approx\sqrt{2/(g-1)}$ and is then multiplied by $k$
in $\text{var}_x = k s_y^2$, amplifying its sampling error into $\hat\sigma^2$. At large
$k$ (small $g$), this **$\hat\sigma^2$ estimation error** dominates the total variance.

**Robustness:** mixed. The log transform tames the right tail of $Y_j$ (extreme large values
become moderate $\ln Y_j$), so $\text{ml}_y$ is less sensitive to large outliers than $m_y$.
However, the log transform **amplifies the left tail**: a single $Y_j$ near zero produces a
large negative $\ln Y_j$ that drags $\text{ml}_y$ down. Breakdown point is **0** (a single
near-zero batch, or a single zero $X_i$ at $k=1$, breaks it). The scale channel
$\hat\sigma^2_w$ inherits the fragility of $\hat\sigma^2$ (breakdown 0 via $s_y^2$).
Replacing $s_y^2$ with a robust scale (§5) partially mitigates this.

### 4.3 Estimator 3 — $\hat\theta_3 = \text{median}(Y_j) \cdot \exp(\hat\sigma^2_w/2)$ (corrected median)

**$k=1$:** $\text{median}(X) = e^\mu$, so $\hat\theta_3 = e^\mu \cdot \exp(\hat\sigma^2/2)$:
the sample median scaled to the mean under log-normality. Consistent and model-dependent
(assumes log-normality for the correction factor). Asymptotic variance
$\frac{\pi}{2} \cdot \frac{\sigma^2 e^{2\mu}}{n}$ (from the median) plus $O(1/n)$ from
$\hat\sigma^2$, giving an ARE of approximately $2/\pi \approx 0.64$ vs the MLE at $k=1$
(ignoring $\hat\sigma^2$ variance). **Robust location** (≈50% breakdown on $Y_j$),
**fragile scale** (breakdown 0 via $\hat\sigma^2$).

**$k>1$:** as $k$ grows, $\text{median}(Y_j)$ drifts from $e^\mu$ toward $\mathbb{E}[X]$
(by the CLT), and the correction $\exp(\hat\sigma^2_w/2)$ shrinks toward $1$. The estimator
remains consistent for $\mathbb{E}[X]$. Under the FW approximation,
$\text{median}(Y_j) \approx \exp(\mu_w) = \exp(\mu + (\sigma^2 - \sigma^2_w)/2)$, so
$\hat\theta_3 \approx \exp(\mu_w + \sigma^2_w/2) = \mathbb{E}[X]$.

**Bias direction.** The corrected mean $\hat\theta_3$ has bias from two sources that
partially offset:
- $\text{median}(Y_j) \le \mathbb{E}[Y_j] = \mathbb{E}[X]$ for right-skewed $Y_j$ (small $k$):
  **negative bias in the location.**
- The FW approximation error plus $\hat\sigma^2$ estimation error can go either way.
At large $k$ (near-normal $Y_j$), both bias sources vanish.

**Robustness — its key strength:** the median location gives ≈50% breakdown against arbitrary
$Y_j$ contamination — the best achievable. The scale channel $\hat\sigma^2$ (via $s_y^2$)
remains fragile, so an outlier batch inflates $\hat\sigma^2$ and biases $\hat\theta_3$ **upward**
(the opposite direction from estimator 2, where outlier deflation of $\text{ml}_y$ dominates).
Replacing $s_y^2$ with a robust scale (§5) delivers a fully robust estimator.

### 4.4 Estimator 4 — $\hat\theta_4 = \text{trimmed or winsorized mean of } Y_j$

**At $k=1$:** $Y_j = X_j \sim \text{Lognormal}(\mu,\sigma^2)$. The distribution is right-skewed,
so **symmetric trimming** (removing equal fractions from both tails) discards more genuine large
values than small ones, producing a downward bias relative to $\mathbb{E}[X]$. The bias grows
with the trim fraction $\alpha$ and with $\sigma^2$. **Asymmetric trimming** (e.g., trim only
the right tail, or different fractions from each tail) can reduce this bias at the cost of
introducing a tuning parameter. The winsorized mean (clamping extreme values rather than
discarding them) has similar properties with slightly better efficiency.

**At $k>1$ (moderate to large):** $Y_j$ becomes approximately normal (CLT). Symmetric
trimming is now **approximately unbiased** and provides bounded-influence protection against
outlier batches. A trim fraction of $\alpha = 0.10$–$0.20$ (10–20% from each tail) gives
good robustness while retaining good efficiency ($\approx 87$–$94\%$ Gaussian efficiency). The
winsorized mean at the same fractions has similar properties.

**Bias–variance trade-off.** Trimming discards data, so variance increases. The trade-off
is favourable when a small fraction of batches are expected to be contaminated:

| Trim fraction $\alpha$ | Gaussian efficiency | Breakdown point |
|------------------------|---------------------|----------------|
| 0 (sample mean) | 100% | 0 |
| 0.05 | ≈97% | 0.05 |
| 0.10 | ≈94% | 0.10 |
| 0.20 | ≈87% | 0.20 |
| 0.25 | ≈84% | 0.25 |

**Comparison to corrected median (estimator 3).** The corrected median has ≈50% breakdown
but lower efficiency (≈64% Gaussian ARE, i.e., asymptotic relative efficiency). The trimmed mean with $\alpha = 0.10$–$0.20$ offers
higher efficiency with moderate breakdown — a pragmatic middle ground when outliers are
infrequent but efficiency matters.

---

## 5. Additional Noteworthy Estimators

These build on or complement the primary four. Items A–D focus on robustness improvements;
item E (the UMVUE) addresses finite-sample optimality — a concern unique to the mean
estimation problem. Excluded proposals (at the end) lack sufficient robustness or
justification for this use case.
**A. Robustified estimator 3 — median location + robust scale.**
$\hat\theta_3^{\text{rob}} = \text{median}(Y_j) \cdot \exp(\tilde\sigma^2_w/2)$,
with $\tilde\sigma^2_w = \ln(1 + (e^{\tilde\sigma^2} - 1)/k)$ and $\tilde\sigma^2$ from a
robust scale estimator (item **C** below). Robust on **both** axes (≈50% breakdown location,
≈50% breakdown scale). The recommended robust estimator for all $k$ where a robust scale can
be reliably estimated ($g \ge 5$). At large $k$, $\exp(\tilde\sigma^2_w/2) \to 1$ and the
estimator reduces to $\text{median}(Y_j)$, which is naturally robust and approximately
unbiased — so the correction adds protection at small $k$ without harm at large $k$.

**B. Robustified estimator 2 — trimmed log-mean + robust scale.**
$\hat\theta_2^{\text{rob}} = \exp(\text{tm}_y + \tilde\sigma^2_w/2)$,
where $\text{tm}_y$ is a trimmed or winsorized mean of $\ln Y_j$, and $\tilde\sigma^2_w$
uses a robust scale. The trimmed log-mean provides bounded influence on both tails of
$\ln Y_j$ — left-tail amplification (the main weakness of estimator 2) is directly addressed.
At $k=1$, where $\ln Y_j$ is exactly normal, trimming costs little bias and the efficiency
loss is small. At $k>1$, the FW correction $\tilde\sigma^2_w/2$ removes the $\mathbb{E}[\ln Y]
\neq \mu$ drift (same mechanism as estimator 2). This is the **most efficient robust option**
at small $k$, where the corrected median's lower efficiency is a real cost.

**C. Robust-scale method of moments.** Throughout, $\tilde\sigma_y$ denotes a **robust
estimator of the standard deviation of the group means $Y_j$**. It feeds the induced robust
estimate of the lognormal shape, $\tilde\sigma^2 = \ln(1 + k\,\tilde\sigma_y^2/m_y^2)$.
Concrete high-breakdown choices for $\tilde\sigma_y$:

- **MAD:** $\tilde\sigma_y = 1.4826 \cdot \text{MAD}(Y)$, where
  $\text{MAD}(Y) = \text{median}_j\,|Y_j - \text{median}(Y)|$. ≈50% breakdown, but only ≈37%
  Gaussian efficiency; the $1.4826$ factor assumes near-normal $Y$ (accurate only for larger $k$).
- **Rousseeuw–Croux $Q_n$ (preferred) / $S_n$:** $\tilde\sigma_y = Q_n(Y)$ or $S_n(Y)$, the
  Rousseeuw–Croux robust scale estimators. Each is a Gaussian consistency constant **times** a
  robust spread of the pairwise absolute differences:
  $$Q_n = c_Q \cdot \big\{\,|Y_i - Y_j| : i<j\,\big\}_{(h)}, \qquad
    S_n = c_S \cdot \operatorname{median}_i \operatorname{median}_{j\ne i} |Y_i - Y_j|,$$
  with $c_Q \approx 2.2219$ and $c_S \approx 1.1926$. In $Q_n$, the subscript $(h)$ selects the
  $h$-th smallest value with $h = \binom{\lfloor g/2\rfloor + 1}{2}$. Both give ≈50% breakdown
  **and** ≈82% ($Q_n$) / ≈58% ($S_n$) Gaussian efficiency. Because they are built from pairwise
  differences $|Y_i - Y_j|$, **neither $Q_n$ nor $S_n$ requires a preliminary center (location)
  estimate** — unlike the MAD. Cost: $O(g\log g)$. The constants $c_Q, c_S$ are calibrated at
  the normal; for skewed $Y$ (small $k$) those same constants yield a mildly biased scale.

  > **IQR is dominated (correcting a common belief).** For any symmetric distribution
  > $\text{MAD} = \text{IQR}/2$ *exactly*, so — once each is rescaled by its consistency
  > constant — an IQR-based scale computes the **same population quantity** as the MAD, with
  > the *same* ≈37% Gaussian efficiency, while its breakdown point is only **25%** vs. the
  > MAD's **50%**. Use MAD or, better, $Q_n$; do not use an IQR-based scale.

**D. Log-scale MLE with trimmed/Winsorized log-data (at $k=1$ only).**
At $k=1$, $\hat\theta = \exp(\overline{\ln X}_{\text{trim}} + s^2_{\ln X,\text{trim}}/2)$,
where both the log-mean and log-variance are computed from trimmed/Winsorized $\ln X$.
This retains much of the MLE's efficiency while protecting against outliers in log-space.
Not recommended for $k>1$ because the FW correction interacts poorly with trimmed log-data
(the FW approximation was derived for full data).

**E. UMVUE — uniformly minimum variance unbiased estimator (Finney 1941; Shimizu & Iwase 1981, ref. 6).**
At $k=1$, there exists an **exactly unbiased** estimator of $\mathbb{E}[X] = e^{\mu+\sigma^2/2}$
that attains the minimum variance among *all* unbiased estimators — a stronger property
than the MLE's asymptotic efficiency. The UMVUE is

$$\hat\theta_{\text{UMVUE}} = \exp(\overline{\ln X}) \cdot \Psi\!\left(\frac{s^2_{\ln X}}{2},\ n\right),$$

where $\overline{\ln X} = \frac{1}{n}\sum \ln X_i$, $s^2_{\ln X} = \frac{1}{n-1}\sum (\ln X_i - \overline{\ln X})^2$, and
$\Psi(t, m)$ is the function (a confluent hypergeometric limit):

$$\Psi(t, m) = 1 + \frac{m-1}{m}t + \frac{(m-1)^3}{m^2 \cdot 2! \cdot (m+1)}t^2 + \frac{(m-1)^5}{m^3 \cdot 3! \cdot (m+1)(m+3)}t^3 + \cdots$$

For typical benchmark sample sizes ($n > 100$), $\Psi \approx \exp(t \cdot (m-1)/m) \approx \exp(s^2_{\ln X}/2)$,
so the UMVUE and MLE (estimator 2 at $k=1$) are **practically identical**. The UMVUE's
advantage over the MLE matters only for small samples ($n < 30$), where the MLE's $O(1/n)$
upward bias becomes non-negligible.

For $k>1$, the UMVUE generalises by replacing $(\overline{\ln X}, s^2_{\ln X}, n)$ with
the FW-corrected log-moments of $Y_j$; the resulting estimator is approximately unbiased
to the same degree as the FW approximation. In practice, at $k>1$ the FW-corrected
geometric mean (estimator 2) or its robustified version (B) is preferred for simplicity.

**Excluded (insufficient robustness or unjustified complexity):**

- **Plain $m_y$ with outlier rejection** — ad-hoc; the trimmed mean (estimator 4) is the
  principled version with known bias–variance properties.
- **Fenton–Wilkinson bias-corrected log-mean without robust scale** — same fragility as
  estimator 2's $\hat\sigma^2$ channel.
- **Shrinkage / blend** $w_k\hat\theta_1 + (1-w_k)\hat\theta_2$ — convex combination of a
  non-robust estimator ($m_y$, breakdown 0) with an estimator whose main weakness is the
  left tail; the blend inherits both fragilities.
- **Harmonic mean of $Y_j$** — targets a quantity ($1/\mathbb{E}[1/X]$) unrelated to
  $\mathbb{E}[X]$ under log-normality; no correction recovers the mean cleanly.

---

## 6. Summary Tables

### 6.1 Pros / cons by regime

Columns describe the $g$–$k$ balance; recall that for the mean, **$m_y$ is unbiased at every
$k$**, so bias entries refer to *alternatives relative to $m_y$*, not to absolute bias.
Cell symbols: ✓ favorable, ✗ unfavorable, ◐ mixed, *italic* = neutral note; **BP** = breakdown
point.

| Estimator | $k=1$ | $k$ small (FW) | $k$ intermediate | $k$ large (CLT) | $k=n$ |
|-----------|-------|-----------------|----------------|------------------|-------|
| **1. $m_y$** | ✓ unbiased, simple ✗ < MLE efficiency; **not robust** (BP 0) | ✓ unbiased ✗ non-robust | ✓ unbiased ✗ non-robust | ✓ unbiased ✗ non-robust | ✓ unbiased ✗ $s_y^2$ undefined |
| **2. $\exp(\text{ml}_y + \hat\sigma^2_w/2)$** | ✓ MLE, efficient ✗ left-tail sensitive; non-robust scale | ✓ ≈unbiased (FW) ✗ left-tail amp.; $\hat\sigma^2$ estim. error grows | ◐ FW approx. degrades; $\hat\sigma^2$ estim. noisy | ✗ $Y_j$ near-normal → correction ≈1 but $\hat\sigma^2$ estim. unreliable | ✗ degenerate |
| **3. $\text{median}(Y_j) \cdot \exp(\hat\sigma^2_w/2)$** | ✓ robust location ✗ ARE 0.64; fragile scale; model-dep. | ✓ robust location ✗ fragile scale; $\hat\sigma^2$ estim. error | ◐ bias small; robust loc. ✗ $\hat\sigma^2$ estim. unreliable | ✓ correction →1; robust loc. ✗ few $Y_j$, $\hat\sigma^2$ estim. unreliable | ✗ degenerate |
| **4. trimmed/winsorized mean** | ✗ downward bias (skewed $X$; needs asym. trim) | ◐ bias vs robustness trade-off | ✓ near-symmetric $Y$ → sym. trim ≈unbiased | ✓ **best robust here**: sym. trim unbiased, good efficiency | ✗ degenerate |
| **A. robustified #3** ($Q_n$ scale) | ✓ robust both axes ✗ ARE 0.64; model-dep. | ◐ robust both axes; $\tilde\sigma^2$ estim. error grows | ✓ ≈unbiased, robust both axes | ✓ correction →1; robust both axes | ✗ degenerate |
| **B. robustified #2** (trimmed $\ln Y$ + $Q_n$ scale) | ✓ efficient robust option ✗ left-tail still somewhat sensitive | ✓ **best robust-efficient at small $k$** | ◐ FW approx. degrades | ✗ $Y_j$ near-normal → log-transform unnecessary | ✗ degenerate |
| **C. $Q_n$/MAD-scale MoM** | ✓ robust scale estimate | ✓ robust scale; mean location non-robust | ◐ scale estimate noisy | ✗ few $Y_j$ for $Q_n$ | ✗ degenerate |
| **E. UMVUE** (Finney, $k=1$) | ✓ exactly unbiased, min. variance ✗ infinite series; ≈MLE for $n>100$ | — (reduces to FW-corrected #2) | — | — | — |

### 6.2 Asymptotic variance scaling (up to the common factor $e^{2\mu+\sigma^2}$)

Each entry is $\text{Var}(\hat\theta)/\theta^2$ where $\theta = e^{\mu+\sigma^2/2}$; multiply by
$\theta^2 = e^{2\mu+\sigma^2}$ to recover absolute variance. Note that $\text{Var}(X)/\theta^2 =
e^{\sigma^2}-1$.

| Estimator | $k=1$ | small $k$ | large $k$ (CLT) |
|-----------|-------|-----------|-----------------|
| **1. $m_y$** | $(e^{\sigma^2}-1)/n$ | same (*k-independent*) | same |
| **2. $\exp(\text{ml}_y + \hat\sigma^2_w/2)$** | $(\sigma^2+\sigma^4/2)/n$ (CRLB; $<(e^{\sigma^2}-1)/n$) | $\approx \sigma^2_w/g + O(1/g)$ from $\hat\sigma^2$ | dominated by $\hat\sigma^2$ estimation error |
| **3. $\text{median}(Y_j) \cdot \exp(\hat\sigma^2_w/2)$** | $(\pi/2)\,\sigma^2/n$ + $O(1/n)$ from $\hat\sigma^2$ | same order | $(\pi/2)(e^{\sigma^2}-1)/n$ + $\hat\sigma^2$ estimation error |
| **4. trimmed mean** | $>(e^{\sigma^2}-1)/n$ (trimming loss) | trim-loss $\times$ skewness-bias | $(e^{\sigma^2}-1)/(c_\alpha\,n)$ where $c_\alpha=$ Gaussian efficiency $\in(0,1]$ |
| **E. UMVUE** ($k=1$) | $(\sigma^2+\sigma^4/2)/n$ (exact CRLB; $\le$ MLE MSE in finite samples) | — | — |

---

## 7. Recommendations (robustness-first, two tracks)

The `bench_utils` reality: **batching is required only for very-low-latency targets** (to
swamp per-measurement overhead), and those targets typically need a **fairly large $k$**;
higher-latency targets can be measured unbatched. For the mean, batching imposes no
intrinsic bias — the tension is entirely about **robustness** to outliers and, at $k=1$,
**efficiency** relative to the MLE.

### Track A — no batching ($k=1$): *preferred whenever latency permits*

Estimate the mean directly from the raw observations $X_i$:

- **Default: sample mean $m_y = \bar{X}$** — unbiased, simple, and for typical latency
  benchmarking $\sigma$ (0.1–0.5) the efficiency loss vs. the lognormal MLE is at most 1%. The
  default choice unless outliers are a concern.
- **If outliers are expected:** trimmed or winsorized mean of $X$ (estimator 4 with
  asymmetric trim) or the robustified estimator B (trimmed log-mean with robust scale).
  The log-scale approach is more efficient under log-normality but assumes it; the
  direct trimmed mean is more distribution-agnostic.
- **If log-normality is trusted and efficiency matters:** the MLE
  $\exp(\overline{\ln X} + s^2_{\ln X}/2)$ (estimator 2 at $k=1$) — the most efficient
  consistent estimator. For small samples ($n < 30$), prefer the **UMVUE** (estimator E,
  §5) which is exactly unbiased at every $n$ and minimum-variance among unbiased
  estimators; for $n > 100$ the MLE and UMVUE are practically identical.
- **Avoid** the corrected median (estimator 3) here — its ARE of 0.64 vs. the MLE is a
  real cost, and its robustness advantage over the trimmed log-mean (B) is marginal at
  $k=1$ where a single extreme log-value is the main threat (both handle it).

### Track B — batching forced ($k>1$): *regime-dependent robust estimator*

The sample mean $m_y$ remains unbiased and is the natural baseline, but it is non-robust.
Choose a robust alternative by regime:

- **Small $k$ ($Y_j$ still skewed, $g$ large):** use **robustified estimator B**
  (trimmed log-mean + $Q_n$-scale FW correction). The trimmed log-mean protects both tails,
  the FW correction removes the $\mathbb{E}[\ln Y] \neq \mu$ drift, and the $Q_n$ scale
  is robust. This is the most efficient robust option where it matters (small $k$, many
  groups).
- **Moderate-to-large $k$ ($Y_j$ near-symmetric — where forced low-latency batching
  lands):** switch to a **direct robust location**: either the **trimmed/winsorized mean**
  (estimator 4, symmetric trim) or the **$Q_n$-robustified corrected median** (estimator
  A). Both are approximately unbiased without correction (since $Y_j \approx N$), so the
  correction factor $\exp(\tilde\sigma^2_w/2)$ is near 1 and inoffensive.
  - The trimmed mean has better Gaussian efficiency (≈94% at $\alpha=0.10$) but lower
    breakdown (10%).
  - The corrected median (A) has ≈50% breakdown but lower efficiency (≈64% ARE).
  - **Default recommendation:** trimmed mean with $\alpha = 0.10$ for moderate-$k$;
    switch to robustified #3 (A) if outlier batches are frequent or if $g$ is small enough
    that a single batch can sway the trimmed mean.
- **Data-driven switch:** use the estimated CV of $Y$,
  $\widehat{\text{CV}}_Y = s_y/m_y = \sqrt{(e^{\hat\sigma^2}-1)/k}$ (robust version:
  $\tilde\sigma_y/m_y$). While $\widehat{\text{CV}}_Y$ is large (skewed $Y$) use the
  log-space robust estimator (B); once $\widehat{\text{CV}}_Y$ is small ($\le 0.2$, $Y$
  near-normal) switch to a direct robust location (estimator 4 or A).
- **Plain $m_y$ (estimator 1)** is the simplest option — unbiased at all $k$, but
  non-robust. Use it as a quick baseline; always report a robust alternative for
  production.

### Guarding against departures from log-normality (applies to all corrected estimators)

Because the FW correction $\exp(\tilde\sigma^2_w/2)$ is lognormal-specific, corrected
estimators (2, 3, A, B) carry an **irreducible model-form bias** if $X$ is not lognormal.
Mitigations:

1. **At moderate-to-large $k$, the correction factor is near 1** — the estimators reduce
   to their uncorrected robust locations, removing the model dependence.
2. **Keep $k$ as small as measurement overhead allows** — less averaging means $Y_j$ stays
   closer to $X$'s own shape, and the log-space robust estimators (B) are more natural.
3. **Run an occasional unbatched pilot** (a subset measured at $k=1$) to (a) estimate
   $\sigma$ directly and (b) check the empirical $\mathbb{E}[X]/\text{median}(X)$ against
   $e^{\sigma^2/2}$; a large discrepancy flags non-log-normality.
4. **Prefer $m_y$ or a direct trimmed/winsorized mean (estimators 1 and 4) when log-normality
   is in doubt** — neither relies on the lognormal model.

### $k=n$ (single group) — degenerate

$g=1$: $s_y^2$ is undefined, so all correction-based estimators collapse. $m_y = \bar{X}_n$
remains unbiased for $\mathbb{E}[X]$ but no internal variance estimate is possible. If a
variance or CI is needed, supply $\sigma^2$ from an external pilot. Prefer designing $g \ge 5$
so that at least a rough variance estimate is available.

---

## 8. One-line takeaways

- **$m_y$ (estimator 1) is unbiased at every $k$** — batching does not hide the mean.
  Always report it; the problem is about robustness and efficiency, not bias recovery.
- **At $k=1$ (no batching):** $m_y$ is the default. If log-normality is trusted, the MLE
  (estimator 2) is more efficient; for small $n$, the UMVUE (estimator E) removes the MLE's
  finite-sample bias.
- **At small $k$ ($Y_j$ still skewed, FW holds):** the robustified corrected
  geometric mean (estimator B) — trimmed $\ln Y_j$ with $Q_n$-scale FW correction — is the
  most efficient robust choice. The corrected median (estimator A) is an alternative when
  maximum breakdown (≈50%) matters more than efficiency.
- **At moderate-to-large $k$ ($Y_j$ near-normal, CLT dominant):** a symmetric
  trimmed/winsorized mean (estimator 4, $\alpha = 0.10$–$0.20$) is ≈unbiased without correction
  and has good efficiency (≈87–94%). The robustified corrected median (estimator A) gives
  maximum breakdown at lower efficiency — use it when outlier batches are the dominant concern.
- **At $k=n$ (single group, degenerate):** only $m_y$ survives; $\hat\sigma^2$ is undefined.
  Supply $\sigma^2$ from a pilot if a correction or CI is needed. Prefer $g \ge 5$.

---

## References

1. Fenton, L. F. (1960). *IRE Trans. Comm. Syst.* 8(1), 57–67.
2. Finney, D. J. (1941). "On the distribution of a variate whose logarithm is normally
   distributed." *J. R. Stat. Soc. Suppl.* 7(2), 155–161.
3. Crow, E. L., & Shimizu, K. (1988). *Lognormal Distributions: Theory and Applications.*
   Dekker.
4. Rousseeuw, P. J., & Croux, C. (1993). "Alternatives to the median absolute deviation."
   *J. Amer. Statist. Assoc.* 88(424), 1273–1283.
5. Hampel, F. R., Ronchetti, E. M., Rousseeuw, P. J., & Stahel, W. A. (1986).
   *Robust Statistics: The Approach Based on Influence Functions.* Wiley.
6. Shimizu, K., & Iwase, K. (1981). "Uniformly minimum variance unbiased estimation in
   lognormal distributions." *Commun. Statist. – Theory Meth.* 10(11), 1127–1147.
