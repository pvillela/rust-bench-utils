# Analysis of Lognormal Median Estimators Under Batch Aggregation

## 1. Setup and Notation

Let $X_1, \ldots, X_n \sim \text{LogNormal}(\mu, \sigma^2)$ IID, where the population median is $\text{med}(X) = e^{\mu}$ and the population mean is $\mathbb{E}[X] = e^{\mu + \sigma^2/2}$.

Let $k \mid n$. Partition the $n$ observations into $g = n/k$ groups of size $k$. Define group means:

$$Y_j = \frac{1}{k} \sum_{i \in \text{group}_j} X_i, \quad j = 1, \ldots, g$$

Define:
- $m_y = \frac{1}{g} \sum_{j=1}^g Y_j$ — grand mean of group means (= sample mean of all $X_i$)
- $s_y^2 = \frac{1}{g-1} \sum_{j=1}^g (Y_j - m_y)^2$ — sample variance of group means
- $\text{ml}_y = \frac{1}{g} \sum_{j=1}^g \ln(Y_j)$ — mean of log group means
- $\text{var}_x = k \cdot s_y^2$ — estimates $\text{Var}(X)$
- $\hat{\sigma} = \sqrt{\ln\!\big(1 + \text{var}_x / m_y^2\big)}$ — estimates $\sigma$
- $\mu(v) = \ln(v) - \hat{\sigma}^2 / 2$

## 2. Distribution of $Y_j$ as $k$ Varies

This is the key driver of estimator behavior.

| Regime | Distribution of $Y_j$ |
|--------|----------------------|
| $k=1$ | $Y_j = X_j \sim \text{LogNormal}(\mu, \sigma^2)$ exactly |
| $1 < k \ll \sqrt{n}$ | Nearly lognormal; sum of a few lognormals is well-approximated by another lognormal (Fenton-Wilkinson). Many groups ($g \gg 1$). |
| $k \sim \sqrt{n}$ | Transition regime: $g \approx k \approx \sqrt{n}$. $Y_j$ begins to look normal (CLT), but lognormal approximation is still usable for small-moderate $\sigma$. |
| $\sqrt{n} \ll k < n$ | $Y_j$ is approximately normal by CLT with mean $\mathbb{E}[X]$ and variance $\text{Var}(X)/k$. Very few groups ($g$ small). |
| $k=n$ | Single group: $g=1$, $Y_1 = \bar{X}_n$ (grand mean). No within-group information. |

Key asymptotic facts:
- $\mathbb{E}[Y_j] = \mathbb{E}[X] = e^{\mu + \sigma^2/2}$ for all $k$
- $\text{Var}(Y_j) = \text{Var}(X) / k$ for all $k$
- $\text{med}(Y_j) \to e^{\mu}$ as $k \to 1$; $\text{med}(Y_j) \to \mathbb{E}[X] = e^{\mu + \sigma^2/2}$ as $k \to \infty$ (since for normal RVs, median = mean)

For $k>1$, $\mathbb{E}[\ln(Y_j)] \neq \mu$. By the delta method (valid for large $k$):

$$\mathbb{E}[\ln(Y_j)] \approx \mu + \frac{\sigma^2}{2} - \frac{e^{\sigma^2} - 1}{2k}$$

This approaches $\mu + \sigma^2/2$ as $k \to \infty$. At $k=1$, the delta method is not valid; the exact result $\mathbb{E}[\ln(X)] = \mu$ holds.

---

## 3. Estimator Analysis

### 3.1 Estimator 1: $\hat{\theta}_1 = \exp(\text{ml}_y)$

Exponentiates the mean of the log-transformed group means.

#### $k=1$
$\text{ml}_y = \frac{1}{n}\sum \ln(X_i)$. This is the sample mean of $\mathcal{N}(\mu, \sigma^2)$ variates — the MLE of $\mu$. $\hat{\theta}_1$ is the MLE of the median.

- **Bias:** $\mathbb{E}[e^{\text{ml}_y}] = e^{\mu} \cdot e^{\sigma^2/(2n)}$ — upward bias of $O(1/n)$. Asymptotically unbiased.
- **Robustness:** Good. The log-transform compresses the heavy right tail of the lognormal, making the mean on the log scale insensitive to large $X$ values. However, very small $X_i$ produce large negative $\ln(X_i)$, which can pull $\text{ml}_y$ down.
- **Efficiency:** Asymptotic variance $= \sigma^2 e^{2\mu} / n$ (attains Cramér-Rao lower bound; asymptotically efficient).

