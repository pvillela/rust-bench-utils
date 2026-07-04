# Optimum Median Estimator Under Batching of Lognormal Data

## 1. Problem Setup and Notation

### 1.1 The Data-Generating Process

Let $X_1, X_2, \ldots, X_n \sim \text{Lognormal}(\mu, \sigma^2)$ be IID. The probability density function of $X_i$ is:

$$f_X(x; \mu, \sigma) = \frac{1}{x\sigma\sqrt{2\pi}} \exp\!\left(-\frac{(\ln x - \mu)^2}{2\sigma^2}\right), \quad x > 0$$

Key population quantities:

| Quantity | Expression |
|----------|-----------|
| **Median** | $\exp(\mu)$ |
| **Mean** | $E[X] = \exp\!\left(\mu + \frac{\sigma^2}{2}\right)$ |
| **Variance** | $\text{Var}(X) = \exp(2\mu + \sigma^2)(\exp(\sigma^2) - 1)$ |
| **Squared CV** | $\text{CV}^2 = \frac{\text{Var}(X)}{E[X]^2} = \exp(\sigma^2) - 1$ |

### 1.2 Batching / Grouping

Let $k \in \mathbb{N}$ be a divisor of $n$. Partition the $n$ observations into $g = n/k$ disjoint groups of size $k$, and define the group means:

$$Y_j = \frac{1}{k} \sum_{i \in \text{group } j} X_i, \qquad j = 1, \ldots, g$$

We observe only the $g$ group means $Y_1, \ldots, Y_g$, not the individual $X_i$.

### 1.3 Sample Statistics of the Group Means

From the $g$ observed group means we compute:

$$\begin{aligned}
m_y &= \frac{1}{g} \sum_{j=1}^{g} Y_j &&\text{(sample mean of group means)} \\[4pt]
s_y^2 &= \frac{1}{g-1} \sum_{j=1}^{g} (Y_j - m_y)^2 &&\text{(sample variance of group means)} \\[4pt]
\text{ml}_y &= \frac{1}{g} \sum_{j=1}^{g} \ln(Y_j) &&\text{(sample mean of log group means)}
\end{aligned}$$

### 1.4 Derived Quantities

$$\text{var}_x = k \cdot s_y^2$$

$$\hat{\sigma}^2 = \ln\!\left(1 + \frac{\text{var}_x}{m_y^2}\right)$$

$$\mu(v) = \ln(v) - \frac{\hat{\sigma}^2}{2}$$

**Rationale for these definitions.** The identities $E[Y_j] = E[X]$ and $\text{Var}(Y_j) = \text{Var}(X)/k$ hold **exactly** for all $k$, by linearity of expectation and independence across the $k$ observations within each group. Therefore:

- $m_y \xrightarrow{p} E[X] = \exp(\mu + \sigma^2/2)$
- $k \cdot s_y^2 \xrightarrow{p} k \cdot \text{Var}(Y_j) = \text{Var}(X)$
- $\hat{\sigma}^2 \xrightarrow{p} \ln\!\left(1 + \frac{\text{Var}(X)}{E[X]^2}\right) = \ln(1 + \exp(\sigma^2) - 1) = \sigma^2$
- $\mu(m_y) \xrightarrow{p} \ln(E[X]) - \frac{\sigma^2}{2} = \mu$

### 1.5 The Four Candidate Estimators

The target of estimation is the population median $\theta = \exp(\mu)$.

| # | Estimator | Formula |
|---|-----------|---------|
| 1 | Geometric mean of $Y_j$ | $\hat{\theta}_1 = \exp(\text{ml}_y) = \left(\prod_{j=1}^{g} Y_j\right)^{1/g}$ |
| 2 | Sample median of $Y_j$ | $\hat{\theta}_2 = \text{median}(Y_1, \ldots, Y_g)$ |
| 3 | Method-of-moments | $\hat{\theta}_3 = \exp(\mu(m_y)) = m_y \cdot \exp(-\hat{\sigma}^2/2)$ |
| 4 | Bias-corrected median | $\hat{\theta}_4 = \text{median}\!\big(\exp(\mu(Y_1)), \ldots, \exp(\mu(Y_g))\big)$ |

---

## 2. Distribution of Group Means: The Shape Transition

Understanding how the distribution of $Y_j$ changes with $k$ is essential for analyzing the estimators.

### 2.1 $k = 1$: Individual Lognormal

$Y_j = X_j \sim \text{Lognormal}(\mu, \sigma^2)$. The distribution is right-skewed; the log-transform produces exact normality: $\ln(Y_j) \sim N(\mu, \sigma^2)$.

### 2.2 $1 < k \ll \sqrt{n}$: Sum of a Few Lognormals

$Y_j$ is the mean of a small number of IID lognormals. The distribution of a sum of lognormals has no closed form, but it remains right-skewed (though less so than the individual $X_i$). The Fenton–Wilkinson approximation models the sum as approximately lognormal with matched first two moments.

### 2.3 $k \approx \sqrt{n}$: Transition Regime

As $k$ grows, two things happen simultaneously:
1. **Shape:** By the CLT, $Y_j$ becomes more symmetric and approaches normality.
2. **Sample size:** $g = n/k$ shrinks, reducing the number of observations available for estimating group-level statistics.

### 2.4 $\sqrt{n} \ll k < n$: Near-Normal

The CLT dominates. $Y_j$ is approximately:

$$Y_j \;\dot{\sim}\; N\!\left(\exp(\mu + \sigma^2/2),\; \frac{\exp(2\mu + \sigma^2)(\exp(\sigma^2) - 1)}{k}\right)$$

When $Y_j$ is symmetric, its median equals its mean. This is the key to understanding why estimators 1 and 2 develop bias.

### 2.5 $k = n$: A Single Grand Mean

$g = 1$. A single observation $Y_1 = \bar{X}$ cannot identify both $\mu$ and $\sigma^2$.

### 2.6 Moment Identities (Exact For All $k$)

Despite the changing shape, the following hold exactly:

$$\begin{aligned}
E[Y_j] &= E[X] = \exp(\mu + \sigma^2/2) \\[4pt]
\text{Var}(Y_j) &= \frac{\text{Var}(X)}{k} = \frac{\exp(2\mu + \sigma^2)(\exp(\sigma^2) - 1)}{k} \\[4pt]
\text{CV}^2(Y_j) &= \frac{\text{Var}(Y_j)}{E[Y_j]^2} = \frac{\exp(\sigma^2) - 1}{k}
\end{aligned}$$

The squared CV of $Y_j$ is $\text{CV}^2(X) / k$ — the group mean is $k$ times more concentrated (in CV terms) than the individual observations.

---

## 3. Estimator Analysis

### 3.1 Estimator 1: $\hat{\theta}_1 = \exp(\text{ml}_y)$

**Mechanism.** Compute the log of each group mean, average, exponentiate. Equivalently, the geometric mean of the $Y_j$.

#### Behavior Across $k$

