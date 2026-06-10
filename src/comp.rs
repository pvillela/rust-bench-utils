use crate::BenchOut;
use basic_stats::{
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
pub struct Comp<'a>(pub(crate) &'a BenchOut, pub(crate) &'a BenchOut);

impl<'a> Comp<'a> {
    /// Constructs a [`Comp`] from [`BenchOut`] references.
    ///
    /// # Panics
    /// Panics if `f1_out` and `f2_out` don't have the same `recording_unit`.
    pub fn new(f1_out: &'a BenchOut, f2_out: &'a BenchOut) -> Self {
        assert_eq!(
            f1_out.recording_unit, f2_out.recording_unit,
            "`f1_out.recording_unit` and `f2_out.recording_unit` must be the same",
        );
        Self(f1_out, f2_out)
    }

    /// Reference to the first benchmark output.
    pub fn out_f1(&self) -> &BenchOut {
        self.0
    }

    /// Reference to the second benchmark output.
    pub fn out_f2(&self) -> &BenchOut {
        self.1
    }

    /// Difference between the median of `f1`'s latencies and the median of `f2`'s latencies,
    /// in seconds.
    pub fn diff_medians_f1_f2(&self) -> f64 {
        self.0.median().as_secs_f64() - self.1.median().as_secs_f64()
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
        self.0.median().as_secs_f64() / self.1.median().as_secs_f64()
    }

    /// The difference between the mean of `f1`'s latencies and the mean of `f2`'s latencies,
    /// in seconds.
    ///
    /// # Panics
    ///
    /// Panics if `self.out_f1().n() == 0` or `self.out_f2().n() == 0`.
    pub fn mean_diff_f1_f2(&self) -> f64 {
        self.0.mean().as_secs_f64() - self.1.mean().as_secs_f64()
    }

    /// The difference between the mean of the natural logarithms of `f1`'s latencies and
    /// the mean of the natural logarithms of`f2`'s latencies.
    ///
    /// # Panics
    ///
    /// Panics if `self.out_f1().n_ln == 0` or `self.out_f2().n_ln == 0`.
    pub fn mean_diff_ln_f1_f2(&self) -> f64 {
        self.0.mean_ln() - self.1.mean_ln()
    }

    /// Estimated ratio of the median `f1` latency to the median `f2` latency,
    /// computed as the `exp()` of [`Self::mean_diff_ln_f1_f2`].
    ///
    /// # Panics
    ///
    /// Panics if `self.out_f1().n_ln == 0` or `self.out_f2().n_ln == 0`.
    pub fn ratio_medians_f1_f2_from_lns(&self) -> f64 {
        self.mean_diff_ln_f1_f2().exp()
    }

    fn moments_ln_f1(&self) -> SampleMoments {
        SampleMoments::new(self.0.n_ln, self.0.sum_ln, self.0.sum2_ln)
    }

    fn moments_ln_f2(&self) -> SampleMoments {
        SampleMoments::new(self.1.n_ln, self.1.sum_ln, self.1.sum2_ln)
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
    /// - `self.out_f1().n_ln <= 1`.
    /// - `self.out_f2().n_ln <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    pub fn welch_ln_t(&self, ln_d0: f64) -> f64 {
        welch_t(&self.moments_ln_f1(), &self.moments_ln_f2(), ln_d0).expect(
            "`number of observations <= 1` for either sample or `both standard deviations == 0`",
        )
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
    /// - `self.out_f1().n_ln <= 1`.
    /// - `self.out_f2().n_ln <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    pub fn welch_ln_df(&self) -> f64 {
        welch_df(&self.moments_ln_f1(), &self.moments_ln_f2()).expect(
            "`number of observations <= 1` for either sample or `both standard deviations == 0`",
        )
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
    /// - `self.out_f1().n_ln <= 1`.
    /// - `self.out_f2().n_ln <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    pub fn welch_ln_p(&self, ln_d0: f64, alt_hyp: AltHyp) -> f64 {
        welch_p(&self.moments_ln_f1(), &self.moments_ln_f2(), ln_d0, alt_hyp).expect(
            "`number of observations <= 1` for either sample or `both standard deviations == 0`",
        )
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
    /// - `self.out_f1().n_ln <= 1`.
    /// - `self.out_f2().n_ln <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn welch_ln_ci(&self, alpha: f64) -> Ci {
        welch_ci(&self.moments_ln_f1(), &self.moments_ln_f2(), alpha).expect("`number of observations <= 1` for either sample, `both standard deviations == 0`, or `alpha` not in open interval `(0, 1)`")
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
    /// Panics if any of the following conditions is true:
    /// - `self.out_f1().n_ln <= 1`.
    /// - `self.out_f2().n_ln <= 1`.
    /// - `self.out_f1().stdev_ln() == 0` and `self.out_f2().stdev_ln() == 0`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn welch_value_position_wrt_ratio_ci(&self, value: f64, alpha: f64) -> PositionWrtCi {
        let ci = self.welch_ratio_ci(alpha);
        ci.position_of(value)
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
        ).expect("`number of observations <= 1` for either sample, `both standard deviations == 0`, or `alpha` not in open interval `(0, 1)`")
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

        RankSum::from_iters_with_counts(iter_f1, iter_f2).expect(
            // samples not in increasing order is impossible due to use of HdrHistogram
            "either sample is empty",
        )
    }

    #[cfg(feature = "_experimental")]
    /// Wilcoxon rank sum *W* statistic for `latency(f1)` and `latency(f2)`.
    /// Gated by feature **"_experimental"**.
    pub fn wilcoxon_rank_sum_w(&self) -> f64 {
        self.rank_sum().w()
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
        self.rank_sum()
            .z()
            .expect("either sample is empty or too many rank ties")
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
        self.rank_sum()
            .z_p(alt_hyp)
            .expect("either sample is empty or too many rank ties")
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
        self.rank_sum().z_test(alt_hyp, alpha).expect(
            "either sample is empty or too many rank ties or `alpha` not in open interval `(0, 1)`",
        )
    }
}