#### $1 < k \ll \sqrt{n}$
$\ln(Y_j)$ is no longer exactly normal, but $Y_j$ remains approximately lognormal. $\text{ml}_y$ estimates $\mathbb{E}[\ln(Y)]$, which is close to $\mu$ for small $k$. Bias grows slowly, and — because the variance is essentially stable (see below) — this growing bias, not variance inflation, is the dominant cost of increasing $k$.
- **Efficiency:** $\text{Var}(\text{ml}_y) = \frac{k}{n}\text{Var}(\ln Y_j)$. Under the Fenton-Wilkinson approximation $\text{Var}(\ln Y_j) \approx \ln\!\big(1 + (e^{\sigma^2}-1)/k\big)$, so
$$\text{Var}(\text{ml}_y) \approx \frac{k}{n}\,\ln\!\Big(1 + \frac{e^{\sigma^2}-1}{k}\Big).$$
This increases **monotonically** from $\sigma^2/n$ at $k=1$ and **saturates at $(e^{\sigma^2}-1)/n$** as $k \to \infty$ — it does **not** grow linearly in $k$. The efficiency-loss factor relative to $k=1$ is bounded by $(e^{\sigma^2}-1)/\sigma^2$, independent of $k$; for $\sigma^2 \ll 1$ this factor $\approx 1$, so batching costs almost nothing in variance.

#### $k \sim \sqrt{n}$
$\mathbb{E}[\ln(Y)]$ has shifted noticeably away from $\mu$ toward $\mu + \sigma^2/2$. $\hat{\theta}_1$ overestimates the median by a factor approaching $\exp(\sigma^2/2 - (e^{\sigma^2}-1)/(2k))$. The lognormal model for $Y_j$ is breaking down.
- **Efficiency:** Intermediate along the monotonic path from $\sigma^2/n$ (at $k=1$) toward the saturating ceiling $(e^{\sigma^2}-1)/n$. Variance grows with $k$ but has nearly stabilized.

#### $\sqrt{n} \ll k < n$
Large bias. $Y_j$ is nearly normal, $\ln(Y_j)$ is not. $\text{ml}_y$ estimates something unrelated to $\mu$. Only $g$ observations to compute $\text{ml}_y$ — high variance.
- **Efficiency:** $\text{Var}(\text{ml}_y) \approx (e^{\sigma^2} - 1)/n$ by delta method (variance stabilizes for large $k$, but the quantity estimated is wrong). Irrelevant given dominating bias.

#### $k=n$
$\text{ml}_y = \ln(\bar{X}_n)$, so $\hat{\theta}_1 = \bar{X}_n$. Estimates $\mathbb{E}[X]$, not the median. Overestimates by factor $e^{\sigma^2/2}$.

### 3.2 Estimator 2: $\hat{\theta}_2 = \text{sample median of } \{Y_j\}$

#### $k=1$
The sample median of the original $X_i$. Directly estimates the population median $e^{\mu}$. Asymptotically unbiased.

- **Bias:** Negligible asymptotically. Small-sample bias is $O(1/n)$.
- **Robustness:** Excellent — the sample median is the canonical robust location estimator, with 50% breakdown point.
- **Efficiency:** Asymptotic variance $= \frac{\pi}{2} \cdot \frac{\sigma^2 e^{2\mu}}{n}$. Asymptotic relative efficiency vs. MLE (estimator 1): $\text{ARE} = 2/\pi \approx 0.637$ — the sample median needs ~1.57× more data for equal precision.

