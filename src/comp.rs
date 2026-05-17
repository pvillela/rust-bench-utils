use crate::{BenchCfg, BenchOut, PanicIfNeeded};
use basic_stats::{
    aok::Aok,
    core::{AltHyp, Ci, HypTestResult, PositionWrtCi, SampleMoments},
    normal::{welch_ci, welch_df, welch_p, welch_t, welch_test},
};

#[cfg(feature = "_experimental")]
use basic_stats::wilcoxon::RankSum;

/// Struct that holds references to the benchmark outputs of two closures (`f1` and `f2`) for comparison purposes.
///
/// All statistics involving differences refer to a value for `f1` minus the corresponding
/// value for `f2`. Similarly for ratios and other comparisons.
///
/// It should be noted that comparisons of latencies measured at different times are subject to distortion due to
/// time-dependent noise. See crate [`bench_diff`](https://docs.rs/bench_diff/latest/bench_diff/) for a discussion
/// of time-dependent noise and why the use `bench_diff` should be preferred for latency comparisons.
///
/// The `*_ln_*` methods provide statistics for `mean(ln(latency(f1))) - mean(ln(latency(f2)))`,
/// where `ln` is the natural logarithm.
/// Under the assumption that latency distributions are approximately log-normal,
/// `mean(ln(latency(f))) == ln(median(latency(f)))`.
/// This assumption is widely supported by performance analysis theory and empirical data.
/// Thus, the `*_ln_*` methods are useful for the analysis of differences of natural logarithms of median latencies,
/// or equivalently, the ratio of median latencies.
pub struct Comp<'a>(&'a BenchOut, &'a BenchOut);

impl<'a> Comp<'a> {
    /// # Panics
    /// Panics in any of the following conditions:
    /// - `f1_out` and `f2_out` don't have the same `recording_unit`.
    /// - `f1_out` and `f2_out` don't have the same `reporting_unit`.
    pub fn new(f1_out: &'a BenchOut, f2_out: &'a BenchOut) -> Self {
        assert_eq!(
            f1_out.recording_unit, f2_out.recording_unit,
            "`f1_out.recording_unit` and `f2_out.recording_unit` must be the same",
        );
        assert_eq!(
            f1_out.reporting_unit, f2_out.reporting_unit,
            "`f1_out.reporting_unit` and `f2_out.reporting_unit` must be the same",
        );
        Self(f1_out, f2_out)
    }

    /// The current value of [`BenchCfg::panic_on_error`].
    fn panic_on_error(&self) -> bool {
        let cfg = BenchCfg::get();
        BenchCfg::panic_on_error(&cfg)
    }

    /// Reference to the first benchmark output.
    pub fn out_f1(&self) -> &BenchOut {
        self.0
    }

    /// Reference to the second benchmark output.
    pub fn out_f2(&self) -> &BenchOut {
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
    ///
    /// # Panics
    ///
    /// Panics if `self.panic_on_error() == true` **and** `self.out_f1().n() == 0` or `self.out_f2().n() == 0`.
    pub fn mean_diff_f1_f2(&self) -> f64 {
        self.0.mean() - self.1.mean()
    }

    /// The difference between the mean of the natural logarithms of `f1`'s latencies and
    /// the mean of the natural logarithms of`f2`'s latencies.
    ///
    /// # Panics
    ///
    /// Panics if `self.panic_on_error() == true` **and** `self.out_f1().n_ln == 0` or `self.out_f2().n_ln == 0`.
    pub fn mean_diff_ln_f1_f2(&self) -> f64 {
        self.0.mean_ln() - self.1.mean_ln()
    }

    /// Estimated ratio of the median `f1` latency to the median `f2` latency,
    /// computed as the `exp()` of [`Self::mean_diff_ln_f1_f2`].
    ///
    /// # Panics
    ///
    /// Panics if `self.panic_on_error() == true` **and** `self.out_f1().n_ln == 0` or `self.out_f2().n_ln == 0`.
    pub fn ratio_medians_f1_f2_from_lns(&self) -> f64 {
        self.mean_diff_ln_f1_f2().exp()
    }

    fn moments_ln_f1(&self) -> SampleMoments {
        SampleMoments::new(self.0.n_ln, self.0.sum_ln, self.0.sum2_ln)
    }

    fn moments_ln_f2(&self) -> SampleMoments {
        SampleMoments::new(self.1.n_ln, self.1.sum_ln, self.1.sum2_ln)
    }

    // ==============
    // IMPORTANT NOTE
    // ==============
    // No need to adjust moments for recording and reporting units for the statistics below because
    // they only depend on the difference of means and the stdevs, all of which are invariant when
    // the conversion factor is the same for both samples. The observations and means of lns are both
    // shifted by the same value, i.e., ln(confersion_factor), and the stdevs are not impacted by the
    // shift of values and means.

    /// Welch's t statistic for the hypothesis that
    /// `mean(ln(latency(f1))) - mean(ln(latency(f2))) == ln_d0` (where `ln` is the natural logarithm), or equivalently,
    /// `median(latency(f1)) / median(latency(f2)) == exp(ln_d0)`.
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
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - `self.out_f1().n_ln <= 1`.
    /// - `self.out_f2().n_ln <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    pub fn welch_ln_t(&self, ln_d0: f64) -> f64 {
        welch_t(&self.moments_ln_f1(), &self.moments_ln_f2(), ln_d0).aok().panic_if_needed(
            self.panic_on_error(),
            "`number of observations <= 1` for either sample or `both standard deviations == 0`",
        )
    }

