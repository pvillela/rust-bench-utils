use crate::BenchOut;
use basic_stats::{
    aok::{AokBasicStats, AokFloat},
    core::{AltHyp, Ci, HypTestResult, PositionWrtCi, SampleMoments, sample_mean},
    normal::{welch_ci, welch_df, welch_p, welch_t, welch_test},
};

/// Struct that holds references to the benchmark outputs of two closures (`f1` and `f2`) for comparison purposes.
///
/// All statistics involving differences refer to a value for `f1` minus the corresponding
/// value for `f2`. Similarly for ratios and other comparisons.
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

    /// Degrees of freedom for Welch's t statistic for
    /// `mean(ln(latency(f1))) - mean(ln(latency(f2)))` (where `ln` is the natural logarithm).
    pub fn welch_ln_df(&self) -> f64 {
        welch_df(&self.moments_ln_f1(), &self.moments_ln_f2()).aok()
    }

    /// p-value of Welch's two-sample t-test of the hypothesis that
    /// `median(latency(f1)) == median(latency(f2))`
    /// (equivalently, `mean(ln(latency(f1))) == mean(ln(latency(f2)))`, where `ln` is the natural logarithm).
    ///
    /// Assumes that both `latency(f1)` and `latency(f2)` are approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    pub fn welch_median_p(&self, alt_hyp: AltHyp) -> f64 {
        welch_p(&self.moments_ln_f1(), &self.moments_ln_f2(), alt_hyp).aok()
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

    /// Welch's two-sample t-test of the hypothesis that
    /// `median(latency(f1)) == median(latency(f2))`
    /// (equivalently, `mean(ln(latency(f1))) == mean(ln(latency(f2)))`, where `ln` is the natural logarithm),
    /// with alternative hypothesis `alt_hyp` and confidence level `(1 - alpha)`.
    ///
    /// Assumes that both `latency(f1)` and `latency(f2)` are approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    pub fn welch_median_test(&self, alt_hyp: AltHyp, alpha: f64) -> HypTestResult {
        welch_test(&self.moments_ln_f1(), &self.moments_ln_f2(), alt_hyp, alpha).aok()
    }
}

#[cfg(test)]
#[cfg(feature = "_dev_utils")]
mod test {
    use super::*;
    use crate::test_support::{
        HI_STDEV_LN, LO_STDEV_LN, lognormal_moments_ln, lognormal_moments_ln_jittered,
        lognormal_out, lognormal_out_jittered,
    };
    use basic_stats::{approx_eq, core::AcceptedHyp};

    const EPSILON: f64 = 0.001;
    const JITTER_EPSILON: f64 = EPSILON;
    const ALPHA: f64 = 0.05;

    fn are_eq_bench_out(out1: &BenchOut, out2: &BenchOut) -> bool {
        out1.recording_unit == out2.recording_unit
            && out1.summary() == out2.summary()
            && out1.sum == out2.sum
            && out1.sum2 == out2.sum2
            && out1.n_ln == out2.n_ln
            && out1.sum_ln == out2.sum_ln
            && out1.sum2_ln == out2.sum2_ln
    }