#### $1 < k \ll \sqrt{n}$
$\text{med}(Y_j)$ shifts from $e^{\mu}$ toward $\mathbb{E}[X] = e^{\mu + \sigma^2/2}$ as $k$ increases. The bias is positive and grows with $k$. Still many $Y_j$, so the sample median has reasonable precision.
- **Efficiency:** The asymptotic variance of the sample median of $g$ IID $Y_j$ is $O(1/n)$ with **scaling independent of $k$** (since $\text{Var}(Y_j) = \text{Var}(X)/k$ and $g = n/k$ cancel). The leading *constant*, however, transitions with the shape of $Y_j$: it is $\frac{\pi}{2}\cdot\frac{\sigma^2 e^{2\mu}}{n}$ at $k=1$ (lognormal-shaped $Y_j$) and approaches $\frac{\pi}{2}\cdot\frac{\text{Var}(X)}{n}$ as $Y_j$ becomes normal-shaped for large $k$ (the two differ by a factor $e^{\sigma^2}(e^{\sigma^2}-1)/\sigma^2$). So the variance scaling is stable, but the bias grows with $k$.

#### $k \sim \sqrt{n}$
Significant positive bias. Only $\approx \sqrt{n}$ observations for the sample median.
- **Efficiency:** Asymptotic variance scaling unchanged from the small-$k$ case — $O(1/n)$, same as mean-based estimators, with the leading constant near $\frac{\pi}{2} \cdot \frac{\text{Var}(X)}{n}$ now that $Y_j$ is close to normal-shaped. Finite-sample performance degrades with fewer $Y_j$.

#### $\sqrt{n} \ll k < n$
Very few $Y_j$ — sample median has high variance. Additionally, $Y_j \approx \text{Normal}$, so $\text{med}(Y_j) \approx \mathbb{E}[X]$, far from $e^{\mu}$. Large bias.

#### $k=n$
One observation. "Sample median" = $\bar{X}_n$. Same as estimator 1 at $k=n$.

### 3.3 Estimator 3: $\hat{\theta}_3 = \exp(\mu(m_y)) = m_y / \exp(\hat{\sigma}^2/2)$

Method-of-moments: estimates $\mathbb{E}[X]$ via $m_y$, estimates $\text{Var}(X)$ via $k \cdot s_y^2$, then inverts the lognormal identities $\mathbb{E}[X] = e^{\mu + \sigma^2/2}$ and $\text{Var}(X) = e^{2\mu + \sigma^2}(e^{\sigma^2} - 1)$.

Crucially, $m_y = \bar{X}_n$ is the grand mean of all $n$ observations (since $m_y = \frac{k}{n}\sum_j Y_j = \frac{1}{n}\sum_i X_i$), so the precision of the mean estimate is $\text{Var}(X)/n$ **independent of $k$**.

- **Bias:** Inputs $m_y$ and $k \cdot s_y^2$ are unbiased for $\mathbb{E}[X]$ and $\text{Var}(X)$. However, the nonlinear transformation (ratio, log) introduces Jensen bias. Asymptotically unbiased for all $k < n$.

#### $k=1$
Best case. $n$ observations for both mean and variance. Consistent, uses all data efficiently. Note: for $k=1$, estimator 1 (the MLE) is more efficient because it uses the sufficient statistics ($\sum \ln X_i$, $\sum (\ln X_i)^2$) rather than moments on the original scale.

- **Robustness:** Poor. Sample mean and variance on the original scale are sensitive to large $X_i$ in the heavy right tail. A single extreme observation inflates both $m_y$ and $\text{var}_x$, distorting $\hat{\sigma}^2$ and thus $\hat{\theta}_3$.
- **Efficiency:** Less efficient than the MLE (estimator 1) for $k=1$. Asymptotic variance $> \sigma^2 e^{2\mu}/n$ because it uses sample moments on the original scale rather than the sufficient statistics $(\sum \ln X_i, \sum (\ln X_i)^2)$. The efficiency gap grows with $\sigma^2$; for $\sigma^2 \ll 1$ the two estimators are nearly equivalent.

#### $1 < k \ll \sqrt{n}$
Works well. $k \cdot s_y^2$ still has many degrees of freedom ($g-1$). Bias is small.
- **Efficiency:** $\text{Var}(\hat{\sigma}^2)$ grows gradually as $g = n/k$ decreases; for modest $k$ the variance penalty is mild.

#### $k \sim \sqrt{n}$
Variance of $\hat{\sigma}^2$ grows. $\approx \sqrt{n}$ degrees of freedom for variance estimation.
- **Efficiency:** $\text{Var}(\hat{\sigma}^2)$ scales as $O(k/n)$ — the overall asymptotic variance grows with $k$ due to imprecise $\sigma^2$ estimation.