**$k = 1$:** This is the standard MLE for the lognormal median.
- $E[\ln(Y_j)] = E[\ln(X)] = \mu$ exactly
- $\text{ml}_y \sim N(\mu, \sigma^2/g)$ exactly
- $\hat{\theta}_1$ is consistent and asymptotically efficient
- Finite-sample bias: $E[\hat{\theta}_1] = \exp(\mu + \sigma^2/(2g))$, upward by factor $\exp(\sigma^2/(2g))$
- Asymptotic variance: $\frac{\sigma^2}{n} \exp(2\mu)$ — the Cramér–Rao lower bound

**$1 < k \ll \sqrt{n}$:** The log of a sum is not the sum of logs.
- By the AM–GM inequality and Jensen: $Y_j > \left(\prod_{i \in \text{group } j} X_i\right)^{1/k}$
- Therefore $E[\ln(Y_j)] > \frac{1}{k} \sum E[\ln(X_i)] = \mu$
- **Bias is strictly positive for all $k > 1$**
- The upward bias is small but already present. $Y_j$ remains right-skewed; the same delta method formula as the large-$k$ regime gives $E[\ln(Y_j)] \approx \mu + \frac{\sigma^2}{2} - \frac{\exp(\sigma^2)-1}{2k}$, so the bias in $\mu$ is approximately $\frac{\sigma^2}{2} - \frac{\exp(\sigma^2)-1}{2k} \approx \frac{\sigma^2(k-1)}{2k}$ for small $\sigma^2$ — growing from $0$ at $k=1$ but still modest here
- The number of groups $g$ is large, so $\text{ml}_y$ is well-estimated — variance is low
- The Fenton–Wilkinson approximation is reasonably accurate in this regime

**$k \approx \sqrt{n}$:** Bias continues to grow.
- The upward bias is now substantial: $E[\ln(Y_j)]$ is noticeably above $\mu$
- $g \approx \sqrt{n}$ groups is still adequate for estimating $\text{ml}_y$ with moderate precision
- The FW approximation begins to degrade as $Y_j$ loses lognormal shape
- The delta method approximation becomes more accurate as $Y_j$ concentrates

**$\sqrt{n} \ll k < n$:** As $Y_j$ approaches normality by the CLT, $Y_j$ concentrates around its mean. Using the delta method:

$$E[\ln(Y_j)] \approx \ln(E[Y_j]) - \frac{\text{Var}(Y_j)}{2E[Y_j]^2} = \mu + \frac{\sigma^2}{2} - \frac{\exp(\sigma^2) - 1}{2k}$$

As $k$ grows in this regime, $E[\ln(Y_j)] \to \mu + \sigma^2/2$, so $\hat{\theta}_1 \to \exp(\mu + \sigma^2/2) = E[X]$. The estimator converges to the population **mean**, not the median. Bias is severe and $g$ is small, so variance is also large.

**$k = n$:** $g = 1$, so $\hat{\theta}_1 = Y_1 = \bar{X}$. This estimates $E[X]$, biased by factor $\exp(\sigma^2/2)$. The estimator is degenerate — zero variance but the wrong target.

#### Pros

- $k = 1$: The optimal estimator — MLE, asymptotically efficient, minimum variance
- Simple to compute; uses all $g$ observations
- The log transform stabilizes variance for skewed data

#### Cons

- **Fundamentally wrong target for $k > 1$.** The geometric mean of sums is not the geometric mean of the underlying values
- Bias is upward, monotonic in $k$, and not self-correcting
- At large $k$, estimates $E[X]$ rather than $\exp(\mu)$ — an error of factor $\exp(\sigma^2/2)$
- The log transform is a distributional mismatch when $Y_j$ is not lognormal

#### Robustness

- Sensitive to values near zero (log of near-zero produces large negative values)
- Not robust to outliers in the $Y_j$ — a single very small $Y_j$ severely depresses $\text{ml}_y$
- Relies on $Y_j > 0$ (always satisfied for lognormal data, but fragile under contamination)

---

### 3.2 Estimator 2: $\hat{\theta}_2 = \text{median}(Y_1, \ldots, Y_g)$

**Mechanism.** The sample median of the $g$ group means. Fully nonparametric.

#### Behavior Across $k$

**$k = 1$:** $Y_j \sim \text{Lognormal}(\mu, \sigma^2)$. The population median is $\exp(\mu)$. The sample median is consistent and asymptotically normal:

$$\hat{\theta}_2 \xrightarrow{d} N\!\left(\exp(\mu),\; \frac{1}{4g \cdot f_Y^2(\exp(\mu))}\right) = N\!\left(\exp(\mu),\; \frac{\pi \sigma^2}{2g} \exp(2\mu)\right)$$

The asymptotic relative efficiency (ARE) vs. the MLE (estimator 1) is $2/\pi \approx 0.637$ — the sample median requires ~57% more data to achieve the same precision.

**$1 < k \ll \sqrt{n}$:** The population median of $Y_j$, denoted $Q_{0.5}(Y_j)$, **depends on $k$**. $Y_j$ is still right-skewed but less so than $X_i$. $Q_{0.5}(Y_j)$ lies between $\exp(\mu)$ (the true median) and $E[Y_j] = \exp(\mu + \sigma^2/2)$. The bias is **positive and small**, growing gradually as $Y_j$ gains symmetry. With many groups ($g \gg \sqrt{n}$), the sample median is well-estimated and variance is modest. The 57% efficiency penalty relative to estimator 1 applies as at $k=1$.

**$k \approx \sqrt{n}$:** The bias is now substantial — $Q_{0.5}(Y_j)$ is roughly midway between $\exp(\mu)$ and $E[Y_j]$. The number of groups $g \approx \sqrt{n}$ is still adequate for median estimation, though the finite-sample variance of the median is becoming noticeable. The trade-off between bias and variance is at its most delicate here.

**$\sqrt{n} \ll k < n$:** The CLT makes $Y_j$ roughly symmetric, so $Q_{0.5}(Y_j) \approx E[Y_j] = \exp(\mu + \sigma^2/2)$. Bias stabilizes at $\approx \exp(\mu)(\exp(\sigma^2/2) - 1)$ and is **independent of $k$** within this regime (once the CLT applies well). However, $g = n/k$ is small, so the sample median has **high finite-sample variance**: $\text{SE} \sim 1.253 \cdot s_y / \sqrt{g}$ with few observations.

**$k = n$:** $g = 1$, so $\hat{\theta}_2 = Y_1 = \bar{X}$, which estimates $E[X]$, not $\exp(\mu)$. Degenerate but wrong target.

#### Pros

- Fully nonparametric — no distributional assumptions
- Robust to outliers and model misspecification
- At $k = 1$: consistent for $\exp(\mu)$, robust alternative to MLE
- Meaningful even when the lognormal assumption is violated

#### Cons

- **Structural bias for $k > 1$:** targets $Q_{0.5}(Y_j)$, not $\exp(\mu)$
- Less efficient than mean-based estimators (ARE = 0.637 at $k = 1$; similar penalty at larger $k$)
- High variance when $g$ is small (few groups): the standard error of the median scales as $\sim 1.253 \cdot s_y / \sqrt{g}$
- Degenerates at $k = n$

#### Robustness

- **Breakdown point: ~50%** — can tolerate nearly half the $Y_j$ being contaminated before breaking down
- Bounded influence function — a single extreme $Y_j$ has limited effect
- The most robust of the four estimators to model misspecification