    #[test]
    fn test_comp() {
        let k = 80;
        let n_jitter = 7; // should be coprime with 2*k

        let sigma_lo = *LO_STDEV_LN;
        let sigma_hi = *HI_STDEV_LN;

        let mu1 = 8.;
        let out1 = lognormal_out(mu1, sigma_lo, k);
        let moments_ln1 = lognormal_moments_ln(mu1, sigma_lo, k);
        let out1j = lognormal_out_jittered(mu1, sigma_hi, k, n_jitter, JITTER_EPSILON);
        let moments_ln1j =
            lognormal_moments_ln_jittered(mu1, sigma_hi, k, n_jitter, JITTER_EPSILON);

        let median_ratio: f64 = 1.01;
        let mu2 = mu1 - median_ratio.ln();
        let out2j = lognormal_out_jittered(mu2, sigma_hi, k, n_jitter, JITTER_EPSILON);
        let moments_ln2j =
            lognormal_moments_ln_jittered(mu2, sigma_hi, k, n_jitter, JITTER_EPSILON);

        {
            let o1 = &out1;
            let m_ln1 = &moments_ln1;
            let o2 = &out1j;
            let m_ln2 = &moments_ln1j;
            let ratio_medians = 1.;
            let alt_hyp = AltHyp::Ne;
            let accepted_hyp = AcceptedHyp::Null;

            let comp = Comp::new(o1, o2);
            let f1_out = comp.f1_out();
            let f2_out = comp.f2_out();

            print!("o1: ");
            o1.print();
            print!("f1_out: ");
            f1_out.print();

            assert!(are_eq_bench_out(o1, f1_out));
            assert!(are_eq_bench_out(o2, f2_out));

            assert_eq!(f1_out.median() - f2_out.median(), comp.diff_medians_f1_f2());
            approx_eq!(ratio_medians, comp.ratio_medians_f1_f2(), EPSILON);
            assert_eq!(f1_out.mean() - f2_out.mean(), comp.mean_diff_f1_f2());
            assert_eq!(
                f1_out.mean_ln() - f2_out.mean_ln(),
                comp.mean_diff_ln_f1_f2()
            );
            approx_eq!(ratio_medians, comp.ratio_medians_f1_f2_from_lns(), EPSILON);
            assert_eq!(welch_t(m_ln1, m_ln2).unwrap(), comp.welch_ln_t());
            assert_eq!(welch_df(m_ln1, m_ln2).unwrap(), comp.welch_ln_df());
            assert_eq!(
                welch_p(m_ln1, m_ln2, alt_hyp).unwrap(),
                comp.welch_median_p(alt_hyp)
            );
            assert_eq!(
                welch_ci(m_ln1, m_ln2, ALPHA).unwrap(),
                comp.welch_ln_ci(ALPHA)
            );
            let ln_ci = welch_ci(m_ln1, m_ln2, ALPHA).unwrap();
            assert_eq!(Ci(ln_ci.0.exp(), ln_ci.1.exp()), comp.welch_ratio_ci(ALPHA));
            assert_eq!(
                PositionWrtCi::In,
                comp.welch_value_position_wrt_ratio_ci(ratio_medians, ALPHA)
            );
            assert_eq!(
                PositionWrtCi::In,
                comp.welch_value_position_wrt_ratio_ci(ratio_medians, ALPHA)
            );
            assert_eq!(
                accepted_hyp,
                comp.welch_median_test(alt_hyp, ALPHA).accepted()
            );
        }

        {
            let o1 = &out1;
            let m_ln1 = &moments_ln1;
            let o2 = &out2j;
            let m_ln2 = &moments_ln2j;
            let ratio_medians = median_ratio;
            let alt_hyp = AltHyp::Gt;
            let accepted_hyp = AcceptedHyp::Alt;

            let comp = Comp::new(o1, o2);
            let f1_out = comp.f1_out();
            let f2_out = comp.f2_out();

            print!("o1: ");
            o1.print();
            print!("f1_out: ");
            f1_out.print();

            assert!(are_eq_bench_out(o1, f1_out));
            assert!(are_eq_bench_out(o2, f2_out));

            assert_eq!(f1_out.median() - f2_out.median(), comp.diff_medians_f1_f2());
            approx_eq!(ratio_medians, comp.ratio_medians_f1_f2(), EPSILON);
            assert_eq!(f1_out.mean() - f2_out.mean(), comp.mean_diff_f1_f2());
            assert_eq!(
                f1_out.mean_ln() - f2_out.mean_ln(),
                comp.mean_diff_ln_f1_f2()
            );
            approx_eq!(ratio_medians, comp.ratio_medians_f1_f2_from_lns(), EPSILON);
            assert_eq!(welch_t(m_ln1, m_ln2).unwrap(), comp.welch_ln_t());
            assert_eq!(welch_df(m_ln1, m_ln2).unwrap(), comp.welch_ln_df());
            assert_eq!(
                welch_p(m_ln1, m_ln2, alt_hyp).unwrap(),
                comp.welch_median_p(alt_hyp)
            );
            assert_eq!(
                welch_ci(m_ln1, m_ln2, ALPHA).unwrap(),
                comp.welch_ln_ci(ALPHA)
            );
            let ln_ci = welch_ci(m_ln1, m_ln2, ALPHA).unwrap();
            assert_eq!(Ci(ln_ci.0.exp(), ln_ci.1.exp()), comp.welch_ratio_ci(ALPHA));
            assert_eq!(
                PositionWrtCi::In,
                comp.welch_value_position_wrt_ratio_ci(ratio_medians, ALPHA)
            );
            assert_eq!(
                PositionWrtCi::In,
                comp.welch_value_position_wrt_ratio_ci(ratio_medians, ALPHA)
            );
            println!("welch_ln_test={:?}", comp.welch_median_test(alt_hyp, ALPHA));
            assert_eq!(
                accepted_hyp,
                comp.welch_median_test(alt_hyp, ALPHA).accepted()
            );
        }
    }
}
