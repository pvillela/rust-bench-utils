use basic_stats::{
    aok::{AokBasicStats, AokFloat},
    core::{AltHyp, Ci, HypTestResult, PositionWrtCi, SampleMoments, sample_mean},
    normal::{welch_ci, welch_df, welch_t, welch_test},
};

use crate::BenchOut;

/// Struct that holds references to the benchmark outputs of two closures (`f1` and `f2`) for comparison purposes.
pub struct Comp<'a>(&'a BenchOut, &'a BenchOut);

impl<'a> Comp<'a> {
    pub fn new(f1_out: &'a BenchOut, f2_out: &'a BenchOut) -> Self {
        Self(f1_out, f2_out)
    }

    pub fn f1_out(&self) -> &BenchOut {
        self.0
    }

    pub fn f2_out(&self) -> &BenchOut {
        self.1
    }

    /// Difference between the median of `f1`'s latencies and the median of `f2`'s latencies.
    pub fn diff_medians_f1_f2(&self) -> f64 {
        self.0.median() - self.1.median()
    }

    /// Ratio of the median of `f1`'s latencies to the median of `f2`'s latencies.
    pub fn ratio_medians_f1_f2(&self) -> f64 {
        self.0.median() / self.1.median()
    }

    /// The difference between the mean of `f1`'s latencies and the mean of `f2`'s latencies.
    pub fn mean_diff_f1_f2(&self) -> f64 {
        let m1 = sample_mean(self.0.n(), self.0.sum as f64).aok();
        let m2 = sample_mean(self.1.n(), self.1.sum as f64).aok();
        m1 - m2
    }

    /// The difference between the mean of the natural logarithms of `f1`'s latencies and
    /// the mean of the natural logarithms of`f2`'s latencies.
    pub fn mean_diff_ln_f1_f2(&self) -> f64 {
        let m1 = sample_mean(self.0.n(), self.0.sum_ln).aok();
        let m2 = sample_mean(self.1.n(), self.1.sum_ln).aok();
        m1 - m2
    }

    /// Estimated ratio of the median `f1` latency to the median `f2` latency,
    /// computed as the `exp()` of [`Self::mean_diff_ln_f1_f2`].
    pub fn ratio_medians_f1_f2_from_lns(&self) -> f64 {
        self.mean_diff_ln_f1_f2().exp()
    }

    fn moments_ln_f1(&self) -> SampleMoments {
        SampleMoments::new(self.0.hist.len(), self.0.sum_ln, self.0.sum2_ln)
    }

    fn moments_ln_f2(&self) -> SampleMoments {
        SampleMoments::new(self.1.hist.len(), self.1.sum_ln, self.1.sum2_ln)
    }

    /// Welch's t statistic for
    /// `mean(ln(latency(f1))) - mean(ln(latency(f2)))` (where `ln` is the natural logarithm).
    pub fn welch_ln_t(&self) -> f64 {
        welch_t(&self.moments_ln_f1(), &self.moments_ln_f2()).aok()
    }

    /// Degrees of freedom for Welch's t-test for
    /// `mean(ln(latency(f1))) - mean(ln(latency(f2)))` (where `ln` is the natural logarithm).
    pub fn welch_ln_df(&self) -> f64 {
        welch_df(&self.moments_ln_f1(), &self.moments_ln_f2()).aok()
    }

    /// Welch confidence interval for
    /// `mean(ln(latency(f1))) - mean(ln(latency(f2)))` (where `ln` is the natural logarithm),
    /// with confidence level `(1 - alpha)`.
    ///
    /// Assumes that both `latency(f1)` and `latency(f2)` are approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// This is also the confidence interval for the difference of medians of logarithms under the above assumption.
    pub fn welch_ln_ci(&self, alpha: f64) -> Ci {
        welch_ci(&self.moments_ln_f1(), &self.moments_ln_f2(), alpha).aok()
    }

    /// Welch confidence interval for
    /// `median(latency(f1)) / median(latency(f2))`,
    /// with confidence level `(1 - alpha)`.
    ///
    /// Assumes that both `latency(f1)` and `latency(f2)` are approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    pub fn welch_ratio_ci(&self, alpha: f64) -> Ci {
        let Ci(log_low, log_high) = self.welch_ln_ci(alpha);
        let low = log_low.exp();
        let high = log_high.exp();
        Ci(low, high)
    }

    /// Position of `value` with respect to the
    /// Welch confidence interval for
    /// `median(latency(f1)) / median(latency(f2))`,
    /// with confidence level `(1 - alpha)`.
    ///
    /// Assumes that both `latency(f1)` and `latency(f2)` are approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    pub fn welch_value_position_wrt_ratio_ci(&self, value: f64, alpha: f64) -> PositionWrtCi {
        let ci = self.welch_ratio_ci(alpha);
        ci.position_of(value)
    }

    /// Welch's test of the hypothesis that
    /// `median(latency(f1)) == median(latency(f2))`,
    /// with alternative hypothesis `alt_hyp` and confidence level `(1 - alpha)`.
    ///
    /// Assumes that both `latency(f1)` and `latency(f2)` are approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    pub fn welch_ln_test(&self, alt_hyp: AltHyp, alpha: f64) -> HypTestResult {
        welch_test(&self.moments_ln_f1(), &self.moments_ln_f2(), alt_hyp, alpha).aok()
    }
}