---

### 3.3 Estimator 3: $\hat{\theta}_3 = \exp(\mu(m_y))$

**Mechanism.** Method of moments. Exploits the exact identities $E[Y_j] = E[X]$ and $\text{Var}(Y_j) = \text{Var}(X)/k$ to recover $\mu$ from the sample moments of the $Y_j$:

$$\begin{aligned}
m_y &\xrightarrow{p} E[X] = \exp(\mu + \sigma^2/2) \\
\text{var}_x = k \cdot s_y^2 &\xrightarrow{p} k \cdot \text{Var}(Y_j) = \text{Var}(X) \\
\hat{\sigma}^2 = \ln\!\left(1 + \frac{k \cdot s_y^2}{m_y^2}\right) &\xrightarrow{p} \sigma^2 \\
\hat{\mu} = \ln(m_y) - \frac{\hat{\sigma}^2}{2} &\xrightarrow{p} \ln(E[X]) - \frac{\sigma^2}{2} = \mu
\end{aligned}$$

#### Behavior Across $k$

**$k = 1$:** Standard lognormal method of moments. Consistent but less efficient than the MLE — the sample variance $s_X^2$ is a noisy statistic, especially for large $\sigma$. At this $k$, $\hat{\theta}_3$ is inferior to $\hat{\theta}_1$, but it correctly targets $\exp(\mu)$.

**$1 < k \ll \sqrt{n}$:** This is the estimator's best regime. Both $m_y$ and $s_y^2$ are well-estimated (large $g = n/k$). The estimator is **approximately unbiased** — the exact moment identities hold regardless of $k$, and the large $g$ ensures good precision. The variance is somewhat higher than what a full likelihood approach would achieve, but no closed-form likelihood exists for $k > 1$. The FW approximation works well here, making estimator 5 a competitive alternative that may offer slightly better efficiency.

**$k \approx \sqrt{n}$:** A transitional regime where $g \approx \sqrt{n}$ groups provide moderate precision. $m_y$ remains well-estimated, but $s_y^2$ begins to show its relative standard error ($\approx \sqrt{2/(g-1)}$). The amplification $\text{var}_x = k \cdot s_y^2$ starts to matter — a $k$ of order $\sqrt{n}$ means the error in $s_y^2$ is multiplied by a non-trivial factor. Bias remains low, but variance is beginning to grow. This is approximately the MSE-optimal trade-off point between bias and variance for this estimator.

**$\sqrt{n} \ll k < n$:**
- $m_y$ remains well-estimated (CLT for the sample mean with $g$ observations)
- $s_y^2$ is estimated from very few points — its relative standard error is approximately $\sqrt{2/(g-1)}$, which is large when $g$ is small
- **Amplification:** $\text{var}_x = k \cdot s_y^2$ multiplies the estimation error by a large $k$. A noisy $s_y^2$ scaled by a large $k$ produces a very noisy $\text{var}_x$
- $\hat{\sigma}^2 = \ln(1 + \text{var}_x/m_y^2)$ compounds this noise through the nonlinear log transform
- **Bias remains low, but variance grows substantially as $g$ shrinks**

**$k = n$:** $g = 1$, $s_y^2$ is undefined. The estimator is **not computable**. If one forces $s_y^2 = 0$, then $\hat{\sigma}^2 = \ln(1 + 0) = 0$ and $\hat{\theta}_3 = m_y = \bar{X}$, which estimates $E[X]$ — the same failure mode as estimators 1 and 2.

#### Pros

- **The only estimator among the four that is asymptotically unbiased for all $k$ with $g \geq 2$**
- Bias correction is exact in expectation — no approximation needed for the moment identities
- Uses all $n$ observations (through the $g$ group means)
- Computationally simple: only requires $m_y$ and $s_y^2$

#### Cons

- Higher variance than estimator 1 at $k = 1$ (MoM vs. MLE efficiency loss)
- $\hat{\sigma}^2$ is sensitive to $s_y^2$ estimation error, amplified by the $k$ scaling factor at large $k$
- Not computable at $k = n$ (requires $g \geq 2$)
- The nonlinear transformation $\ln(1 + \text{var}_x/m_y^2)$ can be unstable when $\text{var}_x/m_y^2$ is imprecisely estimated

#### Robustness

- Relies on the lognormal moment identities $E[X] = \exp(\mu + \sigma^2/2)$ and $\text{CV}^2(X) = \exp(\sigma^2) - 1$
- Under model misspecification, the recovered $\mu$ may not correspond to any meaningful population parameter
- $s_y^2$ has breakdown point 0 — a single extreme $Y_j$ can inflate the variance estimate arbitrarily
- Not robust to heavy tails or contamination

---

### 3.4 Estimator 4: $\hat{\theta}_4 = \text{median}(\exp(\mu(Y_1)), \ldots, \exp(\mu(Y_g)))$

**Mechanism.** Since $\mu(Y_j) = \ln(Y_j) - \hat{\sigma}^2/2$ and exponentiation is monotonic, we have:

$$\exp(\mu(Y_j)) = \exp\!\left(\ln(Y_j) - \frac{\hat{\sigma}^2}{2}\right) = Y_j \cdot \exp(-\hat{\sigma}^2/2)$$

Because $\exp(-\hat{\sigma}^2/2)$ is a **common constant** applied to all $Y_j$, the ordering is unchanged. Therefore:

$$\boxed{\hat{\theta}_4 = \exp(-\hat{\sigma}^2/2) \cdot \text{median}(Y_1, \ldots, Y_g) = \exp(-\hat{\sigma}^2/2) \cdot \hat{\theta}_2}$$

**This is the key simplification: estimator 4 is estimator 2, scaled by the bias-correction factor $\exp(-\hat{\sigma}^2/2)$.**

#### Behavior Across $k$

**$k = 1$:** $Q_{0.5}(Y_j) = \exp(\mu)$. The correction gives $\hat{\theta}_4 \to \exp(-\sigma^2/2) \cdot \exp(\mu) = \exp(\mu - \sigma^2/2)$.
- **Underestimates** by factor $\exp(-\sigma^2/2)$
- The correction is actively harmful — the raw median is already correct

**$1 < k \ll \sqrt{n}$:** $Q_{0.5}(Y_j)$ lies slightly above $\exp(\mu)$ — the upward bias of estimator 2 is small in this regime. The correction $\exp(-\hat{\sigma}^2/2)$ is designed to cancel a full drift to $E[Y_j]$, so it **overcorrects**: $\hat{\theta}_4 < \exp(\mu)$. The negative bias is substantial. However, with many groups ($g$ large), $\hat{\sigma}^2$ is precisely estimated, so the correction factor is stable. The FW approximation can quantify the exact cross-over point where bias changes sign.

**$k \approx \sqrt{n}$:** This is the **bias-crossing regime**. $Q_{0.5}(Y_j)$ is roughly midway between $\exp(\mu)$ and $\exp(\mu + \sigma^2/2)$. The correction $\exp(-\hat{\sigma}^2/2)$ approximately cancels the upward drift — bias is near zero. This is potentially estimator 4's best operating point: bias is small but $g$ is still large enough that both the sample median and $\hat{\sigma}^2$ are reasonably precise. Whether bias is slightly negative or positive depends on $\sigma^2$ and the exact $k/\sqrt{n}$ ratio.