    /// Degrees of freedom for Welch's t statistic for
    /// `mean(ln(latency(f1))) - mean(ln(latency(f2)))` (where `ln` is the natural logarithm).
    ///
    /// Under the assumption that latencies are approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    /// Thus, this statistic equivalently pertains to `ln(median(latency(f1)) / median(latency(f2)))`.
    ///
    /// # Panics
    ///
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - `self.out_f1().n_ln <= 1`.
    /// - `self.out_f2().n_ln <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    pub fn welch_ln_df(&self) -> f64 {
        welch_df(&self.moments_ln_f1(), &self.moments_ln_f2()).aok().panic_if_needed(
            self.panic_on_error(),
            "`number of observations <= 1` for either sample or `both standard deviations == 0`",
        )
    }

    /// p-value of Welch's two-sample t-test of the hypothesis that
    /// `mean(ln(latency(f1))) - mean(ln(latency(f2))) == ln_d0` (where `ln` is the natural logarithm), or equivalently,
    /// `median(latency(f1)) / median(latency(f2)) == exp(ln_d0)`.
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
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - `self.out_f1().n_ln <= 1`.
    /// - `self.out_f2().n_ln <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    pub fn welch_ln_p(&self, ln_d0: f64, alt_hyp: AltHyp) -> f64 {
        welch_p(&self.moments_ln_f1(), &self.moments_ln_f2(), ln_d0, alt_hyp)
            .aok()
            .panic_if_needed(
                self.panic_on_error(),
                "`number of observations <= 1` for either sample or `both standard deviations == 0`",
            )
    }

    /// Welch confidence interval for
    /// `mean(ln(latency(f1))) - mean(ln(latency(f2)))` (where `ln` is the natural logarithm),
    /// with confidence level `(1 - alpha)`.
    ///
    /// Assumes that both `latency(f1)` and `latency(f2)` are approximately log-normal.
    /// This assumption is widely supported by performance analysis theory and empirical data.
    ///
    /// This is also the confidence interval for the difference of medians of logarithms under the above assumption.
    ///
    /// # Panics
    ///
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - `self.out_f1().n_ln <= 1`.
    /// - `self.out_f2().n_ln <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn welch_ln_ci(&self, alpha: f64) -> Ci {
        welch_ci(&self.moments_ln_f1(), &self.moments_ln_f2(), alpha)
            .aok()
            .panic_if_needed(
                self.panic_on_error(),
                "`number of observations <= 1` for either sample, `both standard deviations == 0`, or `alpha` not in open interval `(0, 1)`",
            )
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
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - `self.out_f1().n_ln <= 1`.
    /// - `self.out_f2().n_ln <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    /// - `alpha` not in open interval `(0, 1)`.
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
    ///
    /// # Panics
    ///
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - `self.out_f1().n_ln <= 1`.
    /// - `self.out_f2().n_ln <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn welch_value_position_wrt_ratio_ci(&self, value: f64, alpha: f64) -> PositionWrtCi {
        let ci = self.welch_ratio_ci(alpha);
        ci.position_of(value)
    }

    /// Welch's two-sample t-test of the hypothesis that
    /// `mean(ln(latency(f1))) - mean(ln(latency(f2))) == ln_d0` (where `ln` is the natural logarithm), or equivalently,
    /// `median(latency(f1)) / median(latency(f2)) == exp(ln_d0)`.
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
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - `self.out_f1().n_ln <= 1`.
    /// - `self.out_f2().n_ln <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn welch_ln_test(&self, ln_d0: f64, alt_hyp: AltHyp, alpha: f64) -> HypTestResult {
        welch_test(
            &self.moments_ln_f1(),
            &self.moments_ln_f2(),
            ln_d0,
            alt_hyp,
            alpha,
        )
        .aok()
        .panic_if_needed(
            self.panic_on_error(),
            "`number of observations <= 1` for either sample, `both standard deviations == 0`, or `alpha` not in open interval `(0, 1)`",
        )
    }

    #[cfg(feature = "_experimental")]
    /// Wilcoxon rank sum struct.
    fn rank_sum(&self) -> RankSum {
        let iter_f1 = self.0.hist.iter_recorded().map(|x| {
            let value = x.value_iterated_to();
            let count = x.count_at_value();
            (value as f64, count)
        });

        let iter_f2 = self.1.hist.iter_recorded().map(|x| {
            let value = x.value_iterated_to();
            let count = x.count_at_value();
            (value as f64, count)
        });

        RankSum::from_iters_with_counts(iter_f1, iter_f2)
            .aok()
            .panic_if_needed(
                self.panic_on_error(),
                // samples not in increasing order is impossible due to use of HdrHistogram
                "either sample is empty",
            )
    }

    #[cfg(feature = "_experimental")]
    /// Wilcoxon rank sum *W* statistic for `latency(f1)` and `latency(f2)`.
    ///
    /// Requires feature `_experimental`.
    pub fn wilcoxon_rank_sum_w(&self) -> f64 {
        self.rank_sum().w()
    }

    #[cfg(feature = "_experimental")]
    /// Wilcoxon rank sum normal approximation *z* value for `latency(f1)` and `latency(f2)`.
    ///
    /// Requires feature `_experimental`.
    ///
    /// # Panics
    ///
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - either sample is empty.
    /// - there are too many rank ties between the two samples.
    pub fn wilcoxon_rank_sum_z(&self) -> f64 {
        self.rank_sum().z().aok().panic_if_needed(
            self.panic_on_error(),
            "either sample is empty or too many rank ties",
        )
    }

    #[cfg(feature = "_experimental")]
    /// Wilcoxon rank sum normal approximation *p* value for `latency(f1)` and `latency(f2)`.
    ///
    /// Requires feature `_experimental`.
    ///
    /// # Panics
    ///
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - either sample is empty.
    /// - there are too many rank ties between the two samples.
    pub fn wilcoxon_rank_sum_p(&self, alt_hyp: AltHyp) -> f64 {
        self.rank_sum().z_p(alt_hyp).aok().panic_if_needed(
            self.panic_on_error(),
            "either sample is empty or too many rank ties",
        )
    }

    #[cfg(feature = "_experimental")]
    /// Wilcoxon rank sum test for `latency(f1)` and `latency(f2)`,
    /// with alternative hypothesis `alt_hyp` and confidence level `(1 - alpha)`.
    ///
    /// Requires feature `_experimental`.
    ///
    /// # Panics
    ///
    /// Panics if `self.panic_on_error() == true` **and** any of the following conditions is true:
    /// - either sample is empty.
    /// - there are too many rank ties between the two samples.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn wilcoxon_rank_sum_test(&self, alt_hyp: AltHyp, alpha: f64) -> HypTestResult {
        self.rank_sum()
            .z_test(alt_hyp, alpha)
            .aok()
            .panic_if_needed(
                self.panic_on_error(),
                "either sample is empty or too many rank ties or `alpha` not in open interval `(0, 1)`",
            )
    }
}

