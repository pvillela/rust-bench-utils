# Optimum Median Estimator Under Batching of Lognormal Latency Data

*This document analyses estimators of the population **median** $\theta = e^\mu$ from **batched** latency
measurements $Y$, where batching **hides** the median — recovering $e^\mu$ from batch means requires a
distributional model. Robustness — including robustness to departures from log-normality — is a
first-class requirement.*

---

## 1. Setup and Notation

Let $X_1, \ldots, X_n \sim \text{Lognormal}(\mu, \sigma^2)$ be IID. Population quantities:

| Quantity | Expression |
|----------|-----------|
| **Median** (the target $\theta$) | $\exp(\mu)$ |
| **Mean** | $\mathbb{E}[X] = \exp(\mu + \sigma^2/2)$ |
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
\mu(v) = \ln(v) - \frac{\hat{\sigma}^2}{2}.
$$

Here $\hat{\sigma}^2$ estimates the log-scale variance $\sigma^2 = \text{Var}(\ln X)$.
The function $\mu(v)$ maps a mean-scale location $v$ to the lognormal-implied log-median, so
$\exp(\mu(v))$ back-transforms $v$ to the median scale.
Estimator 3 applies this map to the grand mean $m_y$; estimator 4 applies it pointwise to each
batch mean $Y_j$ and then takes the median.

**Exact moment identities (hold for every $k$, by linearity and within-group independence):**

$$
\mathbb{E}[Y_j] = \mathbb{E}[X] = e^{\mu+\sigma^2/2}, \qquad
\text{Var}(Y_j) = \frac{\text{Var}(X)}{k}, \qquad
\text{CV}^2(Y_j) = \frac{e^{\sigma^2}-1}{k}.
$$

Consequently $m_y = \bar{X}_n$ exactly (the grand mean of all $n$ observations), and — writing
$\xrightarrow{p}$ for convergence in probability, with $\operatorname{plim}$ its probability limit —

$$
m_y \xrightarrow{p} e^{\mu+\sigma^2/2}, \quad
\text{var}_x \xrightarrow{p} \text{Var}(X), \quad
\hat{\sigma}^2 \xrightarrow{p} \sigma^2, \quad
\mu(m_y) \xrightarrow{p} \mu .
$$

### The four candidate estimators (target $\theta = e^\mu$)

| # | Name | Formula |
|---|------|---------|
| 1 | Geometric mean of $Y_j$ | $\hat\theta_1 = \exp(\text{ml}_y) = \big(\prod_j Y_j\big)^{1/g}$ |
| 2 | Sample median of $Y_j$ | $\hat\theta_2 = \text{median}(Y_1,\ldots,Y_g)$ |
| 3 | Method-of-moments | $\hat\theta_3 = \exp(\mu(m_y)) = m_y\,e^{-\hat\sigma^2/2}$ |
| 4 | Corrected median | $\hat\theta_4 = \text{median}\big(e^{\mu(Y_1)},\ldots,e^{\mu(Y_g)}\big)$ |

---

## 2. Why Batching Forces a Model (the crux)

The batch mean carries the population **mean**, not the median: $\mathbb{E}[Y_j] = \mathbb{E}[X]$,
and as $k$ grows the central limit theorem (CLT) makes $Y_j$ symmetric, so $\text{median}(Y_j) \to \mathbb{E}[X]$ as well.
**Batching therefore destroys the per-operation median information.** Recovering $e^\mu$ from batch
means is only possible through a distributional model that ties the recoverable mean/variance back
to the median — for the lognormal, the single relation

$$\frac{\mathbb{E}[X]}{\text{median}(X)} = e^{\sigma^2/2}.$$

**Implication.** For $k>1$, *every* median estimator is model-dependent to some degree; the honest
goals are (a) minimize the model's leverage, and (b) use robust location/scale *inside* the required
correction. Only at $k=1$ can the median be estimated model-free.

**Two independent axes vary with $k$:**

- **Bias** of the median-of-means and log-mean estimators (1, 2, 4) is driven by the *near-normality
  of $Y_j$* — a function of $k$ and $\sigma^2$ only (skewness of $Y_j$ $\propto 1/\sqrt{k}$). It does
  **not** depend on $n$.
- **Estimation variance** is driven by the *number of groups* $g = n/k$.

---

## 3. Distribution of $Y_j$ as $k$ Varies

