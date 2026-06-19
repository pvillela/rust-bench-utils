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

    /// Difference between the median of `f1`'s latencies and the median of `f2`'s latencies,
    /// in seconds.
    pub fn diff_medians_f1_f2(&self) -> FpSeconds {
        self.comp().diff_medians_f1_f2()
    }

    /// Ratio of the median of `f1`'s latencies to the median of `f2`'s latencies.
    ///
    /// Returns `f64::INFINITY` if the median of `f2` is zero, and `f64::NAN` if both
    /// medians are zero.
    ///
    /// # Panics
    ///
    /// Panics if `self.out_f1().n() == 0` or `self.out_f2().n() == 0`, since
    pub fn ratio_medians_f1_f2(&self) -> f64 {
        self.comp().ratio_medians_f1_f2()
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

    /// Estimated ratio of the median `f1` latency to the median `f2` latency,
    /// computed as the `exp()` of [`Self::mean_diff_ln_f1_f2`].
    ///
    /// # Panics
    ///
    /// Panics if `self.out_f1().n_nz == 0` or `self.out_f2().n_nz == 0`.
    pub fn ratio_medians_f1_f2_from_lns(&self) -> f64 {
        self.comp().ratio_medians_f1_f2_from_lns()
    }

    /// Welch's t statistic for the hypothesis that
    /// `mean(ln(latency(f1))) - mean(ln(latency(f2))) == ln_d0` (where `ln` is the natural logarithm, in the recording unit),
    /// or equivalently, `median(latency(f1)) / median(latency(f2)) == exp(ln_d0)`.
    ///
    /// Under the assumption that latencies are approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// Arguments:
    /// - `ln_d0`: hypothesized value of `mean(ln(latency(f1))) - mean(ln(latency(f2)))`, or equivalently,
    ///   `ln(median(latency(f1)) / median(latency(f2)))`.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `self.out_f1().n_nz <= 1`.
    /// - `self.out_f2().n_nz <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    pub fn welch_ln_t(&self, ln_d0: f64) -> f64 {
        self.comp().welch_ln_t(ln_d0)
    }

    /// Degrees of freedom for Welch's t statistic for
    /// `mean(ln(latency(f1))) - mean(ln(latency(f2)))` (where `ln` is the natural logarithm, in the recording unit).
    ///
    /// Under the assumption that latencies are approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    /// Thus, this statistic equivalently pertains to `ln(median(latency(f1)) / median(latency(f2)))`.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `self.out_f1().n_nz <= 1`.
    /// - `self.out_f2().n_nz <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    pub fn welch_ln_df(&self) -> f64 {
        self.comp().welch_ln_df()
    }

    /// p-value of Welch's two-sample t-test of the hypothesis that
    /// `mean(ln(latency(f1))) - mean(ln(latency(f2))) == ln_d0` (where `ln` is the natural logarithm, in the recording unit),
    /// or equivalently, `median(latency(f1)) / median(latency(f2)) == exp(ln_d0)`.
    ///
    /// Under the assumption that latencies are approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// Arguments:
    /// - `ln_d0`: hypothesized value of `mean(ln(latency(f1))) - mean(ln(latency(f2)))`, or equivalently,
    ///   `ln(median(latency(f1)) / median(latency(f2)))`.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `self.out_f1().n_nz <= 1`.
    /// - `self.out_f2().n_nz <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    pub fn welch_ln_p(&self, ln_d0: f64, alt_hyp: AltHyp) -> f64 {
        self.comp().welch_ln_p(ln_d0, alt_hyp)
    }

    /// Welch confidence interval for
    /// `mean(ln(latency(f1))) - mean(ln(latency(f2)))` (where `ln` is the natural logarithm, in the recording unit),
    /// with confidence level `(1 - alpha)`.
    ///
    /// Assumes that both `latency(f1)` and `latency(f2)` are approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// This is also the confidence interval for the difference of medians of logarithms under the above assumption.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `self.out_f1().n_nz <= 1`.
    /// - `self.out_f2().n_nz <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn welch_ln_ci(&self, alpha: f64) -> Ci {
        self.comp().welch_ln_ci(alpha)
    }

    /// Welch confidence interval for
    /// `median(latency(f1)) / median(latency(f2))`,
    /// with confidence level `(1 - alpha)`.
    ///
    /// Assumes that both `latency(f1)` and `latency(f2)` are approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `self.out_f1().n_nz <= 1`.
    /// - `self.out_f2().n_nz <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn welch_ratio_ci(&self, alpha: f64) -> Ci {
        self.comp().welch_ratio_ci(alpha)
    }

    /// Position of `value` with respect to the
    /// Welch confidence interval for
    /// `median(latency(f1)) / median(latency(f2))`,
    /// with confidence level `(1 - alpha)`.
    ///
    /// Assumes that both `latency(f1)` and `latency(f2)` are approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `self.out_f1().n_nz <= 1`.
    /// - `self.out_f2().n_nz <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn welch_value_position_wrt_ratio_ci(&self, value: f64, alpha: f64) -> PositionWrtCi {
        self.comp().welch_value_position_wrt_ratio_ci(value, alpha)
    }

    /// Welch's two-sample t-test of the hypothesis that
    /// `mean(ln(latency(f1))) - mean(ln(latency(f2))) == ln_d0` (where `ln` is the natural logarithm, in the recording unit),
    /// or equivalently, `median(latency(f1)) / median(latency(f2)) == exp(ln_d0)`.
    ///
    /// Under the assumption that latencies are approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// Arguments:
    /// - `ln_d0`: hypothesized value of `mean(ln(latency(f1))) - mean(ln(latency(f2)))`, or equivalently,
    ///   `ln(median(latency(f1)) / median(latency(f2)))`.
    /// - `alt_hyp`: alternative hypothesis.
    /// - `alpha`: confidence level is `1 - alpha`.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `self.out_f1().n_nz <= 1`.
    /// - `self.out_f2().n_nz <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn welch_ln_test(&self, ln_d0: f64, alt_hyp: AltHyp, alpha: f64) -> HypTestResult {
        self.comp().welch_ln_test(ln_d0, alt_hyp, alpha)
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