#### $\sqrt{n} \ll k < n$
Very few $Y_j$. $s_y^2$ is unreliable — $\hat{\sigma}^2 \approx 0$ if $\text{var}_x \ll m_y^2$, degenerating toward the naive $\hat{\theta} = m_y$; $\hat{\sigma}^2$ is also wildly unstable with few $Y_j$.

#### $k=n$
$g=1$, $s_y^2 = 0$ (or undefined). $\hat{\sigma} = 0$, so $\hat{\theta}_3 = m_y = \bar{X}_n$. Degenerates to grand mean.

### 3.4 Estimator 4: $\hat{\theta}_4 = \text{sample median of } \{\exp(\mu(Y_j))\}$

$\exp(\mu(Y_j)) = \exp(\ln(Y_j) - \hat{\sigma}^2/2) = Y_j / \exp(\hat{\sigma}^2/2)$. This divides each $Y_j$ by the estimated mean/median ratio, then takes the sample median.

#### $k=1$
$Y_j / e^{\hat{\sigma}^2/2} = X_j / e^{\hat{\sigma}^2/2}$. Since $X_j \sim \text{LogNormal}(\mu, \sigma^2)$, each transformed value has distribution $\text{LogNormal}(\mu - \hat{\sigma}^2/2, \sigma^2)$. The sample median estimates $\exp(\mu - \hat{\sigma}^2/2) \to \exp(\mu - \sigma^2/2)$.

- **Bias:** Downward by factor $e^{-\sigma^2/2}$. For $\sigma=1$, this is $\approx 0.61$ — a 39% downward bias. This estimator is **not suitable** for $k=1$.
- **Robustness:** Good (uses median), but irrelevant given the bias.
- **Efficiency:** The sample median's asymptotic variance is $\frac{\pi}{2} \cdot \frac{\sigma^2 e^{2\mu}}{n}$ (same as estimator 2 at $k=1$). But the $e^{-\sigma^2/2}$ bias dwarfs any efficiency consideration.

#### $1 < k \ll \sqrt{n}$
Bias magnitude shrinks as $k$ grows, because $\text{med}(Y_j)$ moves toward $\mathbb{E}[X]$, and dividing by $e^{\hat{\sigma}^2/2}$ increasingly gives the right answer. Still negatively biased.

#### $k \sim \sqrt{n}$
Interesting regime. $Y_j$ is becoming normal, so $\text{med}(Y_j) \approx \mathbb{E}[Y_j] = \mathbb{E}[X]$. The correction factor $e^{\hat{\sigma}^2/2}$ then correctly transforms $\mathbb{E}[X] \mapsto e^{\mu}$. Bias crosses from negative toward zero. Approximate unbiasedness is achievable.

#### $\sqrt{n} \ll k < n$
The **only estimator** that is approximately unbiased in this regime. Since $Y_j$ is approximately normal, $\text{med}(Y_j) \approx \mathbb{E}[X]$, and $\hat{\theta}_4 \approx \mathbb{E}[X] / e^{\hat{\sigma}^2/2} \approx e^{\mu}$. Caveats: few $Y_j$ for the sample median (high variance), and $\hat{\sigma}^2$ is also noisy.
- **Efficiency:** Asymptotic variance $\approx \frac{\pi}{2} \cdot \frac{\text{Var}(X)}{n} \cdot \left(1 + O(1/g)\right)$ where the $O(1/g)$ term accounts for $\hat{\sigma}^2$ estimation error. The leading term $\frac{\pi}{2} \cdot \frac{e^{2\mu+\sigma^2}(e^{\sigma^2}-1)}{n}$ is $\frac{\pi}{2} \cdot \frac{e^{\sigma^2}(e^{\sigma^2}-1)}{\sigma^2}$ times larger than the $k=1$ MLE variance — a substantial efficiency penalty for largish $\sigma^2$, but the only game in town for unbiased estimation in this regime.

#### $k=n$
$g=1$, single $Y$. $\hat{\sigma} = 0$, so $\hat{\theta}_4 = Y_1 = \bar{X}_n$. Degenerates.