| Regime | Shape of $Y_j$ | Groups $g=n/k$ |
|--------|----------------|----------------|
| $k=1$ | Exactly $\text{Lognormal}(\mu,\sigma^2)$; $\ln Y_j\sim N(\mu,\sigma^2)$ | $g=n$ (many) |
| $k$ small (FW holds) | Right-skewed, less so than $X$; well approximated as lognormal (Fenton–Wilkinson) | many |
| $k$ intermediate | Transition: FW degrading, CLT emerging — neither lognormal nor normal | moderate |
| $k$ large (CLT dominant) | Approximately $N\!\big(\mathbb{E}[X],\ \text{Var}(X)/k\big)$ | few |
| $k=n$ | Single value $Y_1=\bar X_n$; cannot identify $\mu$ and $\sigma^2$ | $g=1$ |

The regime boundaries are determined by $k$ and $\sigma^2$ — **not** by $n$. The shape of
$Y_j$ depends only on $\text{CV}(Y_j) = \sqrt{(e^{\sigma^2}-1)/k}$, which is a function of
$k$ and $\sigma^2$ alone. "Small $k$" means the FW lognormal approximation is accurate —
typically $k \lesssim 10$–$20$, depending on $\sigma^2$ (the approximation degrades faster
for larger $\sigma^2$). "Large $k$" means the CLT has made $Y_j$ approximately normal —
roughly when $\text{CV}(Y_j) \lesssim 0.2$, i.e., $k \gtrsim 25(e^{\sigma^2}-1)$. The
number of groups $g = n/k$ (rightmost column) is a separate axis that governs estimation
variance, not the shape of $Y_j$.

Because $\text{CV}(Y_j)$ is the primary shape parameter, it is also the most direct
regime indicator in practice: estimate it as $\widehat{\text{CV}}_Y = s_y/m_y$ (robust:
$\tilde\sigma_y/m_y$) from the observed batch means — no separate knowledge of $k$ or
$\sigma^2$ is needed to gauge regime position.

The **Fenton–Wilkinson approximation**, invoked at several points below (see ref. 1 — Wilkinson's
contribution circulated informally and is traditionally credited in the name), models the sum (or
mean) of a few IID lognormals as *itself* lognormal with the same first two moments; it is accurate
for small $k$ and degrades as $Y_j$ turns Gaussian. Under FW:

$$Y_j \overset{\text{approx}}{\sim} \text{Lognormal}\!\big(\mu_w,\ \sigma^2_w\big),\qquad
\mu_w = \mu + \frac{\sigma^2 - \sigma^2_w}{2},\qquad
\sigma^2_w = \ln\!\Big(1 + \frac{e^{\sigma^2} - 1}{k}\Big).$$

This parameterisation is chosen so that $\exp(\mu_w + \sigma^2_w/2) = \exp(\mu + \sigma^2/2) =
\mathbb{E}[X]$ exactly. The median-of-means under FW is $\exp(\mu_w) = e^\mu \cdot
e^{(\sigma^2 - \sigma^2_w)/2}$, which drifts from $e^\mu$ at $k=1$ (where $\sigma^2_w = \sigma^2$) to
$\mathbb{E}[X]$ as $k\to\infty$ (where $\sigma^2_w \to 0$).

A useful drift formula (**delta method** — a first-order Taylor expansion of $\ln(\cdot)$ about
$\mathbb{E}[Y_j]$, valid once $Y_j$ concentrates):

$$\mathbb{E}[\ln Y_j] \approx \mu + \frac{\sigma^2}{2} - \frac{e^{\sigma^2}-1}{2k}.$$

It is **not** valid at $k=1$ in general (the exact anchor is $\mathbb{E}[\ln Y_1]=\mu$); however for
small $\sigma^2$ it correctly returns $\mu$ at $k=1$, and the induced bias in $\mu$ is
$\approx \tfrac{\sigma^2}{2}-\tfrac{e^{\sigma^2}-1}{2k}\approx \tfrac{\sigma^2(k-1)}{2k}$, i.e. $0$ at
$k=1$ and growing toward $\sigma^2/2$.

---

## 4. Analysis of the Four Estimators

### 4.1 Estimator 1 — $\hat\theta_1 = \exp(\text{ml}_y)$ (geometric mean of $Y_j$)

**$k=1$:** the lognormal maximum-likelihood estimator (MLE) of the median. $\text{ml}_y\sim N(\mu,\sigma^2/n)$, so
$\hat\theta_1$ is consistent and asymptotically efficient (attains the Cramér–Rao lower bound, CRLB —
the smallest variance any unbiased estimator can have; asymptotic variance
$\sigma^2 e^{2\mu}/n$). Finite-sample bias $\mathbb{E}[\hat\theta_1]=e^{\mu}\,e^{\sigma^2/(2n)}$
(upward, $O(1/n)$).