**$\sqrt{n} \ll k < n$ (the "sweet spot" for bias):** The CLT makes $Y_j$ roughly symmetric, so $Q_{0.5}(Y_j) \approx E[Y_j] = \exp(\mu + \sigma^2/2)$. The correction becomes:
$$\exp(-\sigma^2/2) \cdot \exp(\mu + \sigma^2/2) = \exp(\mu)$$
**Bias asymptotically vanishes.** However, this comes at a steep variance cost: $g$ is small, so both the sample median and $\hat{\sigma}^2$ are noisy. The estimator is approximately unbiased but may have the highest MSE among all four due to the **double variance penalty** (inefficient median + noisy $\hat{\sigma}^2$, each estimated from few observations).

**$k = n$:** $g = 1$, $\hat{\sigma}^2$ undefined. If forced to 0, $\hat{\theta}_4 = \bar{X}$, biased as before.

#### The Bias-Crossing Property

Estimator 4 has a distinctive bias profile:
- $k = 1$: negative bias (overcorrection)
- $1 < k \ll \sqrt{n}$: bias is negative, transitioning toward zero as $k$ grows
- $k \approx \sqrt{n}$: bias crosses through zero — the "sweet spot" where correction is exactly right
- $\sqrt{n} \ll k < n$: bias is near zero (correction cancels the median's drift) but variance is large
- $k \to n$: degenerates

This is the **opposite** of estimators 1 and 2, whose bias starts at zero and grows with $k$.

#### Pros

- Combines the median's robustness to outlier $Y_j$ with a batch-size-aware bias correction
- Approximately unbiased for intermediate-to-large $k$
- More robust than estimator 3 to extreme group means (uses median, not mean)

#### Cons

- Biased downward at small $k$ (overcorrection)
- **Double variance penalty:** inherits the median's lower efficiency AND the $\hat{\sigma}^2$ estimation noise
- Both components degrade as $g$ shrinks — the worst degradation curve of all four estimators
- Not computable at $k = n$

#### Robustness

- Better outlier resistance than estimator 3 (median component has ~50% breakdown)
- Still vulnerable through $\hat{\sigma}^2$ (which uses $s_y^2$, breakdown point 0)
- The $\hat{\sigma}^2$ channel creates an indirect sensitivity: an extreme $Y_j$ inflates $s_y^2$, which inflates $\hat{\sigma}^2$, which shrinks $\exp(-\hat{\sigma}^2/2)$, which biases $\hat{\theta}_4$ downward

---

## 4. Bias-Variance Decomposition

### 4.1 Summary of Asymptotic Bias

Let $b(\hat{\theta}_i, k) = \text{plim}_{g \to \infty} \hat{\theta}_i - \exp(\mu)$.

| Estimator | $k = 1$ | $1 < k \ll \sqrt{n}$ | $k \approx \sqrt{n}$ | $\sqrt{n} \ll k < n$ | $k = n$ |
|-----------|---------|----------------------|----------------------|----------------------|---------|
| $\hat{\theta}_1$ | 0 | Small $> 0$, increasing | Moderate $> 0$ | $\approx \exp(\mu)(\exp(\sigma^2/2) - 1)$ | Same |
| $\hat{\theta}_2$ | 0 | Small $> 0$, increasing | Moderate $> 0$ | $\approx \exp(\mu)(\exp(\sigma^2/2) - 1)$ | Same |
| $\hat{\theta}_3$ | 0 | $\approx 0$ | $\approx 0$ | $\approx 0$ | Undefined |
| $\hat{\theta}_4$ | $< 0$ | $< 0$, approaching 0 | $\approx 0$ (crosses) | $\approx 0$ | Undefined |

### 4.2 Variance Considerations

The variance of each estimator depends on $g = n/k$ (the number of groups):

- **$\text{Var}(\hat{\theta}_1)$:** Dependent on $\text{Var}(\ln Y_j)$, which shrinks as $Y_j$ concentrates (large $k$), but the effective sample size $g$ also shrinks. The net effect is complex.

- **$\text{Var}(\hat{\theta}_2)$:** Scales as $\sim 1.57 \cdot \text{Var}(Y_j) / g$. Since $\text{Var}(Y_j) = \text{Var}(X)/k$, this becomes $\sim 1.57 \cdot \text{Var}(X) / n$, which is **independent of $k$** for large $g$. But for small $g$, the constant 1.57 is an asymptotic approximation that degrades.

- **$\text{Var}(\hat{\theta}_3)$:** Dominated by the variance of $\hat{\sigma}^2$, which involves $s_y^2$ estimated from $g$ observations, then scaled by $k$. The variance inflates as $g$ shrinks.

- **$\text{Var}(\hat{\theta}_4)$:** Inherits both the median's variance and the $\hat{\sigma}^2$ variance. Largest variance among the four estimators across most regimes.

### 4.3 The Fundamental Tension

Choosing $k$ involves two competing effects:

| Effect | As $k$ increases... |
|--------|-------------------|
| **Distribution shape** | $Y_j$ becomes more symmetric/Gaussian → moment estimators stabilize |
| **Sample size** | $g = n/k$ shrinks → all group-level statistics become noisier |

The optimal $k$ balances these, typically around $k \approx \sqrt{n}$ (so $g \approx \sqrt{n}$).

---

## 5. Robustness Analysis

### 5.1 To Model Misspecification (Non-Lognormal $X$)

If $X_i$ are not exactly lognormal:

| Estimator | Behavior under misspecification |
|-----------|-------------------------------|
| $\hat{\theta}_1$ | Estimates $\exp(E[\ln X])$ — the geometric mean of the population. For $k = 1$, this is a well-defined population parameter (the geometric mean). For $k > 1$, the interpretation becomes murky. |
| $\hat{\theta}_2$ | Estimates $Q_{0.5}(Y_j)$ — always well-defined and interpretable regardless of the distribution. **Most robust to misspecification.** |
| $\hat{\theta}_3$ | The identities $E[X] = \exp(\mu + \sigma^2/2)$ and $\text{CV}^2 = \exp(\sigma^2) - 1$ are **specific to the lognormal**. Under misspecification, the "recovered" $\mu$ is not the log-median. |
| $\hat{\theta}_4$ | Inherits estimator 3's sensitivity through $\hat{\sigma}^2$, plus estimator 2's nonparametric median. Mixed robustness. |

### 5.2 To Outliers / Contamination

If some $Y_j$ are contaminated (e.g., one abnormally slow batch in a benchmarking context):

- **$\hat{\theta}_1$:** $\ln(Y_j)$ is sensitive to values near zero. A very small $Y_j$ produces a large negative $\ln(Y_j)$ that strongly influences the mean.
- **$\hat{\theta}_2$:** High breakdown point (~50%). A single contaminated $Y_j$ has bounded influence.
- **$\hat{\theta}_3$:** $s_y^2$ has breakdown point 0. One extreme $Y_j$ can inflate $s_y^2$ arbitrarily, which propagates through to $\hat{\sigma}^2$ and biases the estimate.
- **$\hat{\theta}_4$:** The median component is robust, but $\hat{\sigma}^2$ still uses $s_y^2$. An inflated $\hat{\sigma}^2$ shrinks $\exp(-\hat{\sigma}^2/2)$, biasing $\hat{\theta}_4$ downward.

### 5.3 To Small $g$ (Few Groups)

When $g$ is small (large $k$):

- $\hat{\theta}_3$ and $\hat{\theta}_4$ both rely on $s_y^2$, which has high relative standard error $\approx \sqrt{2/(g-1)}$.
- $\hat{\theta}_2$ (sample median) has large finite-sample variance when $g$ is small.
- $\hat{\theta}_1$ also suffers from small $g$, but uses the mean (which is more efficient than the median for a given sample size).

---

## 6. Summary Table

| Estimator | $k = 1$ | $1 < k \ll \sqrt{n}$ | $k \approx \sqrt{n}$ | $\sqrt{n} \ll k < n$ | $k = n$ |
|-----------|---------|----------------------|----------------------|----------------------|---------|
| **1.** $\exp(\text{ml}_y)$ | **Pro:** MLE — minimum variance, asymptotically efficient. Unbiased. **Con:** None (optimal choice). | **Pro:** Simple, uses all groups, low variance. **Con:** Small upward bias (Jensen inequality). FW approximation can quantify bias. | **Pro:** Simple. Moderate variance. **Con:** Substantial upward bias — targets a value between median and mean. | **Pro:** Simple. **Con:** Severe upward bias — converges to $E[X]$, not median. Small $g$ amplifies variance. | **Con:** Degenerate. Estimates $E[X]$, biased by factor $\exp(\sigma^2/2)$. |
| **2.** $\text{median}(Y_j)$ | **Pro:** Robust (50% breakdown), nonparametric. Consistent. **Con:** ~57% higher variance than MLE. | **Pro:** Robust to outliers and misspecification. Many groups → precise median. **Con:** Small upward bias as $Y_j$ gains some symmetry. | **Pro:** Robust. Moderate $g$ → acceptable precision. **Con:** Moderate upward bias — median drifts toward $E[Y_j]$. Bias-variance trade-off is delicate. | **Pro:** Robust to outliers. **Con:** Large bias ($\approx E[X]-\exp(\mu)$). High variance from few groups. | **Con:** Single value $= \bar{X}$. Same bias as #1. |
| **3.** $\exp(\mu(m_y))$ | **Pro:** Consistent, correct target. **Con:** Higher variance than MLE (MoM efficiency loss). | **Pro:** ~Unbiased — the best estimator in this regime. Low variance. Only estimator with consistently low bias across $k$. **Con:** Relies on lognormal assumption. | **Pro:** Bias remains low. $g \approx \sqrt{n}$ gives adequate precision for $m_y$ and $s_y^2$. **Con:** $k \cdot s_y^2$ amplification becomes noticeable. | **Pro:** Bias stays low. **Con:** Variance grows substantially ($g$ small; $k \cdot s_y^2$ amplification). $\hat{\sigma}^2$ noisy. | **Con:** Not computable ($s_y^2$ undefined). If forced ($s_y^2 = 0$): degenerate, biased. |
| **4.** $\text{median}(\exp(\mu(Y_j)))$ | **Con:** Underestimates by factor $\exp(-\sigma^2/2)$. Correction is actively harmful here. | **Pro/Con:** Negative bias (overcorrection), approaching zero as $k$ grows. **Pro:** $\hat{\sigma}^2$ well-estimated (large $g$). | **Pro:** Bias $\approx 0$ — the crossing point where correction is exactly right. **Con:** Sensitive to $k$ choice; slight $\hat{\sigma}^2$ error shifts bias. | **Pro:** Bias $\approx 0$ (correction cancels the median's drift). **Con:** Double variance penalty (median + $\hat{\sigma}^2$). Both noisy with small $g$. Largest MSE of all four. | **Con:** Not computable ($\hat{\sigma}^2$ undefined). If forced: degenerate. |

---

## 7. Additional Noteworthy Estimators

### 7.1 Estimator 5: Fenton–Wilkinson Bias-Corrected Log-Mean

The Fenton–Wilkinson (FW) approximation models the sum of $k$ IID lognormals as approximately lognormal with matched first two moments. Under this approximation:

$$\ln(Y_j) \;\dot{\sim}\; N\!\left(\mu + \frac{\sigma^2}{2} - \frac{1}{2}\ln\!\left(1 + \frac{\exp(\sigma^2) - 1}{k}\right),\; \ln\!\left(1 + \frac{\exp(\sigma^2) - 1}{k}\right)\right)$$

Solving for $\mu$ yields the bias-corrected estimator:

$$\hat{\mu}_5 = \text{ml}_y - \frac{\hat{\sigma}^2}{2} + \frac{1}{2}\ln\!\left(1 + \frac{\exp(\hat{\sigma}^2) - 1}{k}\right)$$

$$\hat{\theta}_5 = \exp(\hat{\mu}_5)$$

**Properties:**
- $k = 1$: The correction term becomes $\frac{1}{2}\ln(1 + (\exp(\hat{\sigma}^2) - 1)) = \hat{\sigma}^2/2$, canceling the $-\hat{\sigma}^2/2$ term exactly. Reduces to $\hat{\theta}_1$, the MLE. ✓
- $1 < k \ll \sqrt{n}$: Actively corrects estimator 1's upward bias using FW approximation. This is where estimator 5 shines — the FW approximation is most accurate for small $k$ (sum of a few lognormals is well-approximated by a lognormal).
- $k \approx \sqrt{n}$: FW approximation begins to degrade as $Y_j$ loses lognormal shape. Correction accuracy decreases, but may still offer MSE improvement over estimator 3.
- $\sqrt{n} \ll k < n$: FW approximation degrades further (the sum becomes normal, not lognormal). The correction becomes less reliable and may introduce its own bias. Estimator 3 is preferred in this regime.

**Pros:**
- Bridges the gap between estimator 1 (efficient at $k=1$) and estimator 3 (unbiased for all $k$)
- For $1 < k \ll \sqrt{n}$, may offer better MSE than estimator 3 by starting from the more efficient $\text{ml}_y$ statistic and applying a small FW correction
- Smoothly reduces to estimator 1 at $k=1$

**Cons:**
- Relies on FW approximation, which degrades as $k$ grows. By $k \approx \sqrt{n}$, the correction accuracy is questionable
- Uses $\hat{\sigma}^2$ (from estimator 3's moment matching), inheriting its noise, especially for $\sqrt{n} \ll k < n$
- More complex; requires numerical evaluation of $\exp(\hat{\sigma}^2)$

### 7.2 Estimator 6: Robust Scale Method-of-Moments

Replace $s_y^2$ in estimator 3 with a robust scale estimator. Let $\text{MAD}(Y) = \text{median}(|Y_j - \text{median}(Y)|)$ be the median absolute deviation, and define:

$$\tilde{\sigma}_y = \frac{\text{MAD}(Y)}{\Phi^{-1}(0.75)} \approx 1.4826 \cdot \text{MAD}(Y)$$

For normal data, $\tilde{\sigma}_y$ consistently estimates the standard deviation. For the $Y_j$ (which are approximately normal for large $k$), estimate $\text{Var}(Y_j)$ by $\tilde{\sigma}_y^2$, then proceed as in estimator 3:

$$\tilde{\text{var}}_x = k \cdot \tilde{\sigma}_y^2, \qquad \tilde{\sigma}^2 = \ln\!\left(1 + \frac{\tilde{\text{var}}_x}{m_y^2}\right), \qquad \hat{\theta}_6 = m_y \cdot \exp(-\tilde{\sigma}^2/2)$$

**Pros:**
- Robust to outlier $Y_j$ (MAD has ~50% breakdown point)
- Retains estimator 3's asymptotic unbiasedness for all $k$
- Particularly useful when some batches may be contaminated (e.g., system interrupts during benchmarking)

**Cons:**
- MAD-to-SD conversion factor (1.4826) assumes normality of $Y_j$, which only holds for large $k$
- For small $k$, $Y_j$ is skewed; MAD underestimates the standard deviation of a right-skewed distribution
- Less efficient than $s_y^2$ when no contamination is present (MAD efficiency is ~37% at normal)
- The tuning constant $\Phi^{-1}(0.75)$ is correct only asymptotically

### 7.3 Estimator 7: IQR-Based Scale Estimation

Use the interquartile range in place of $s_y^2$:

$$\tilde{\sigma}_y^{\text{IQR}} = \frac{\text{IQR}(Y)}{2 \cdot \Phi^{-1}(0.75)} \approx 0.7413 \cdot \text{IQR}(Y)$$

Then proceed identically to estimator 6.

**A caution on efficiency (correcting a common misconception).** The IQR is often presented as a more efficient alternative to the MAD, but for the symmetric case this is false. For *any* symmetric distribution the two are the **same scale functional**: the median of $|X - \text{med}|$ equals half the interquartile range, so $\text{MAD} = \text{IQR}/2$ and therefore $1.4826\,\text{MAD} = 0.7413\,\text{IQR}$ *exactly* — the consistency constants in estimators 6 and 7 are two expressions of the same quantity. Consequently the MAD- and IQR-based scale estimators have essentially the **same Gaussian asymptotic efficiency, ≈ 37%** (a direct computation gives $0.368$ for both), not 67%. They differ only in their finite-sample estimator and, decisively, in breakdown point: the MAD tolerates 50% contamination, whereas the IQR breaks at 25% (corrupting a quarter of the $Y_j$ can drive a quartile arbitrarily far).

Because the IQR yields no efficiency gain over the MAD yet has a strictly lower breakdown point, **estimator 6 (MAD) weakly dominates estimator 7 for symmetric $Y_j$**, and estimator 7 is not recommended. When one genuinely wants both high breakdown *and* high efficiency, use a Rousseeuw–Croux scale estimator instead (§7.6).

**Pros:**
- Marginally simpler to describe than the MAD

**Cons:**
- No efficiency advantage over the MAD (both ≈ 37% at normal), contrary to the common belief
- Strictly lower breakdown point (25% vs. the MAD's 50%)
- Same normality assumption for the conversion factor

### 7.4 Estimator 8: Shrinkage / Blended Estimator

Since estimator 1 has lower variance at small $k$ and estimator 3 has lower bias at large $k$, a shrinkage estimator can blend them:

$$\hat{\theta}_8 = w_k \cdot \hat{\theta}_1 + (1 - w_k) \cdot \hat{\theta}_3$$

where $w_k \in [0, 1]$ is a weight that decreases with $k$. A natural choice is $w_k = 1/k$ or $w_k = \exp(-k/\sqrt{n})$.

**Pros:**
- Adapts automatically across $k$ regimes
- At $k=1$: $w_1 = 1$, uses the MLE exclusively
- At large $k$: $w_k \approx 0$, uses estimator 3 exclusively

**Cons:**
- Introduces a weight function that must be chosen (tuning parameter)
- The two estimators may be correlated (both use the same $Y_j$), complicating variance estimation
- Ad-hoc; not derived from an optimality criterion

### 7.5 Estimator 9: Log-Scale Winsorized Mean

Apply a winsorized mean in the log domain to robustly estimate $E[\ln(Y_j)]$, then bias-correct using the FW approximation:

$$\text{ml}_y^{\text{win}(\alpha)} = \frac{1}{g} \sum_{j=1}^{g} \psi_\alpha(\ln(Y_j))$$

where $\psi_\alpha$ censors values below the $\alpha$-quantile and above the $(1-\alpha)$-quantile of the $\ln(Y_j)$. Then apply estimator 5's correction.

**Pros:**
- Robust to extreme $\ln(Y_j)$ values while retaining more efficiency than the median
- The winsorizing fraction $\alpha$ controls the robustness-efficiency trade-off
- $k = 1$: $\ln(Y_j) = \ln(X_j)$ is exactly normal, so winsorizing adds little bias

**Cons:**
- For $k > 1$, $\ln(Y_j)$ is not normal; the optimal winsorizing fraction is unknown
- Additional tuning parameter ($\alpha$)

### 7.6 Estimator 10: Rousseeuw–Croux Robust Scale ($Q_n$ / $S_n$)

The MAD's (and IQR's) chief weakness is low Gaussian efficiency (~37%). The Rousseeuw–Croux estimators achieve **both** a 50% breakdown point **and** high efficiency, making them the natural robust replacement for $s_y^2$ in estimator 3's moment-matching step. Define

$$S_n = c_S \cdot \operatorname*{med}_i \operatorname*{med}_{j \neq i} |Y_i - Y_j|, \qquad Q_n = c_Q \cdot \big\{\,|Y_i - Y_j| : i < j\,\big\}_{(h)},\quad h = \binom{\lfloor g/2\rfloor + 1}{2},$$

i.e. $S_n$ is a median of pairwise-distance medians, and $Q_n$ is (approximately) the first quartile of the $\binom{g}{2}$ pairwise absolute differences. Unlike the MAD, neither requires first estimating a center, so neither imposes a symmetry assumption on that step. With the Gaussian consistency constants $c_S \approx 1.1926$ and $c_Q \approx 2.2219$, both estimate $\sigma_Y$ consistently at the normal. Substitute $\tilde{\sigma}_y = Q_n$ (or $S_n$) into the estimator-3 machinery:

$$\hat{\theta}_{10} = m_y \cdot \exp(-\tilde{\sigma}^2/2), \qquad \tilde{\sigma}^2 = \ln\!\left(1 + \frac{k\,\tilde{\sigma}_y^2}{m_y^2}\right)$$

**Pros:**
- **Breakdown point 50%** — as robust as the MAD.
- **Gaussian efficiency ≈ 82% for $Q_n$** (≈ 58% for $S_n$) — far above the ≈ 37% of the MAD/IQR, and approaching the non-robust $s_y^2$. This is the estimator estimator 7 was mistakenly believed to be.
- $Q_n$ avoids a center estimate, an advantage at small-to-moderate $k$ where $Y_j$ is skewed.

**Cons:**
- More complex; the naive form is $O(g^2)$ in time/memory, though $O(g \log g)$ algorithms exist.
- The consistency constants $c_Q, c_S$ are calibrated at the normal; for skewed $Y_j$ (small $k$) a finite-sample/skewness correction improves accuracy, as with every scale estimator here.
- Requires $g \geq 2$; precision still degrades as $g$ shrinks.

$Q_n$ is the preferred choice whenever outlier resistance **and** efficiency both matter — i.e. it dominates estimators 6 and 7 in Regimes 2–4 when contaminated batches are a concern but the ~37% efficiency loss of the MAD is unacceptable.

---

## 8. Practical Recommendations

### 8.1 Decision Flowchart

For each regime of $k$, first select the standard estimator, then evaluate whether a robust alternative is warranted.

---

#### Regime 1: $k = 1$ (No Batching — Raw Individual Data)

**Standard choice:** $\hat{\theta}_1 = \exp(\overline{\ln X})$
- The MLE — asymptotically efficient, unbiased, minimum variance. This is the gold standard when individual observations are available and the lognormal assumption holds.

**Robust alternative:** $\hat{\theta}_2 = \text{median}(X_1, \ldots, X_n)$
- **When to use:** Outlier concerns (e.g., GC pauses, scheduler preemption, measurement artifacts) or uncertain lognormality. The sample median has ~50% breakdown point and targets $\exp(\mu)$ without parametric assumptions.
- **Cost:** ~57% higher asymptotic variance than the MLE.
- **Avoid:** $\hat{\theta}_4$ — the correction factor $\exp(-\hat{\sigma}^2/2)$ overcorrects at $k = 1$, producing downward bias by factor $\exp(-\sigma^2/2)$.

---

#### Regime 2: $1 < k \ll \sqrt{n}$ (Many Groups, Small Batches)

**Standard choice:** $\hat{\theta}_3 = \exp(\mu(m_y))$
- The only estimator that is approximately unbiased across this entire regime. Large $g = n/k$ ensures both $m_y$ and $s_y^2$ are well-estimated. The $k \cdot s_y^2$ amplification is negligible because $k$ is small.

**Robust alternative (outlier-resistant):** $\hat{\theta}_6$ (robust scale MoM with MAD)
- **When to use:** Contaminated batches (e.g., occasional system interrupts during a batch). Replaces $s_y^2$ with MAD-based scale, giving ~50% breakdown against outlier $Y_j$.
- **Cost:** MAD efficiency is ~37% at normality — trades precision for robustness. For small $k$, $Y_j$ is skewed, so the MAD → SD conversion factor (1.4826, calibrated for normal data) may introduce a small additional bias.
- **Higher-efficiency robust alternative:** $\hat{\theta}_{10}$ (Rousseeuw–Croux $Q_n$ scale) keeps the ~50% breakdown of the MAD while recovering ~82% Gaussian efficiency (vs. the MAD's ~37%). Prefer this when both robustness and efficiency matter. (Note: $\hat{\theta}_7$, the IQR-based scale, gives *no* efficiency gain over the MAD — it is the same functional for symmetric data — and has a lower 25% breakdown, so it is not recommended.)

**Robust alternative (misspecification-resistant):** $\hat{\theta}_2 = \text{median}(Y_j)$
- **When to use:** The lognormal assumption is in doubt, but robustness to outliers in $Y_j$ is the primary concern (rather than unbiasedness for $\exp(\mu)$). Note that estimator 2 has a small upward bias in this regime.

**Efficiency-focused alternative:** $\hat{\theta}_5$ (Fenton–Wilkinson bias-corrected log-mean)
- **When to use:** $k$ is very small (e.g., $k = 2, 3, 4$) and maximum statistical efficiency is desired. The FW approximation is most accurate for small sums, so this estimator may achieve lower MSE than $\hat{\theta}_3$ by building on the more efficient $\text{ml}_y$ statistic.
- **Cost:** FW approximation quality degrades as $k$ increases. More complex to compute.

---

#### Regime 3: $k \approx \sqrt{n}$ (Balanced — Transitional Regime)

**Standard choice:** $\hat{\theta}_3 = \exp(\mu(m_y))$
- Bias remains low. $g \approx \sqrt{n}$ groups provides adequate precision. The $k \cdot s_y^2$ amplification becomes noticeable but is not yet severe. This is approximately the MSE-optimal operating point.

**Robust alternative (outlier-resistant):** $\hat{\theta}_6$ (robust scale MoM with MAD)
- **When to use:** Same contamination concerns as Regime 2. At $k \approx \sqrt{n}$, $Y_j$ is more symmetric (CLT applies better), so the MAD → SD conversion is more accurate than in Regime 2.
- **Cost:** MAD efficiency loss (~37%) is the main penalty. The moderate $g$ means the median-of-absolute-deviations itself has noticeable finite-sample variance.

**Robust alternative (bias-correction + robustness):** $\hat{\theta}_4 = \exp(-\hat{\sigma}^2/2) \cdot \text{median}(Y_j)$
- **When to use:** Both outlier robustness (via median) and approximate unbiasedness are desired. This is the regime where estimator 4's bias **crosses zero** — the correction $\exp(-\hat{\sigma}^2/2)$ approximately cancels the median's upward drift.
- **Cost:** Sensitive to exact $k$ — small deviations shift the bias sign. The double variance penalty (median + $\hat{\sigma}^2$) begins to matter.

**Alternative (efficiency-focused):** $\hat{\theta}_5$ (FW bias-corrected log-mean)
- **When to use:** If efficiency is prioritized over robustness. FW approximation is degrading but may still provide MSE improvement. Use with caution.

---

#### Regime 4: $\sqrt{n} \ll k < n$ (Few Groups, Large Batches)

**Standard choice:** $\hat{\theta}_3 = \exp(\mu(m_y))$ or $\hat{\theta}_6$ (robust scale MoM)
- $\hat{\theta}_3$ bias remains low but variance grows substantially due to the $k \cdot s_y^2$ amplification. $s_y^2$ is estimated from few points (relative SE $\approx \sqrt{2/(g-1)}$), and multiplying by large $k$ produces noisy $\text{var}_x$.
- $\hat{\theta}_6$ trades some additional efficiency for protection against the most damaging outcome in this regime: a single extreme $Y_j$ inflating $s_y^2$ and corrupting $\hat{\sigma}^2$ entirely.

**Robust alternative (bias-correction + robustness):** $\hat{\theta}_4$
- **When to use:** The CLT ensures $Y_j$ is approximately symmetric, so the correction $\exp(-\hat{\sigma}^2/2)$ works well in expectation — bias is approximately zero. The median provides outlier resistance.
- **Cost:** **Severe double variance penalty.** Both the sample median and $\hat{\sigma}^2$ are estimated from very few groups. This is typically the highest-MSE estimator among the four, despite having the lowest bias. Only use when bias is the overriding concern and variance is secondary.

**Fallback (highly robust, high bias):** $\hat{\theta}_2$ (sample median)
- **When to use:** Distribution-free inference is required and the $\exp(\sigma^2/2)$ upward bias is acceptable or can be bounded. Simplest robust option.

**Avoid:** $\hat{\theta}_1$ (geometric mean) — severely biased; converges to $E[X]$, not $\exp(\mu)$.

---

#### Regime 5: $k = n$ (Single Group — Degenerate)

**No estimator works well.** With $g = 1$, $s_y^2$ is undefined, making estimators 3, 4, 6, and 7 non-computable. Estimators 1 and 2 both degenerate to $\bar{X}$, which estimates $E[X] = \exp(\mu + \sigma^2/2)$ — biased by factor $\exp(\sigma^2/2)$.

**Recommendations:**
- If any within-group data is accessible, use it to estimate $\sigma^2$ or individual-level statistics directly.
- If $\sigma^2$ is known from prior experiments or domain knowledge, apply $\hat{\mu} = \ln(\bar{X}) - \sigma^2/2$ and report $\exp(\hat{\mu})$ — this is estimator 3 with externally supplied $\sigma^2$.
- Otherwise, report $\bar{X}$ as an estimate of $E[X]$ and **document the $\exp(\sigma^2/2)$ bias factor explicitly**. For typical benchmarking $\sigma$ values (e.g., $\sigma \approx 0.1$–$0.3$), this factor is modest ($\exp(0.1^2/2) \approx 1.005$ to $\exp(0.3^2/2) \approx 1.046$).
- Redesign the experiment to have $g \geq 2$ groups (i.e., $k \leq n/2$) so that $\sigma^2$ becomes estimable.

---

### 8.2 Summary by Regime

| Regime | Standard choice | Robust (outliers) | Robust (misspecification) | Efficiency-focused | Avoid |
|--------|----------------|-------------------|--------------------------|-------------------|-------|
| $k = 1$ | $\hat{\theta}_1$ (MLE) | $\hat{\theta}_2$ (median) | $\hat{\theta}_2$ (median) | $\hat{\theta}_1$ (already optimal) | $\hat{\theta}_4$ (overcorrects) |
| $1 < k \ll \sqrt{n}$ | $\hat{\theta}_3$ (MoM) | $\hat{\theta}_6$ (robust MoM) | $\hat{\theta}_2$ (median) | $\hat{\theta}_5$ (FW) | $\hat{\theta}_1$ (biased), $\hat{\theta}_4$ (overcorrects) |
| $k \approx \sqrt{n}$ | $\hat{\theta}_3$ (MoM) | $\hat{\theta}_6$ (robust MoM) or $\hat{\theta}_4$ | $\hat{\theta}_2$ (median) | $\hat{\theta}_5$ (FW, cautious) | $\hat{\theta}_1$ (substantial bias) |
| $\sqrt{n} \ll k < n$ | $\hat{\theta}_3$ or $\hat{\theta}_6$ | $\hat{\theta}_6$ (robust MoM) | $\hat{\theta}_4$ (low bias) or $\hat{\theta}_2$ | — (variance dominates) | $\hat{\theta}_1$ (severely biased) |
| $k = n$ | None viable | — | — | — | All (degenerate) |

Where the "Robust (outliers)" column lists $\hat{\theta}_6$ (MAD-based), $\hat{\theta}_{10}$ (Rousseeuw–Croux $Q_n$) is a strictly better default when efficiency also matters: same ~50% breakdown, but ~82% vs. ~37% Gaussian efficiency. $\hat{\theta}_7$ (IQR) is dominated by $\hat{\theta}_6$ and omitted.

### 8.3 Benchmarking Context

In latency benchmarking with `bench-utils`:
- $X_i$ is per-operation latency
- $Y_j$ is the mean latency of a batch of $k$ operations
- The goal is to recover per-operation latency statistics from batched measurements

Estimator 3 is the recommended default: it explicitly uses the batch size $k$ to undo the batching effect. It is the only estimator that correctly targets $\exp(\mu)$ regardless of $k$ (provided $g \geq 2$).

For production use where outlier batches are a concern (e.g., GC pauses, scheduler preemption), a robust-scale variant of estimator 3 is preferred. Estimator 6 (MAD-based) is the simplest such option; estimator 10 (Rousseeuw–Croux $Q_n$) is better still, retaining the ~50% breakdown point while recovering most of the efficiency lost by the MAD (~82% vs. ~37% at the normal).

---

## 9. Key Mathematical Identities (Reference)

For the lognormal distribution:

$$\begin{aligned}
E[X] &= \exp(\mu + \sigma^2/2) \\
\text{Var}(X) &= \exp(2\mu + \sigma^2)(\exp(\sigma^2) - 1) \\
\text{Median}(X) &= \exp(\mu) \\
\text{Mode}(X) &= \exp(\mu - \sigma^2) \\
\text{CV}^2(X) &= \frac{\text{Var}(X)}{E[X]^2} = \exp(\sigma^2) - 1 \\
E[\ln X] &= \mu \\
\text{Var}(\ln X) &= \sigma^2
\end{aligned}$$

For the group means $Y_j$ (exact, for all $k$):

$$\begin{aligned}
E[Y_j] &= E[X] = \exp(\mu + \sigma^2/2) \\
\text{Var}(Y_j) &= \frac{\text{Var}(X)}{k} = \frac{\exp(2\mu + \sigma^2)(\exp(\sigma^2) - 1)}{k} \\
\text{CV}^2(Y_j) &= \frac{\exp(\sigma^2) - 1}{k}
\end{aligned}$$

Parameter recovery from moments (method of moments):

$$\begin{aligned}
\sigma^2 &= \ln\!\left(1 + \frac{\text{Var}(X)}{E[X]^2}\right) = \ln(1 + \text{CV}^2(X)) \\
\mu &= \ln(E[X]) - \frac{\sigma^2}{2}
\end{aligned}$$

These identities are the foundation of estimator 3 and explain why it works uniformly in $k$.

---

## References

1. Fenton, L. F. (1960). "The sum of log-normal probability distributions in scatter transmission systems." *IRE Transactions on Communications Systems*, 8(1), 57–67.
2. Schwartz, S. C., & Yeh, Y. S. (1982). "On the distribution function and moments of power sums with log-normal components." *Bell System Technical Journal*, 61(7), 1441–1462.
3. Beaulieu, N. C., & Xie, Q. (2004). "An optimal lognormal approximation to lognormal sum distributions." *IEEE Transactions on Vehicular Technology*, 53(2), 479–489.
4. Mehta, N. B., Wu, J., Molisch, A. F., & Zhang, J. (2007). "Approximating a sum of random variables with a lognormal distribution." *IEEE Transactions on Wireless Communications*, 6(7), 2690–2699.
5. Finney, D. J. (1941). "On the distribution of a variate whose logarithm is normally distributed." *Supplement to the Journal of the Royal Statistical Society*, 7(2), 155–161.
6. Crow, E. L., & Shimizu, K. (1988). *Lognormal Distributions: Theory and Applications*. Marcel Dekker.
7. Hampel, F. R., Ronchetti, E. M., Rousseeuw, P. J., & Stahel, W. A. (1986). *Robust Statistics: The Approach Based on Influence Functions*. Wiley.