This estimator has the unique property that it *improves* (in bias) as $k$ increases from 1, crosses through approximate unbiasedness, and only degrades at $k \approx n$ due to insufficient data.

---

## 4. Additional Noteworthy Estimators

### 4.1 $\hat{\theta}_5$: Direct sample median of $\{X_i\}$

If the individual $X_i$ are available (bypassing the grouping), the sample median of all $n$ observations is the best estimator: robust (50% BP), asymptotically unbiased for $e^{\mu}$, and requires no modeling of the group-mean distribution. This is the baseline against which all grouped-data estimators should be judged.

### 4.2 $\hat{\theta}_6$: Bias-corrected log-scale MLE

$$\hat{\theta}_6 = \exp\!\left(\text{ml}_y - \frac{s^2_{\ln Y}}{2g}\right)$$

where $s^2_{\ln Y} = \frac{1}{g-1}\sum(\ln Y_j - \text{ml}_y)^2$ and $g = n/k$.

For $k=1$, this is the standard small-sample bias correction for the lognormal MLE: it removes the $e^{\sigma^2/(2n)}$ upward bias, making it approximately unbiased to $O(1/n^2)$. For $k>1$, the $s^2_{\ln Y}/(2g)$ correction addresses only the Jensen (convexity-of-exp) bias from exponentiating a sample mean. It does **not** correct for the fundamental $\mathbb{E}[\ln(Y_j)] \neq \mu$ shift that occurs for $k>1$. The two biases have opposite signs for moderate $k$, so the correction may incidentally reduce total bias in some regimes, but this is not guaranteed.

### 4.3 $\hat{\theta}_7$: Trimmed log-scale mean

$$\hat{\theta}_7 = \exp(\text{trimmed mean of } \ln(Y_j))$$

Trimming (e.g., 10% each tail) before exponentiating provides robustness against extreme $\ln(Y_j)$ values. For $k=1$, this protects against very small $X_i$ (which become large negative $\ln$ values) and very large $X_i$ (large positive $\ln$ values). For $k>1$, the same benefit applies but does not address the fundamental $\mathbb{E}[\ln(Y)] \neq \mu$ issue.

### 4.4 $\hat{\theta}_8$: Adaptive combination

**Decision rule:** Compute the estimated coefficient of variation of $Y_j$: $\widehat{\text{CV}}_Y = s_y / m_y$. When $\widehat{\text{CV}}_Y < 0.3$ **and** $g = n/k \ge 5$, the $Y_j$ are approximately normal (median ≈ mean) with enough groups for a reliable sample median — use estimator 4. When $\widehat{\text{CV}}_Y \ge 0.3$ or $g < 5$, the $Y_j$ retain lognormal character or there are too few groups — use estimator 1 (or 6). Since $\text{CV}_Y = \sqrt{e^{\sigma^2}-1}/\sqrt{k}$, the threshold $\widehat{\text{CV}}_Y = 0.3$ pins $k \approx (e^{\sigma^2}-1)/0.09$ for a given $\sigma^2$: the rule adapts directly to the near-normality of $Y_j$ (the property that actually matters), and its correspondence to $k \approx \sqrt{n}$ holds only for a particular $n$. The threshold can be calibrated via simulation for the specific $\sigma^2$ and $n$ of interest.

---

## 5. Summary Table

