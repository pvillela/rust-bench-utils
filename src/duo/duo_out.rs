use crate::{
    BenchOut, Comp, FpSeconds, multi,
    stats_types::{AltHyp, Ci, HypTestResult, PositionWrtCi},
};

/// Alias for [`multi::BenchOut<2>`](crate::multi::BenchOut<2>).
pub type DuoOut = multi::BenchOut<2>;

impl DuoOut {
    /// Returns a [`Comp`] comparing the two benchmark outputs.
    pub fn comp(&self) -> Comp<'_> {
        Comp(&self.arr[0], &self.arr[1])
    }

    /// Reference to the first benchmark output.
    pub fn out_f1(&self) -> &BenchOut {
        &self.arr[0]
    }

    /// Reference to the second benchmark output.
    pub fn out_f2(&self) -> &BenchOut {
        &self.arr[1]
    }

    /// Difference between the medians of recorded values for `f1` and `f2`, respectively, in [`FpSeconds`].
    /// in seconds.
    pub fn diff_medians_f1_f2_r(&self) -> FpSeconds {
        self.comp().diff_medians_f1_f2_r()
    }

    /// Ratio between the medians of recorded values for `f1` and `f2`, respectively.
    ///
    /// Returns `f64::INFINITY` if the median for `f2` is zero, and `f64::NAN` if both
    /// medians are zero.
    ///
    /// # Panics
    ///
    /// Panics if `self.out_f1().n() == 0` or `self.out_f2().n() == 0`.
    pub fn ratio_medians_f1_f2_r(&self) -> f64 {
        self.comp().ratio_medians_f1_f2_r()
    }

    /// Difference between the robust median estimates for `f1` and `f2`, respectively, in [`FpSeconds`].
    pub fn diff_medians_f1_f2_rob(&self) -> FpSeconds {
        self.comp().diff_medians_f1_f2_rob()
    }

    /// Ratio between the robust median estimates for `f1` and `f2`, respectively.
    ///
    /// Returns `f64::INFINITY` if the median for `f2` is zero, and `f64::NAN` if both
    /// medians are zero.
    ///
    /// # Panics
    ///
    /// Panics if `self.out_f1().n() == 0` or `self.out_f2().n() == 0`.
    pub fn ratio_medians_f1_f2_rob(&self) -> f64 {
        self.comp().ratio_medians_f1_f2_rob()
    }

    /// The difference between the mean of `f1`'s latencies and the mean of `f2`'s latencies,
    /// in seconds.
    ///
    /// # Panics
    ///
    /// Panics if `self.out_f1().n() == 0` or `self.out_f2().n() == 0`.
    pub fn mean_diff_f1_f2(&self) -> FpSeconds {
        self.comp().mean_diff_f1_f2()
    }

    /// The difference between the mean of the natural logarithms of `f1`'s latencies and
    /// the mean of the natural logarithms of`f2`'s latencies.
    ///
    /// # Panics
    ///
    /// Panics if `self.out_f1().n_nz == 0` or `self.out_f2().n_nz == 0`.
    pub fn mean_diff_ln_f1_f2(&self) -> f64 {
        self.comp().mean_diff_ln_f1_f2()
    }

    /// Difference between the robust mean estimates for `f1` and `f2`, respectively, in [`FpSeconds`].
    pub fn mean_diff_f1_f2_rob(&self) -> FpSeconds {
        self.comp().mean_diff_f1_f2_rob()
    }

    /// Ratio between the robust mean estimates for `f1` and `f2`, respectively.
    ///
    /// Returns `f64::INFINITY` if the mean for `f2` is zero, and `f64::NAN` if both
    /// means are zero.
    ///
    /// # Panics
    ///
    /// Panics if `self.out_f1().n() == 0` or `self.out_f2().n() == 0`.
    pub fn mean_ratio_f1_f2_rob(&self) -> f64 {
        self.comp().mean_ratio_f1_f2_rob()
    }

    /// Ratio between the (naive arithmetic) means of `f1` and `f2`. Delegates to [`Comp::ratio_means_f1_f2`].
    pub fn ratio_means_f1_f2(&self) -> f64 {
        self.comp().ratio_means_f1_f2()
    }

    /// Welch's two-sample t statistic for `mean(latency(f1)) - mean(latency(f2)) == d0`.
    /// Delegates to [`Comp::welch_mean_t`].
    pub fn welch_mean_t(&self, d0: FpSeconds) -> f64 {
        self.comp().welch_mean_t(d0)
    }

    /// Degrees of freedom for Welch's t statistic for `mean(latency(f1)) - mean(latency(f2))`.
    /// Delegates to [`Comp::welch_mean_df`].
    pub fn welch_mean_df(&self) -> f64 {
        self.comp().welch_mean_df()
    }

    /// p-value of Welch's two-sample t-test of `mean(latency(f1)) - mean(latency(f2)) == d0`.
    /// Delegates to [`Comp::welch_mean_p`].
    pub fn welch_mean_p(&self, d0: FpSeconds, alt_hyp: AltHyp) -> f64 {
        self.comp().welch_mean_p(d0, alt_hyp)
    }

    /// Welch confidence interval for the difference of means, with confidence level `(1 - alpha)`.
    /// Delegates to [`Comp::welch_mean_diff_ci`].
    pub fn welch_mean_diff_ci(&self, alpha: f64) -> (FpSeconds, FpSeconds) {
        self.comp().welch_mean_diff_ci(alpha)
    }

    /// Position of `value` with respect to the Welch difference-of-means confidence interval.
    /// Delegates to [`Comp::welch_value_position_wrt_mean_diff_ci`].
    pub fn welch_value_position_wrt_mean_diff_ci(
        &self,
        value: FpSeconds,
        alpha: f64,
    ) -> PositionWrtCi {
        self.comp().welch_value_position_wrt_mean_diff_ci(value, alpha)
    }

    /// Welch's two-sample t-test of `mean(latency(f1)) - mean(latency(f2)) == d0`.
    /// Delegates to [`Comp::welch_mean_test`].
    pub fn welch_mean_test(&self, d0: FpSeconds, alt_hyp: AltHyp, alpha: f64) -> HypTestResult {
        self.comp().welch_mean_test(d0, alt_hyp, alpha)
    }

    /// Delta-method confidence interval for the ratio of means. Delegates to [`Comp::ratio_means_ci`].
    pub fn ratio_means_ci(&self, alpha: f64) -> Ci {
        self.comp().ratio_means_ci(alpha)
    }

    /// Position of `value` with respect to the delta-method ratio-of-means confidence interval.
    /// Delegates to [`Comp::welch_value_position_wrt_ratio_means_ci`].
    pub fn welch_value_position_wrt_ratio_means_ci(&self, value: f64, alpha: f64) -> PositionWrtCi {
        self.comp().welch_value_position_wrt_ratio_means_ci(value, alpha)
    }

    /// Percentile bootstrap confidence interval for the ratio of means.
    /// Delegates to [`Comp::ratio_means_boot_ci`].
    pub fn ratio_means_boot_ci(&self, alpha: f64) -> Ci {
        self.comp().ratio_means_boot_ci(alpha)
    }

    #[cfg(feature = "_experimental")]
    /// Wilcoxon rank sum *W* statistic for `latency(f1)` and `latency(f2)`.
    /// Gated by feature **"_experimental"**.
    pub fn wilcoxon_rank_sum_w(&self) -> f64 {
        self.comp().wilcoxon_rank_sum_w()
    }

    #[cfg(feature = "_experimental")]
    /// Wilcoxon rank sum normal approximation *z* value for `latency(f1)` and `latency(f2)`.
    /// Gated by feature **"_experimental"**.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - either sample is empty.
    /// - there are too many rank ties between the two samples.
    pub fn wilcoxon_rank_sum_z(&self) -> f64 {
        self.comp().wilcoxon_rank_sum_z()
    }

    #[cfg(feature = "_experimental")]
    /// Wilcoxon rank sum normal approximation *p* value for `latency(f1)` and `latency(f2)`.
    /// Gated by feature **"_experimental"**.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - either sample is empty.
    /// - there are too many rank ties between the two samples.
    pub fn wilcoxon_rank_sum_p(&self, alt_hyp: AltHyp) -> f64 {
        self.comp().wilcoxon_rank_sum_p(alt_hyp)
    }

    #[cfg(feature = "_experimental")]
    /// Wilcoxon rank sum test for `latency(f1)` and `latency(f2)`,
    /// with alternative hypothesis `alt_hyp` and confidence level `(1 - alpha)`.
    /// Gated by feature **"_experimental"**.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - either sample is empty.
    /// - there are too many rank ties between the two samples.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn wilcoxon_rank_sum_test(&self, alt_hyp: AltHyp, alpha: f64) -> HypTestResult {
        self.comp().wilcoxon_rank_sum_test(alt_hyp, alpha)
    }
}