**$k>1$:** by Jensen's inequality (equivalently the arithmetic-mean–geometric-mean, AM–GM, inequality), $\mathbb{E}[\ln Y_j] > \mu$, so the bias is **strictly positive and
grows monotonically** with $k$, converging to $e^{\mu+\sigma^2/2}=\mathbb{E}[X]$ — i.e. at large $k$
the estimator silently targets the **mean**, overstating the median by $e^{\sigma^2/2}$.

**Variance — a saturation result.** With
$\text{Var}(\ln Y_j)\approx\ln\!\big(1+(e^{\sigma^2}-1)/k\big)$ (Fenton–Wilkinson),

$$\text{Var}(\text{ml}_y) = \frac{\text{Var}(\ln Y_j)}{g} \approx \frac{k}{n}\ln\!\Big(1+\frac{e^{\sigma^2}-1}{k}\Big),$$

which rises **monotonically** from $\sigma^2/n$ (at $k=1$) and **saturates** at $(e^{\sigma^2}-1)/n$
— it does *not* grow linearly in $k$. The efficiency-loss factor relative to $k=1$ is bounded by
$(e^{\sigma^2}-1)/\sigma^2$, independent of $k$; for $\sigma^2\ll1$ it is $\approx1$. **So batching's
cost to estimator 1 is almost entirely *bias*, not variance.**