| Estimator | $k=1$ | $1 < k \ll \sqrt{n}$ | $k \sim \sqrt{n}$ | $\sqrt{n} \ll k < n$ | $k=n$ |
|-----------|-------|---------------------|-------------------|---------------------|-------|
| **1.** $\exp(\text{ml}_y)$ | ✓ MLE, efficient, asymptotically unbiased; attains CRLB<br>✗ Slight upward bias $O(1/n)$; sensitive to tiny $X$ | ✓ Log transform still compresses tails<br>✗ $\mathbb{E}[\ln(Y)]$ drifts from $\mu$, bias grows | ✗ Significant positive bias; lognormal approx for $Y_j$ failing | ✗ Large bias; $\ln(Y_j)$ ≈ $\ln(\text{normal})$, unrelated to $\mu$ | ✗ Degenerates to $\bar{X}_n$; estimates mean, not median |
| **2.** $\text{med}(Y_j)$ | ✓ Robust, unbiased for $e^\mu$, direct; ARE = 2/π ≈ 0.64 vs MLE<br>✗ Less efficient than MLE | ✓ Still robust<br>✗ $\text{med}(Y_j) \to \mathbb{E}[X]$, positive bias grows; fewer $Y_j$ | ✗ Large positive bias; $\approx \sqrt{n}$ obs for median | ✗ Large bias; very few $Y_j$, high variance | ✗ Degenerates to $\bar{X}_n$ |
| **3.** $\exp(\mu(m_y))$ | ✓ Consistent, uses all data; $m_y$ precision independent of $k$<br>✗ Less efficient than MLE (both for $k=1$ and increasingly for $\sigma^2 \gg 0$); **not robust** (moments) | ✓ Method-of-moments still valid; reasonable bias<br>✗ $\hat{\sigma}^2$ variance grows with $k$ | ✗ $\hat{\sigma}^2$ increasingly noisy; $\text{Var}(\hat{\sigma}^2) = O(k/n)$ | ✗ $\hat{\sigma}^2$ unreliable; few $Y_j$ for variance | ✗ $s_y^2=0 \Rightarrow \hat{\sigma}=0$; degenerates to $\bar{X}_n$ |
| **4.** $\text{med}(e^{\mu(Y_j)})$ | ✗ **Large negative bias** ($e^{-\sigma^2/2}$); unsuitable | ✗ Still negatively biased, though shrinking | ✓/✗ Bias crosses toward zero; interesting transition regime | ✓ **Only estimator approx. unbiased here** (median≈mean for normal $Y_j$); leading AsVar ≈ (π/2)·Var(X)/n<br>✗ Few $Y_j$, high variance | ✗ Degenerates to $\bar{X}_n$ |
| **5.** $\text{med}(X_i)$ | ✓ Best overall: robust, unbiased, direct<br>✗ Requires individual data | ✓ Same<br>✗ Same | ✓ Same<br>✗ Same | ✓ Same<br>✗ Same | ✓ Same<br>✗ Same |
| **6.** $\exp(\text{ml}_y - \frac{s^2_{\ln Y}}{2g})$ | ✓ Bias-corrected MLE, $O(1/n^2)$ bias<br>✗ Assumes normality of $\ln(Y_j)$ | ✓ Reduces small-$k$ bias<br>✗ $\ln(Y_j)$ not exactly normal | ✗ Model mis-specified | ✗ Poor | ✗ Poor |
| **7.** Trimmed $\exp(\text{ml}_y)$ | ✓ Robust to extreme $\ln(X)$<br>✗ Slight efficiency loss vs. MLE | ✓ Still robust<br>✗ Doesn't fix $\mathbb{E}[\ln(Y)]$ shift | ✗ Same fundamental issue as estimator 1 | ✗ Same as estimator 1 | ✗ Same as estimator 1 |
| **8.** Adaptive (CV-based) | ✓ Delegates to estimator 1/6; inherits MLE efficiency<br>✗ Same as estimator 1/6 | ✓ Same as estimator 1/6<br>✗ Same | ✓/✗ Picks between estimator 1/6 and 4 based on $\widehat{\text{CV}}_Y$; requires $g \ge 5$ for estimator 4 branch; may misclassify near the CV threshold | ✓ Delegates to estimator 4; gains its approx. unbiasedness<br>✗ Inherits estimator 4's high variance; requires $g \ge 5$ | ✗ No choice possible (single $Y$); degenerates |


### 5.1 Asymptotic Variance Scaling (Efficiency Summary)

All variances are asymptotic ($n \to \infty$, $k$ fixed) and reported up to the common factor $e^{2\mu}$. "—" means the estimator is unusable in that regime due to bias.