#[cfg(test)]
#[cfg(feature = "_experimental")]
#[cfg(feature = "_test_support")]
mod test {
    use super::*;
    use crate::test_support::{
        HI_STDEV_LN, LO_STDEV_LN, lognormal_moments_ln, lognormal_moments_ln_jittered,
        lognormal_out, lognormal_out_jittered,
    };
    use crate::{BenchCfg, LatencyUnit};
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
    fn test_comp_new_panics_on_reporting_unit_mismatch() {
        let saved_cfg = BenchCfg::get();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let cfg1 = BenchCfg::get();
            cfg1.with_reporting_unit(LatencyUnit::Nano).set();
            let out1 = lognormal_out(8., *LO_STDEV_LN, 5);

            let cfg2 = BenchCfg::get();
            cfg2.with_reporting_unit(LatencyUnit::Micro).set();
            let out2 = lognormal_out(8., *LO_STDEV_LN, 5);

            Comp::new(&out1, &out2);
        }));
        saved_cfg.set();
        assert!(
            result.is_err(),
            "expected Comp::new to panic on reporting unit mismatch"
        );
    }

    #[test]
    fn test_comp() {
        let k = 80;
        let n_jitter = 7; // should be coprime with 2*k

        let sigma_lo = *LO_STDEV_LN;
        let sigma_hi = *HI_STDEV_LN;

        let mu_a = 8.;
        let out_a = lognormal_out(mu_a, sigma_lo, k);
        let moments_ln_a = lognormal_moments_ln(mu_a, sigma_lo, k);
        let out_aj = lognormal_out_jittered(mu_a, sigma_hi, k, n_jitter, JITTER_EPSILON);
        let moments_ln_aj =
            lognormal_moments_ln_jittered(mu_a, sigma_hi, k, n_jitter, JITTER_EPSILON);

        let median_ratio_a_b: f64 = 1.01;
        let mu_b = mu_a - median_ratio_a_b.ln();
        let out_bj = lognormal_out_jittered(mu_b, sigma_hi, k, n_jitter, JITTER_EPSILON);
        let moments_ln_bj =
            lognormal_moments_ln_jittered(mu_b, sigma_hi, k, n_jitter, JITTER_EPSILON);

        #[derive(Debug)]
        struct TestArgs<'a> {
            ratio_medians: f64,
            ln_d0: f64,
            o1: &'a BenchOut,
            mom_ln1: &'a SampleMoments,
            o2: &'a BenchOut,
            mom_ln2: &'a SampleMoments,
            alt_hyp: AltHyp,
            accepted_hyp: AcceptedHyp,
        }

        let run_test = |args: TestArgs<'_>| {
            let TestArgs {
                ratio_medians,
                ln_d0,
                o1,
                mom_ln1,
                o2,
                mom_ln2,
                alt_hyp,
                accepted_hyp,
            } = args;

            println!(
                "ratio_medians={ratio_medians}, ln_d0={ln_d0}, alt_hyp={alt_hyp:?}, accepted_hyp={accepted_hyp:?}"
            );

            let comp = Comp::new(o1, o2);
            let f1_out = comp.out_f1();
            let f2_out = comp.out_f2();

            // print!("o1: ");
            // o1.print();
            // print!("f1_out: ");
            // f1_out.print();
            // print!("o2: ");
            // o2.print();
            // print!("f2_out: ");
            // f2_out.print();

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
            assert_eq!(
                welch_t(mom_ln1, mom_ln2, ln_d0).unwrap(),
                comp.welch_ln_t(ln_d0)
            );
            assert_eq!(welch_df(mom_ln1, mom_ln2).unwrap(), comp.welch_ln_df());
            assert_eq!(
                welch_p(mom_ln1, mom_ln2, ln_d0, alt_hyp).unwrap(),
                comp.welch_ln_p(ln_d0, alt_hyp)
            );
            assert_eq!(
                welch_ci(mom_ln1, mom_ln2, ALPHA).unwrap(),
                comp.welch_ln_ci(ALPHA)
            );
            let ln_ci = welch_ci(mom_ln1, mom_ln2, ALPHA).unwrap();
            assert_eq!(Ci(ln_ci.0.exp(), ln_ci.1.exp()), comp.welch_ratio_ci(ALPHA));
            assert_eq!(
                PositionWrtCi::In,
                comp.welch_value_position_wrt_ratio_ci(ratio_medians, ALPHA)
            );
            assert_eq!(
                PositionWrtCi::In,
                comp.welch_value_position_wrt_ratio_ci(ratio_medians, ALPHA)
            );
            println!(
                "welch_ln_test={:?}",
                comp.welch_ln_test(ln_d0, alt_hyp, ALPHA)
            );
            assert_eq!(
                accepted_hyp,
                comp.welch_ln_test(ln_d0, alt_hyp, ALPHA).accepted()
            );
        };

        {
            let ratio_medians = 1.0_f64;
            let ln_d0 = 0.;

            let o1 = &out_a;
            let mom_ln1 = &moments_ln_a;
            let o2 = &out_aj;
            let mom_ln2 = &moments_ln_aj;
            let alt_hyp = AltHyp::Ne;
            let accepted_hyp = AcceptedHyp::Null;

            let args = TestArgs {
                ratio_medians,
                ln_d0,
                o1,
                mom_ln1,
                o2,
                mom_ln2,
                alt_hyp,
                accepted_hyp,
            };
            run_test(args);
        }

        {
            let ratio_medians = median_ratio_a_b;
            let ln_d0 = 0.;

            let o1 = &out_a;
            let mom_ln1 = &moments_ln_a;
            let o2 = &out_bj;
            let mom_ln2 = &moments_ln_bj;
            let alt_hyp = AltHyp::Gt;
            let accepted_hyp = AcceptedHyp::Alt;

            let args = TestArgs {
                ratio_medians,
                ln_d0,
                o1,
                mom_ln1,
                o2,
                mom_ln2,
                alt_hyp,
                accepted_hyp,
            };
            run_test(args);
        }

        {
            let ratio_medians = median_ratio_a_b;
            let ln_d0 = ratio_medians.ln();

            let o1 = &out_a;
            let mom_ln1 = &moments_ln_a;
            let o2 = &out_bj;
            let mom_ln2 = &moments_ln_bj;
            let alt_hyp = AltHyp::Gt;
            let accepted_hyp = AcceptedHyp::Null;

            let args = TestArgs {
                ratio_medians,
                ln_d0,
                o1,
                mom_ln1,
                o2,
                mom_ln2,
                alt_hyp,
                accepted_hyp,
            };
            run_test(args);
        }
    }
}