**Robustness:** poor. $\ln(\cdot)$ tames the right tail but *amplifies* the left: a single small
$Y_j$ produces a large negative $\ln Y_j$ that drags $\text{ml}_y$ down. Mean-based, **breakdown
point** 0 — the breakdown point is the smallest fraction of arbitrarily corrupted observations that
can drive an estimator to an arbitrary value, so 0 means a single bad point suffices, whereas ≈50%
(the median's value) is the best attainable.

### 4.2 Estimator 2 — $\hat\theta_2 = \text{median}(Y_j)$

**$k=1$:** sample median of lognormal data. Consistent for $e^\mu$; asymptotic variance
$\frac{\pi}{2}\cdot\frac{\sigma^2 e^{2\mu}}{n}$, giving an asymptotic relative efficiency (ARE) of
$2/\pi\approx0.637$ vs. the MLE (i.e. it needs ≈57% more data for equal precision). The canonical
robust location estimator (≈50% breakdown).

**$k>1$:** the population median of $Y_j$ drifts from $e^\mu$ toward $\mathbb{E}[X]$; bias is positive
and grows with $k$, stabilizing near $e^\mu(e^{\sigma^2/2}-1)$ once $Y_j$ is near-normal.

**Variance is $k$-independent in scaling:** since
$\text{Var}(Y_j)=\text{Var}(X)/k$ and $g=n/k$, the $k$ cancels. The leading constant transitions from
$\frac{\pi}{2}\cdot\frac{\sigma^2 e^{2\mu}}{n}$ (lognormal-shaped $Y_j$) to
$\frac{\pi}{2}\cdot\frac{\text{Var}(X)}{n}$ (normal-shaped $Y_j$) — a factor
$e^{\sigma^2}(e^{\sigma^2}-1)/\sigma^2$ apart. Finite-sample precision still degrades as $g$ shrinks.

**Robustness:** excellent (≈50% breakdown, model-free). The most misspecification-robust of the four,
but it does **not** estimate $e^\mu$ once $k>1$.

**Two deliberate uses.** At $k=1$, $Y_j=X_j$, so this *is* the sample median of the raw data — the
model-free, ≈50%-breakdown, assumption-free ideal for median estimation, and the baseline against
which every batched estimator is judged (Track A, §7). At $k>1$, though biased upward for $e^\mu$, it
remains valuable as a **robust, misspecification-resistant sanity value**: reported next to a
corrected estimate, a gap larger than the expected $e^{\sigma^2/2}$ drift signals trouble (§7 guard).

### 4.3 Estimator 3 — $\hat\theta_3 = m_y\, e^{-\hat\sigma^2/2}$ (method of moments)

Uses the exact identities $\mathbb{E}[Y]=\mathbb{E}[X]$ and $\text{Var}(Y)=\text{Var}(X)/k$ to
reconstruct $\text{Var}(X)=k\,s_y^2$, then inverts the lognormal moment map. It is the **only one of
the four that is asymptotically unbiased for *every* $k$** with $g\ge2$ (its consistency does not
depend on the shape of $Y_j$, because it explicitly uses $k$). At $k=1$ it is the ordinary lognormal
method of moments — consistent but less efficient than the MLE (estimator 1), the gap widening with
$\sigma^2$.

**Behavior in $k$:** bias stays $\approx0$ throughout; variance is dominated by $\hat\sigma^2$, whose
input $s_y^2$ has relative standard error $\approx\sqrt{2/(g-1)}$ and is then multiplied by $k$
($\text{var}_x=k\,s_y^2$). As $g$ shrinks (large $k$) this amplification makes $\hat\sigma^2$ — and
hence $\hat\theta_3$ — noisy. At $k=n$, $s_y^2$ is undefined; forcing $s_y^2=0$ gives
$\hat\theta_3=\bar X_n$ (the mean).

**Robustness — its decisive weakness for this use case.** Estimator 3 is the **least robust** of the
four on *both* axes: the location $m_y$ is a raw mean (breakdown 0) and the scale $s_y^2$ is a raw
variance (breakdown 0; one extreme batch inflates $\hat\sigma^2$, biasing $\hat\theta_3$ *downward*).
It also leans hardest on the lognormal identity: under misspecification the "recovered $\mu$" need not
correspond to the log-median at all. This is exactly the fragility that motivates the robust variants
in §5. Estimator 3 is best viewed as the **simple baseline**, not the robust default.

### 4.4 Estimator 4 — $\hat\theta_4 = \text{median}(e^{\mu(Y_j)})$

**Key identity.** Since $e^{\mu(Y_j)} = Y_j\,e^{-\hat\sigma^2/2}$ and $e^{-\hat\sigma^2/2}$
is a common, order-preserving constant,

$$\boxed{\ \hat\theta_4 = e^{-\hat\sigma^2/2}\cdot \text{median}(Y_j) = e^{-\hat\sigma^2/2}\cdot\hat\theta_2\ }$$

— estimator 4 is estimator 2 scaled by the batch-aware correction $e^{-\hat\sigma^2/2}$.

**Bias is one-sided.** In the limit $g\to\infty$,
$\hat\theta_4 \xrightarrow{p} e^{-\sigma^2/2}\,Q_{0.5}(Y)$, where $Q_{0.5}(Y)$ — the population median
(0.5-quantile) of $Y$ — rises **monotonically** from $e^\mu$ (at $k=1$) to
$\mathbb{E}[X]=e^{\mu+\sigma^2/2}$ (as $k\to\infty$). Hence

$$\text{plim}\,\hat\theta_4 \ \text{rises from}\ e^{\mu-\sigma^2/2}\ \text{to}\ e^{\mu},\quad \text{always } \le e^\mu.$$

So the asymptotic bias is **non-positive throughout**, approaching zero **from below** as $k$ grows —
it does *not* cross through zero at a sweet spot. At small $k$ the
overcorrection is severe (factor $e^{-\sigma^2/2}$ at $k=1$); at moderate-to-large $k$, where $Y_j$ is
near-symmetric so $\text{median}(Y_j)\approx\mathbb{E}[X]$, the correction becomes approximately exact
and the bias is negligible. Finite-sample noise in $\hat\sigma^2$ and the median adds scatter but no
systematic overshoot. This is the **opposite** bias trend to estimators 1 and 2.

**Robustness:** robust *location* (median, ≈50% breakdown) but the scale channel $\hat\sigma^2$ still
uses $s_y^2$ (breakdown 0): an outlier batch inflates $\hat\sigma^2$ and biases $\hat\theta_4$
downward. Replacing $s_y^2$ with a robust scale (§5) removes this last weakness.

> **Note:** estimator 4 is *not* the only estimator approximately unbiased at large $k$.
> Estimator 3 is (asymptotically) unbiased there too; estimator 4's advantage there is *robustness*
> (median + robust scale), not uniqueness of unbiasedness.

---

## 5. Additional Noteworthy Estimators (robustness-filtered)

Per the intended use — latency benchmarking with common outliers and possible departures from
log-normality — only estimators with genuine robustness are listed. These build on the primary four:
the sample median (estimator 2) is itself the primary model-free robust option — at $k=1$ the median
of the raw data, at $k>1$ a robust sanity value (§4.2) — so it is not repeated here. Non-robust
proposals are **excluded** at the end of this section with reasons.

**A. Robustified estimator 4 — median location + robust scale.**
$\hat\theta_4^{\text{rob}} = e^{-\tilde\sigma^2/2}\cdot\text{median}(Y_j)$,
with $\tilde\sigma^2 = \ln\!\big(1 + k\,\tilde\sigma_y^2/m_y^2\big)$ and $\tilde\sigma_y$ a robust scale
(item **B** below). Robust on **both** axes; ≈unbiased in the moderate-to-large-$k$, near-normal
regime — precisely where forced low-latency batching lands. The recommended Track-B estimator for
larger $k$, where a robust scale can be reliably estimated ($g \ge 5$).

**B. Robust-scale method of moments.** Throughout, $\tilde\sigma_y$ denotes a **robust estimator of
the standard deviation of the group means $Y_j$** — a high-breakdown replacement for the ordinary
sample standard deviation $s_y$. It feeds the induced robust estimate of the lognormal shape,
$\tilde\sigma^2 = \ln\!\big(1 + k\,\tilde\sigma_y^2/m_y^2\big)$ — the same $\ln(1+k\,(\cdot)/m_y^2)$
map that estimator 3 applies to $s_y^2$, now fed $\tilde\sigma_y^2$ instead; both inputs are
natural-scale variances of $Y_j$, and the map's $\ln(\cdot)$ converts them to the log scale. Concrete high-breakdown choices for $\tilde\sigma_y$:

- **MAD:** $\tilde\sigma_y = 1.4826\cdot\text{MAD}(Y)$, where
  $\text{MAD}(Y) = \text{median}_j\,\big|Y_j - \text{median}(Y)\big|$. ≈50% breakdown, but only ≈37%
  Gaussian efficiency; the $1.4826$ factor assumes near-normal $Y$ (accurate only for larger $k$).
- **Rousseeuw–Croux $Q_n$ (preferred) / $S_n$:** $\tilde\sigma_y = Q_n(Y)$ or $S_n(Y)$, the
  Rousseeuw–Croux robust scale estimators. Each is a Gaussian consistency constant **times** a robust
  spread of the pairwise absolute differences:
  $$Q_n = c_Q \cdot \big\{\,|Y_i - Y_j| : i<j\,\big\}_{(h)}, \qquad
    S_n = c_S \cdot \operatorname{median}_i \operatorname{median}_{j\ne i} |Y_i - Y_j|,$$
  with $c_Q\approx2.2219$ and $c_S\approx1.1926$. In $Q_n$, the braces
  $\big\{\,|Y_i - Y_j| : i<j\,\big\}$ denote the **set of all $\binom{g}{2}$ pairwise absolute
  differences** of the group means, and the subscript $(h)$ selects that set's **$h$-th smallest
  value** (its $h$-th order statistic) with $h = \binom{\lfloor g/2\rfloor + 1}{2}$ — approximately
  the first quartile of the pairwise differences. In $S_n$, for each $i$ take the median over
  $j\ne i$ of $|Y_i - Y_j|$, then take the median of those $g$ values, and scale by $c_S$. Both give
  ≈50% breakdown **and** ≈82% ($Q_n$) / ≈58% ($S_n$) Gaussian efficiency — far above the MAD,
  approaching the non-robust $s_y^2$. Because they are built from pairwise differences
  $|Y_i - Y_j|$, **neither $Q_n$ nor $S_n$ requires a preliminary center (location) estimate** —
  unlike the MAD, which must first compute $\text{median}(Y)$ and then measures spread *around that
  center*, implicitly assuming symmetry about it. Avoiding the center is an advantage for skewed $Y$
  at small $k$. Cost: $O(g\log g)$. The constants $c_Q, c_S$ (like the MAD's $1.4826$) are
  **calibrated at the normal**: each is the value that makes its estimator converge to the true
  standard deviation $\sigma_Y$ *when $Y$ is Gaussian*. For skewed $Y$ (small $k$) those same
  constants yield a mildly biased scale, so a finite-sample/skewness correction helps.

  Then $\hat\theta = m_y\,e^{-\tilde\sigma^2/2}$ (robust *scale*, mean *location*) or feed $\tilde\sigma$
  into estimator A (robust scale **and** location).

  > **IQR is dominated (correcting a common belief).** A *functional* here is a rule mapping a
  > distribution to a single number — the population quantity the estimator targets. For any
  > symmetric distribution $\text{MAD}=\text{IQR}/2$ *exactly*, so — once each is rescaled by its
  > consistency constant — an IQR-based scale computes the **same population quantity** as the MAD
  > (i.e. it is the *same functional*), with the *same* ≈37% Gaussian efficiency, not higher, while
  > its breakdown point is only **25%** vs. the MAD's **50%**. Use MAD or, better, $Q_n$; do not use
  > an IQR-based scale.

**C. Trimmed / winsorized log-scale mean.**
$\hat\theta = \exp(\text{trimmed or winsorized mean of } \ln Y_j)$ (a *trimmed* mean discards the most
extreme $\ln Y_j$ before averaging; a *winsorized* mean instead clamps them to a chosen quantile).
Bounded influence against extreme
$\ln Y_j$ (both tails) while retaining more efficiency than the median; the trim/winsor fraction tunes
the robustness–efficiency trade-off. At $k=1$, $\ln Y_j$ is exactly normal so trimming costs little
bias. For $k>1$ it still does **not** remove the $\mathbb{E}[\ln Y]\neq\mu$ drift, so — if an unbiased
median is required — pair it with a bias correction: either the $e^{-\hat\sigma^2/2}$ factor (as in
estimators A and B) **or** the Fenton–Wilkinson correction. A useful
robust member of the $k=1$ log-scale family and a robust input to the correction at small $k$.

**Excluded (insufficient robustness for this use case):**

- **Fenton–Wilkinson bias-corrected log-mean** $\big(\text{ml}_y-\tfrac{\hat\sigma^2}{2}+\tfrac12\ln(1+(e^{\hat\sigma^2}-1)/k)\big)$
  — built on the log-mean $\text{ml}_y$ (breakdown 0; small $Y_j$ dominate). Efficient but not robust.
- **Bias-corrected MLE** $\exp\!\big(\text{ml}_y - s^2_{\ln Y}/2g\big)$ — same mean/log-mean fragility.
- **Shrinkage / blend** $w_k\hat\theta_1+(1-w_k)\hat\theta_3$ — a convex combination of two non-robust
  estimators is itself non-robust; also introduces an ad-hoc weight.

---

## 6. Summary Tables

### 6.1 Pros / cons by regime

Columns describe the $g$–$k$ balance; recall **bias depends on $k,\sigma^2$; variance depends on
$g=n/k$**. Cell symbols: ✓ favorable, ✗ unfavorable, ◐ mixed, *italic* = neutral note; **BP** = breakdown point.

| Estimator | $k=1$ | $k$ small (FW) | $k$ intermediate | $k$ large (CLT) | $k=n$ |
|-----------|-------|-----------------|----------------|------------------|-------|
| **1. $\exp(\text{ml}_y)$** | ✓ MLE, efficient (CRLB), ≈unbiased ✗ tiny-$X$ sensitive *(targets median only if $\ln X$ symmetric — weaker than full lognormality)* | ✓ variance nearly flat (saturates) ✗ upward bias appears, grows | ✗ substantial upward bias; lognormal fit for $Y$ failing | ✗ targets $\mathbb{E}[X]$, off by $e^{\sigma^2/2}$ | ✗ $=\bar X_n$; estimates mean |
| **2. $\text{median}(Y)$** | ✓ robust (≈50% BP), model-free, consistent ✗ ARE 0.64 vs MLE | ✓ still robust ✗ upward bias grows | ✗ moderate upward bias; fewer $Y_j$ | ✗ large bias ($\to\mathbb{E}[X]$); high variance | ✗ $=\bar X_n$ |
| **3. $m_y e^{-\hat\sigma^2/2}$** | ✓ correct target, consistent ✗ < MLE efficiency; **not robust** | ✓ **≈unbiased** (uses $k$); good precision ✗ non-robust (mean+var) | ✓ ≈unbiased ✗ $k\,s_y^2$ noise appears; non-robust | ✓ bias low ✗ $\hat\sigma^2$ very noisy; non-robust | ✗ $s_y^2$ undefined → $\bar X_n$ |
| **4. $e^{-\hat\sigma^2/2}\text{median}(Y)$** | ✗ over-corrected, bias $e^{-\sigma^2/2}$ | ✗ still biased low (approaching 0 from below) | ◐ bias small (one-sided); robust location | ✓ ≈unbiased **and** robust location ✗ scale $\hat\sigma^2$ noisy/fragile → use robust scale | ✗ $\hat\sigma^2$ undefined → $\bar X_n$ |
| **A. robustified #4** ($Q_n$ scale) | ✗ over-corrected at $k=1$ | ◐ biased low but robust both axes | ✓ ≈unbiased, robust both axes | ✓ **best Track-B here**: ≈unbiased + robust | ✗ degenerate |
| **B. $Q_n$/MAD-scale MoM** | ✓ correct target, robust scale ✗ < MLE eff. | ✓ ≈unbiased, robust scale ✗ mean location non-robust | ✓ ≈unbiased; robust scale | ✓ bias low; robust scale ✗ few $Y_j$ | ✗ degenerate |
| **C. trimmed/winsor. log-mean** | ✓ robust, near-MLE eff. *(targets median only if $\ln X$ symmetric — weaker than full lognormality)* | ✓ robust ✗ correction reintroduces lognormal/$\hat\sigma$ dependence | ◐ correction accuracy drops | ✗ log-scale mismatch grows | ✗ degenerate |

### 6.2 Asymptotic variance scaling (up to the common factor $e^{2\mu}$)

Each entry is $\text{Var}(\hat\theta)/\theta^2$ where $\theta = e^\mu$; multiply by
$\theta^2 = e^{2\mu}$ to recover absolute variance. Note that $\text{Var}(X)/\theta^2 =
e^{\sigma^2}(e^{\sigma^2}-1)$.

| Estimator | $k=1$ | small $k$ | large $k$ (CLT) |
|-----------|-------|-----------|-----------------|
| **1.** $\exp(\text{ml}_y)$ | $\sigma^2/n$ (CRLB) | $\nearrow$ toward $(e^{\sigma^2}-1)/n$ (saturating; loss $\le (e^{\sigma^2}-1)/\sigma^2$) | $\approx(e^{\sigma^2}-1)/n$ *(biased)* |
| **2.** $\text{median}(Y)$ | $\frac{\pi}{2}\sigma^2/n$ | const $\to \frac{\pi}{2}e^{\sigma^2}(e^{\sigma^2}-1)/n$ *(k-indep. scaling)* | — *(biased)* |
| **3./B.** MoM | $>\sigma^2/n$ | grows with $k$ (via $\hat\sigma^2$) | large *(noisy $\hat\sigma^2$)* |
| **4./A.** corrected median | — *(biased)* | — *(biased low)* | $\approx\frac{\pi}{2}e^{\sigma^2}(e^{\sigma^2}-1)/n$ + $O(1/g)$ from $\hat\sigma^2$ |

---

## 7. Recommendations (robustness-first, two tracks)

The `bench_utils` reality: **batching is required only for very-low-latency targets** (to swamp
per-measurement overhead), and those targets typically need a **fairly large $k$**; higher-latency
targets can be measured unbatched. This maps cleanly onto two tracks.

### Track A — no batching ($k=1$): *preferred whenever latency permits*

Estimate the median **model-free and robustly**:

- **Default: sample median of $X$** — this is estimator 2 at $k=1$ (where $Y_j=X_j$): ≈50% breakdown,
  unbiased for $e^\mu$, no distributional assumption. Best possible answer to the robustness +
  non-log-normality requirement.
- **If more efficiency is wanted and tails are controlled:** trimmed log-scale mean (estimator C), or
  the MLE $\exp(\overline{\ln X})$ when log-normality is trusted.
- **Avoid** estimator 4 here — its $e^{-\hat\sigma^2/2}$ correction over-corrects the already-correct
  median (downward bias $e^{-\sigma^2/2}$).

### Track B — batching forced ($k>1$): *regime-dependent robust estimator*

Median recovery is now parametric; write it as $\;e^\mu \approx e^{-\hat\sigma^2/2}\cdot
\text{location}(Y)\;$ and choose robust location/scale by regime:

- **Small $k$ ($Y$ still skewed, $g$ large):** use **mean location** — it is unbiased for
  $\mathbb{E}[X]$ at *every* $k$, whereas $\text{median}(Y)$ would bias low here. Take the
  **$Q_n$-scale method of moments** (estimator B): $m_y\,e^{-\tilde\sigma^2/2}$ with $\tilde\sigma$
  from $Q_n$ (fallback MAD). Its residual weakness is the non-robust mean location, so **keep $k$
  small and screen batches** (drop or down-weight obvious outlier batches).
- **Moderate-to-large $k$ ($Y$ near-symmetric — where forced low-latency batching lands):** switch to
  **robustified estimator 4** (estimator A): $e^{-\tilde\sigma^2/2}\cdot\text{median}(Y)$ with $Q_n$
  scale. Robust on **both** axes and ≈unbiased because $\text{median}(Y)\approx\mathbb{E}[X]$ here.
- **Data-driven switch:** compute $\widehat{\text{CV}}_Y = s_y/m_y$ (robust:
  $\tilde\sigma_y/m_y$) from the observed batch means — no separate knowledge of $k$ or
  $\sigma^2$ is required. Since $\text{CV}(Y_j) = \sqrt{(e^{\sigma^2}-1)/k}$ is what
  directly determines the near-normality of $Y_j$, it is the natural observable gauge of
  regime position, and its estimate is computable from the same data used for estimation.
  While $\widehat{\text{CV}}_Y$ is large (skewed $Y$) use the mean-location MoM (estimator B);
  switch to the median-location estimator (estimator A) once $\widehat{\text{CV}}_Y \lesssim 0.2$
  **and** $g \ge 5$. The CV threshold $0.2$ corresponds to $k \gtrsim 25(e^{\hat\sigma^2}-1)$ —
  **not** to $k \sim \sqrt{n}$ — so the switch point is a property of $k$ and $\sigma^2$ only,
  independent of $n$. Calibrate by simulation at the target $\sigma^2$.
- **Plain estimator 3** ($m_y e^{-\hat\sigma^2/2}$ with $s_y^2$) is the simplest option but the
  **least robust** (non-robust location *and* scale, maximal model reliance). Use it only as a quick
  baseline, not for production reporting.

### Guarding against departures from log-normality (applies to all of Track B)

Because the mean/median map $e^{\sigma^2/2}$ is lognormal-specific, batched median estimates carry an
**irreducible model-form bias** if $X$ is not lognormal. Mitigations:

1. **Keep $k$ as small as measurement overhead allows** — less averaging, less model leverage, and
   $Y_j$ closer to $X$'s own shape.
2. **Run an occasional unbatched pilot** (a subset measured at $k=1$) to (a) estimate $\sigma$
   directly and (b) check the empirical $\mathbb{E}[X]/\text{median}(X)$ against $e^{\sigma^2/2}$;
   a large discrepancy flags non-log-normality.
3. **Report the model-free $\text{median}(Y)$ (estimator 2) alongside** the corrected estimate as a
   sanity value; a large gap between them beyond the expected $e^{\sigma^2/2}$ drift is a warning.

### $k=n$ (single group) — degenerate

$g=1$: $s_y^2$ is undefined, so all correction-based estimators collapse; estimators 1, 2, and (forced)
3, 4 all reduce to $\bar X_n$, which estimates $\mathbb{E}[X]$ (biased by $e^{\sigma^2/2}$). If a
median is needed, supply $\sigma^2$ from an external pilot and report $\bar X_n\,e^{-\sigma^2/2}$;
otherwise report $\bar X_n$ as a mean and document the bias factor. Prefer designing $g\ge5$
so that at least a rough variance estimate is available; $g\ge2$ is the bare minimum for $s_y^2$
to exist.

---

## 8. One-line takeaways

- **If you can avoid batching, do** — sample median of $X$ is robust, model-free, and unbiased.
- **Batching hides the median**; recovering it needs the lognormal mean/median relation, so $k>1$
  median estimation is unavoidably model-dependent — minimize $k$ and validate with a pilot.
- **Estimator 3 is unbiased-for-all-$k$ but the least robust**; robustify its scale ($Q_n$) and, once
  $Y$ is near-normal, its location (median) → the regime-dependent robust estimator is the Track-B
  default.
- **Estimator 4 = $e^{-\hat\sigma^2/2}\cdot\text{median}(Y)$**, with one-sided (non-positive)
  asymptotic bias that vanishes only as $Y$ becomes symmetric — not a zero-crossing sweet spot.

---

## References

1. Fenton, L. F. (1960). *IRE Trans. Comm. Syst.* 8(1), 57–67.
2. Finney, D. J. (1941). "On the distribution of a variate whose logarithm is normally
   distributed." *J. R. Stat. Soc. Suppl.* 7(2), 155–161.
3. Crow, E. L., & Shimizu, K. (1988). *Lognormal Distributions: Theory and Applications.* Dekker.
4. Rousseeuw, P. J., & Croux, C. (1993). "Alternatives to the median absolute deviation."
   *J. Amer. Statist. Assoc.* 88(424), 1273–1283.
5. Hampel, F. R., Ronchetti, E. M., Rousseeuw, P. J., & Stahel, W. A. (1986). *Robust Statistics:
   The Approach Based on Influence Functions.* Wiley.