| Estimator | $k=1$ | $k>1$ (small $k$) | $k \gg 1$ (large $k$, CLT regime) |
|-----------|-------|-------------------|-----------------------------------|
| **1.** $\exp(\text{ml}_y)$ | $\sigma^2 / n$ ✓ CRLB | $\nearrow$ toward $(e^{\sigma^2}-1)/n$ (saturating; loss factor $\le (e^{\sigma^2}-1)/\sigma^2$) ¶ | $\approx (e^{\sigma^2}-1)/n$ (biased) |
| **2.** $\text{med}(Y_j)$ | $\frac{\pi}{2} \cdot \sigma^2 / n$ | $\frac{\pi}{2} \cdot \text{Var}(X)/(n e^{2\mu})$ † | — (biased) |
| **3.** $\exp(\mu(m_y))$ | $> \sigma^2/n$ ‡ | $> \sigma^2/n$ (grows with $k$) | — (unstable $\hat{\sigma}^2$) |
| **4.** $\text{med}(e^{\mu(Y_j)})$ | — (biased) | — (biased) | $\approx \frac{\pi}{2} \cdot \frac{\text{Var}(X)}{n e^{2\mu}}$ § |

† Scaling independent of $k$ asymptotically — the $k$ cancels out of $\text{Var}(Y_j)/g$ — but the leading constant transitions from $\frac{\pi}{2}\sigma^2 e^{2\mu}/n$ (lognormal-shaped $Y_j$, small $k$) to $\frac{\pi}{2}\text{Var}(X)/n$ (normal-shaped $Y_j$, large $k$). And the quantity estimated shifts.

¶ Not linear in $k$: $\text{Var}(\text{ml}_y) \approx \frac{k}{n}\ln(1 + (e^{\sigma^2}-1)/k)$ rises monotonically from $\sigma^2/n$ and saturates at $(e^{\sigma^2}-1)/n$. For $\sigma^2 \ll 1$ the loss factor $\to 1$.

‡ Exact ARE depends on $\sigma^2$; close to 1 for $\sigma^2 \ll 1$, degrades for $\sigma^2 \gg 1$.

§ Plus an $O(1/g)$ term from $\hat{\sigma}^2$ estimation error. Leading term is $\frac{\pi}{2} \cdot \frac{e^{\sigma^2}(e^{\sigma^2}-1)}{n}$.

---

## 6. Recommendations

1. **If individual $X_i$ are available:** Use the sample median (estimator 5). No grouped-data estimator can outperform it.

2. **If only grouped means $Y_j$ are available:**
   - **$k=1$ (no grouping):** Estimator 1 (or bias-corrected estimator 6 for small $n$). Estimator 2 (sample median) is a robust alternative.
   - **Small $k$ ($1 < k \ll \sqrt{n}$):** Estimator 1 or 6. Bias is manageable and log-scale estimators are efficient.
   - **Moderate $k$ ($k \sim \sqrt{n}$):** Estimator 4 begins to become viable as $Y_j$ approaches normality. Estimator 3 (method of moments) is an alternative if $\sigma^2$ is small enough that the variance of $\hat{\sigma}^2$ remains acceptable. Estimator 8 (adaptive) automatically selects between estimator 1/6 and estimator 4 based on the data. At this boundary, bias-variance tradeoffs are estimator-specific and simulation at the target $(n, k, \sigma^2)$ is advisable.
   - **Large $k$ ($\sqrt{n} \ll k < n$):** Estimator 4 is the only approximately unbiased option, exploiting the near-normality of $Y_j$. Accept the higher variance from few groups.
   - **$k=n$:** Cannot reliably estimate the median from grouped data alone. At minimum, an estimate of $\sigma^2$ from external knowledge is required; otherwise fall back to $\bar{X}_n$ with the acknowledgment that it estimates the mean, not the median.

3. **If $k$ is a design choice:** $k=1$ (no grouping) is optimal for median estimation. Grouping sacrifices median-estimation accuracy — but for the log-scale estimator 1 the sacrifice is almost entirely *bias* (the $\mathbb{E}[\ln Y] - \mu$ drift toward $\sigma^2/2$), not variance, since estimator 1's variance stays bounded by $(e^{\sigma^2}-1)/n$ regardless of $k$. If grouping is required for other reasons (privacy, storage, computation), keep $k$ as small as possible to control that bias, unless $k$ is large enough that estimator 4 becomes viable — in which case the optimal $k$ trades off estimator 4's improving bias against its increasing variance.

4. **Robustness-conscious pipelines:** For $k=1$, prefer the sample median (estimator 2) or trimmed log-scale mean (estimator 7) when the data may contain contamination or when $\sigma$ is large (heavy tails). For $k>1$, estimator 4 naturally uses the median internally and inherits robustness.