#[cfg(test)]
#[cfg(feature = "_test")]
mod test {
    use super::*;
    use crate::multi::LatencySrc;
    use crate::multi::test_support::ConstLatencySrc;
    use crate::test_support::{
        HI_STDEV_LN, LO_STDEV_LN, lognormal_moments_ln, lognormal_moments_ln_jittered,
        lognormal_out, lognormal_out_jittered,
    };
    use crate::{BenchCfg, LatencyUnit};
    use basic_stats::{approx_eq, core::AcceptedHyp};
    use std::time::Duration;

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
    fn test_comp_new_panics_on_recording_unit_mismatch() {
        let result = {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let cfg1 = BenchCfg::default().with_recording_unit(LatencyUnit::Nano);
                let out1 = lognormal_out(&cfg1, 8., *LO_STDEV_LN, 5);

                let cfg2 = cfg1.with_recording_unit(LatencyUnit::Micro);
                let out2 = lognormal_out(&cfg2, 8., *LO_STDEV_LN, 5);

                Comp::new(&out1, &out2);
            }))
        };

        assert!(
            result.is_err(),
            "expected Comp::new to panic on recording unit mismatch"
        );
    }

    #[test]
    // cargo test --package bench_utils --lib --all-features -- comp::test::test_comp --exact --nocapture --include-ignored
    fn test_comp() {
        let cfg = BenchCfg::default();
        let ru = cfg.recording_unit();

        let samp_size = 12_800;
        let n_jitter = 7;

        let sigma_lo = *LO_STDEV_LN;
        let sigma_hi = *HI_STDEV_LN;

        let mu_a = 8.;
        let out_a = lognormal_out(&cfg, mu_a, sigma_lo, samp_size);
        let moments_ln_a = lognormal_moments_ln(ru, mu_a, sigma_lo, samp_size);
        let out_aj =
            lognormal_out_jittered(&cfg, mu_a, sigma_hi, samp_size, n_jitter, JITTER_EPSILON);
        let moments_ln_aj =
            lognormal_moments_ln_jittered(ru, mu_a, sigma_hi, samp_size, n_jitter, JITTER_EPSILON);

        let median_ratio_a_b: f64 = 1.01;
        let mu_b = mu_a - median_ratio_a_b.ln();
        let out_bj =
            lognormal_out_jittered(&cfg, mu_b, sigma_hi, samp_size, n_jitter, JITTER_EPSILON);
        let moments_ln_bj =
            lognormal_moments_ln_jittered(ru, mu_b, sigma_hi, samp_size, n_jitter, JITTER_EPSILON);

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

            assert!(are_eq_bench_out(o1, f1_out));
            assert!(are_eq_bench_out(o2, f2_out));

            assert_eq!(
                f1_out.median().as_secs_f64() - f2_out.median().as_secs_f64(),
                comp.diff_medians_f1_f2()
            );
            approx_eq!(ratio_medians, comp.ratio_medians_f1_f2(), EPSILON);
            assert_eq!(
                f1_out.mean().as_secs_f64() - f2_out.mean().as_secs_f64(),
                comp.mean_diff_f1_f2()
            );
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

    #[test]
    fn test_comp_panics_on_empty_sample() {
        let cfg = BenchCfg::default();
        let out1 = BenchOut::from_iter(&cfg, std::iter::empty::<Duration>());
        let mut src2 = ConstLatencySrc::new([Duration::from_millis(3)]);
        let out2 = BenchOut::from_iter(&cfg, src2.aggregate().take(10));
        let comp = Comp::new(&out1, &out2);

        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| comp.welch_ln_t(0.0)))
                .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| comp.welch_ln_df())).is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                comp.welch_ln_p(0.0, AltHyp::Ne)
            }))
            .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| comp.welch_ln_ci(0.05)))
                .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                comp.welch_ln_test(0.0, AltHyp::Ne, 0.05)
            }))
            .is_err()
        );
    }

    #[test]
    fn test_comp_panics_on_singleton_sample() {
        let cfg = BenchCfg::default();
        let mut src1 = ConstLatencySrc::new([Duration::from_millis(3)]);
        let out1 = BenchOut::from_iter(&cfg, src1.aggregate().take(10));
        let out2 = BenchOut::from_iter(&cfg, [Duration::from_millis(1)].into_iter());
        let comp = Comp::new(&out1, &out2);

        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| comp.welch_ln_t(0.0)))
                .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| comp.welch_ln_df())).is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                comp.welch_ln_p(0.0, AltHyp::Ne)
            }))
            .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| comp.welch_ln_ci(0.05)))
                .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                comp.welch_ln_test(0.0, AltHyp::Ne, 0.05)
            }))
            .is_err()
        );
    }

    #[test]
    fn test_comp_panics_on_both_stdev_zero() {
        let cfg = BenchCfg::default();
        let out1 = BenchOut::from_iter(
            &cfg,
            [Duration::from_millis(5), Duration::from_millis(5)].into_iter(),
        );
        let out2 = BenchOut::from_iter(
            &cfg,
            [Duration::from_millis(5), Duration::from_millis(5)].into_iter(),
        );
        let comp = Comp::new(&out1, &out2);

        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| comp.welch_ln_t(0.0)))
                .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| comp.welch_ln_df())).is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                comp.welch_ln_p(0.0, AltHyp::Ne)
            }))
            .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| comp.welch_ln_ci(0.05)))
                .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                comp.welch_ln_test(0.0, AltHyp::Ne, 0.05)
            }))
            .is_err()
        );
    }
}

