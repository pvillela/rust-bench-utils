# Equivalence of regime thresholds based on $\mathrm{CV}(Y)$ and $\hat{S}$

The equivalence holds for two stacked reasons:

1. Exact, given the FW lognormal shape: for a lognormal, $\mathrm{CV}(Y)^2 = e^{\sigma_Y^2} - 1$, so $\sigma_Y$ (what $\hat{S}$ estimates) and $\mathrm{CV}(Y)$ are monotone one-to-one transforms of each other — thresholds on one are thresholds on the other.
  2. First-order, with no distributional assumption: for any positive $Y$ concentrated near its mean, $\ln Y \approx \ln\mathrm{E}[Y] + (Y-\mathrm{E}[Y])/\mathrm{E}[Y]$, hence $\mathrm{SD}(\ln Y) \approx \mathrm{CV}(Y)$. Since the diagnostic's job is precisely to detect "concentrated enough to be near-normal," the approximation is trustworthy right
         at the decision boundary: $\hat{S}=0.3 \leftrightarrow \mathrm{CV}\approx 0.31$, $\hat{S}=0.6 \leftrightarrow \mathrm{CV}\approx 0.66$.