#[cfg(test)]
#[cfg(feature = "_test")]
#[cfg(feature = "_experimental")]
// cargo test --package bench_utils --lib --all-features -- comp::test::wilcoxon_tests::test_wilcoxon_rank_sum_methods --exact --nocapture --include-ignored
mod wilcoxon_tests {
    use std::time::Duration;

    use basic_stats::core::AcceptedHyp;

    use super::*;
    use crate::{
        BenchCfg,
        multi::{LatencySrc, test_support::ConstLatencySrc},
        test_support::{LO_STDEV_LN, lognormal_out, lognormal_samp},
    };

    const ALPHA: f64 = 0.05;

    #[test]
    fn test_wilcoxon_rank_sum_methods() {
        let cfg = BenchCfg::default();

        let median_ratio_1_2: f64 = 1.05;
        let mu1 = 8.0;
        let mu2 = mu1 - median_ratio_1_2.ln();
        let samp_size = 20;
        let alt_hyp = AltHyp::Gt;

        let out1 = lognormal_out(&cfg, mu1, *LO_STDEV_LN, samp_size);
        let out2 = lognormal_out(&cfg, mu2, *LO_STDEV_LN, samp_size);

        let comp = Comp::new(&out1, &out2);

        let w = comp.wilcoxon_rank_sum_w();
        assert!(w > 0.0, "w should be positive, got {}", w);

        let z = comp.wilcoxon_rank_sum_z();
        assert!(z > 1.0, "z should be greater than 1.0, got {}", z);

        let p = comp.wilcoxon_rank_sum_p(alt_hyp);
        assert!(
            0.0 < p && p < 0.5,
            "p should be between 0.5 and 1, got {}",
            p
        );

        let result = comp.wilcoxon_rank_sum_test(alt_hyp, ALPHA);
        let accepted = result.accepted();
        assert_eq!(accepted, AcceptedHyp::Alt);
    }

    #[test]
    fn test_wilcoxon_empty_sample_panic() {
        let cfg = BenchCfg::default();
        let out1 = BenchOut::from_iter(&cfg, std::iter::empty::<Duration>());
        let mut src2 = ConstLatencySrc::new([Duration::from_millis(3)]);
        let out2 = BenchOut::from_iter(&cfg, src2.aggregate().take(10));
        let comp = Comp::new(&out1, &out2);

        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                comp.wilcoxon_rank_sum_w()
            }))
            .is_err()
        );
    }

    #[test]
    fn test_wilcoxon_equal_distribution_null() {
        let cfg = BenchCfg::default();
        let ru = cfg.recording_unit();
        let mu = 8.0;
        let sigma = *LO_STDEV_LN;
        let samp_size = 500;
        let durations: Vec<Duration> = lognormal_samp(mu, sigma, samp_size)
            .map(|v| ru.latency_from_f64(v))
            .collect();
        let out1 = BenchOut::from_iter(&cfg, durations.iter().cloned());
        let out2 = BenchOut::from_iter(&cfg, durations.iter().cloned());
        let comp = Comp::new(&out1, &out2);

        let p = comp.wilcoxon_rank_sum_p(AltHyp::Ne);
        assert!(p > 0.05, "expected p > 0.05, got {}", p);
    }
}
