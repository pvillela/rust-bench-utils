use crate::{
    BenchOut, DEFAULT_MEAN_CI_RESAMPLES, FpSeconds,
    dev_support::{DEFAULT_BOOTSTRAP_SEED, two_sample_ratio_means_ci},
};
use basic_stats::{
    core::{AltHyp, Ci, HypTestResult, PositionWrtCi, SampleMoments},
    normal::{welch_ci, welch_df, welch_p, welch_t, welch_test, z_alpha},
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
/// The `welch_mean_*` methods provide parametric two-sample inference for the difference of means
/// `mean(latency(f1)) - mean(latency(f2))`, and [`Self::ratio_means_ci`] for the ratio of means. The
/// mean is the location parameter that batching preserves in expectation, so these are valid under
/// batching with no log-normality assumption. The robust companions ([`Self::mean_diff_f1_f2_rob`],
/// [`Self::mean_ratio_f1_f2_rob`], [`Self::ratio_means_boot_ci`]) resist contamination.
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

    /// Difference between the medians of recorded values for `f1` and `f2`, respectively, in [`FpSeconds`].
    pub fn diff_medians_f1_f2_r(&self) -> FpSeconds {
        self.0.median_r() - self.1.median_r()
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
        self.0.median_r().as_f64() / self.1.median_r().as_f64()
    }

    /// Difference between the robust median estimates for `f1` and `f2`, respectively, in [`FpSeconds`].
    pub fn diff_medians_f1_f2_rob(&self) -> FpSeconds {
        self.0.median_rob() - self.1.median_rob()
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
        self.0.median_rob().as_f64() / self.1.median_rob().as_f64()
    }

    /// The difference between the mean of `f1`'s latencies and the mean of `f2`'s latencies,
    /// in [`FpSeconds`].
    ///
    /// # Panics
    ///
    /// Panics if `self.out_f1().n() == 0` or `self.out_f2().n() == 0`.
    pub fn mean_diff_f1_f2(&self) -> FpSeconds {
        self.0.mean() - self.1.mean()
    }

    /// The difference between the mean of the natural logarithms of `f1`'s latencies and
    /// the mean of the natural logarithms of`f2`'s latencies.
    ///
    /// # Panics
    ///
    /// Panics if `self.out_f1().n_nz == 0` or `self.out_f2().n_nz == 0`.
    pub fn mean_diff_ln_f1_f2(&self) -> f64 {
        self.0.mean_ln_r() - self.1.mean_ln_r()
    }

    /// Difference between the robust mean estimates for `f1` and `f2`, respectively, in [`FpSeconds`].
    pub fn mean_diff_f1_f2_rob(&self) -> FpSeconds {
        self.0.mean_rob() - self.1.mean_rob()
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
        self.0.mean_rob().as_f64() / self.1.mean_rob().as_f64()
    }

    /// Ratio between the (naive arithmetic) means of `f1` and `f2`.
    ///
    /// # Panics
    /// Panics if `self.out_f1().n() == 0` or `self.out_f2().n() == 0`.
    pub fn ratio_means_f1_f2(&self) -> f64 {
        self.0.mean().as_f64() / self.1.mean().as_f64()
    }

    fn moments_mean_f1(&self) -> SampleMoments {
        SampleMoments::new(self.0.n_r(), self.0.sum, self.0.sum2)
    }

    fn moments_mean_f2(&self) -> SampleMoments {
        SampleMoments::new(self.1.n_r(), self.1.sum, self.1.sum2)
    }

    /// Welch's two-sample t statistic for the hypothesis that
    /// `mean(latency(f1)) - mean(latency(f2)) == d0`.
    ///
    /// Under batching the recorded batch means are approximately normal (CLT) and their grand means
    /// are unbiased for the true per-execution means, so Welch's t applies directly to the raw
    /// recorded values with no log-normality assumption.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `self.out_f1().n_r() <= 1`.
    /// - `self.out_f2().n_r() <= 1`.
    /// - `self.out_f1().stdev_r() == 0` and `self.out_f2().stdev_r() == 0`.
    pub fn welch_mean_t(&self, d0: FpSeconds) -> f64 {
        welch_t(&self.moments_mean_f1(), &self.moments_mean_f2(), d0.into()).expect(
            "`number of recorded values <= 1` for either sample or `both standard deviations == 0`",
        )
    }

    /// Degrees of freedom for Welch's t statistic for `mean(latency(f1)) - mean(latency(f2))`.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `self.out_f1().n_r() <= 1`.
    /// - `self.out_f2().n_r() <= 1`.
    /// - `self.out_f1().stdev_r() == 0` and `self.out_f2().stdev_r() == 0`.
    pub fn welch_mean_df(&self) -> f64 {
        welch_df(&self.moments_mean_f1(), &self.moments_mean_f2()).expect(
            "`number of recorded values <= 1` for either sample or `both standard deviations == 0`",
        )
    }

    /// p-value of Welch's two-sample t-test of the hypothesis that
    /// `mean(latency(f1)) - mean(latency(f2)) == d0`.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `self.out_f1().n_r() <= 1`.
    /// - `self.out_f2().n_r() <= 1`.
    /// - `self.out_f1().stdev_r() == 0` and `self.out_f2().stdev_r() == 0`.
    pub fn welch_mean_p(&self, d0: FpSeconds, alt_hyp: AltHyp) -> f64 {
        welch_p(
            &self.moments_mean_f1(),
            &self.moments_mean_f2(),
            d0.into(),
            alt_hyp,
        )
        .expect(
            "`number of recorded values <= 1` for either sample or `both standard deviations == 0`",
        )
    }

    /// Welch confidence interval for the difference of means
    /// `mean(latency(f1)) - mean(latency(f2))`, expressed as a pair of [`FpSeconds`] (low, high),
    /// with confidence level `(1 - alpha)`.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `self.out_f1().n_r() <= 1`.
    /// - `self.out_f2().n_r() <= 1`.
    /// - `self.out_f1().stdev_r() == 0` and `self.out_f2().stdev_r() == 0`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn welch_mean_diff_ci(&self, alpha: f64) -> (FpSeconds, FpSeconds) {
        let Ci(low, high) = welch_ci(&self.moments_mean_f1(), &self.moments_mean_f2(), alpha).expect("`number of recorded values <= 1` for either sample, `both standard deviations == 0`, or `alpha` not in open interval `(0, 1)`");
        (low.into(), high.into())
    }

    /// Position of `value` with respect to the Welch confidence interval for the difference of means
    /// `mean(latency(f1)) - mean(latency(f2))`, with confidence level `(1 - alpha)`.
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`Self::welch_mean_diff_ci`].
    pub fn welch_value_position_wrt_mean_diff_ci(
        &self,
        value: FpSeconds,
        alpha: f64,
    ) -> PositionWrtCi {
        let (low, high) = self.welch_mean_diff_ci(alpha);
        if value < low {
            PositionWrtCi::Below
        } else if value > high {
            PositionWrtCi::Above
        } else {
            PositionWrtCi::In
        }
    }

    /// Welch's two-sample t-test of the hypothesis that
    /// `mean(latency(f1)) - mean(latency(f2)) == d0`.
    ///
    /// # Panics
    ///
    /// Panics if any of the following conditions is true:
    /// - `self.out_f1().n_r() <= 1`.
    /// - `self.out_f2().n_r() <= 1`.
    /// - `self.out_f1().stdev_r() == 0` and `self.out_f2().stdev_r() == 0`.
    /// - `alpha` not in open interval `(0, 1)`.
    pub fn welch_mean_test(&self, d0: FpSeconds, alt_hyp: AltHyp, alpha: f64) -> HypTestResult {
        welch_test(
            &self.moments_mean_f1(),
            &self.moments_mean_f2(),
            d0.into(),
            alt_hyp,
            alpha,
        ).expect("`number of recorded values <= 1` for either sample, `both standard deviations == 0`, or `alpha` not in open interval `(0, 1)`")
    }

    /// Confidence interval for the ratio of means `mean(latency(f1)) / mean(latency(f2))`,
    /// with confidence level `(1 - alpha)`, via the delta method.
    ///
    /// The interval is built on `ln(x̄1 / x̄2)` with variance
    /// `s1²/(n1·x̄1²) + s2²/(n2·x̄2²)` (Welch-style, unequal variances), then exponentiated. This is
    /// well-behaved for latencies, whose means are bounded away from zero. For a distribution-free
    /// alternative, see [`Self::ratio_means_boot_ci`].
    ///
    /// # Panics
    /// Panics if either sample has fewer than 2 recorded values or `alpha` is not in `(0, 1)`.
    pub fn ratio_means_ci(&self, alpha: f64) -> Ci {
        assert!(alpha > 0.0 && alpha < 1.0, "alpha must be in (0, 1)");
        let x1 = self.0.mean().as_f64();
        let x2 = self.1.mean().as_f64();
        let s1 = self.0.stdev_r().as_f64();
        let s2 = self.1.stdev_r().as_f64();
        let n1 = self.0.n_r() as f64;
        let n2 = self.1.n_r() as f64;
        let var_ln = s1 * s1 / (n1 * x1 * x1) + s2 * s2 / (n2 * x2 * x2);
        let se = var_ln.sqrt();
        let center = x1.ln() - x2.ln();
        let z = z_alpha(alpha / 2.0).expect("alpha / 2 is in (0, 1)");
        Ci((center - z * se).exp(), (center + z * se).exp())
    }

    /// Position of `value` with respect to the delta-method ratio-of-means confidence interval,
    /// with confidence level `(1 - alpha)`.
    pub fn welch_value_position_wrt_ratio_means_ci(&self, value: f64, alpha: f64) -> PositionWrtCi {
        self.ratio_means_ci(alpha).position_of(value)
    }

    /// Percentile bootstrap confidence interval for the ratio of means
    /// `mean(latency(f1)) / mean(latency(f2))`, with confidence level `(1 - alpha)`.
    ///
    /// Two-sample bootstrap: independently resamples the recorded observations in each output's
    /// bounded reservoir and recomputes the ratio of resample means. Distribution-free companion to
    /// [`Self::ratio_means_ci`].
    ///
    /// # Panics
    /// Panics if either reservoir is empty or `alpha` is not in `(0, 1)`.
    pub fn ratio_means_boot_ci(&self, alpha: f64) -> Ci {
        two_sample_ratio_means_ci(
            // Unlike the single-sample robust CIs ([`BenchOut::median_rob_ci`]/[`BenchOut::mean_rob_ci`]),
            // this is **not** memoized: it resamples only *naive arithmetic means* (no per-resample robust
            // estimator or histogram), so it is inexpensive (~10ms at the defaults) and simply recomputed
            // on each call — which also keeps [`Comp`] stateless and transient.
            self.0.reservoir(),
            self.1.reservoir(),
            DEFAULT_MEAN_CI_RESAMPLES,
            alpha,
            DEFAULT_BOOTSTRAP_SEED,
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

        RankSum::from_iters_with_counts(iter_f1, iter_f2).expect(
            // samples not in increasing order is impossible due to use of Histogram
            "either sample is empty",
        )
    }

    #[cfg(feature = "_experimental")]
    /// Wilcoxon rank sum *W* statistic for `latency(f1)` and `latency(f2)`.
    /// Gated by feature **"_experimental"**.
    ///
    /// Distribution-free companion to the mean suite: it assumes no particular latency distribution.
    /// Note that under batching the recorded observations are batch means, so this tests for a shift
    /// in the distribution of the *batch means* rather than of per-execution latencies.
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
        HI_STDEV_LN, LO_STDEV_LN, lognormal_moments, lognormal_moments_jittered, lognormal_out,
        lognormal_out_jittered,
    };
    use crate::{BenchCfg, LatencyUnit};
    use basic_stats::approx_eq;

    const EPSILON: f64 = 0.001;
    const JITTER_EPSILON: f64 = EPSILON;
    // `ratio_medians_f1_f2_r` is computed from a genuinely random sample (see
    // `normal_rand_samp`/`lognormal_rand_samp` in `basic_stats`), so it carries real sampling
    // noise around the theoretical ratio, unlike the exact-equality assertions in this test
    // (which compare the crate's internal computation against an independently-but-identically
    // seeded reference sample, and so remain exact regardless of the sampling method).
    const RATIO_MEDIANS_EPSILON: f64 = 0.005;
    const ALPHA: f64 = 0.05;

    fn are_eq_bench_out(out1: &BenchOut, out2: &BenchOut) -> bool {
        out1.recording_unit == out2.recording_unit
            && out1.summary() == out2.summary()
            && out1.sum == out2.sum
            && out1.sum2 == out2.sum2
            && out1.n_nz == out2.n_nz
            && out1.sum_ln == out2.sum_ln
            && out1.sum2_ln == out2.sum2_ln
    }

    #[test]
    fn test_comp_new_panics_on_recording_unit_mismatch() {
        let result = {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let cfg1 = BenchCfg::default().with_recording_unit(LatencyUnit::NANO);
                let out1 = lognormal_out(&cfg1, 8., *LO_STDEV_LN, 5);

                let cfg2 = cfg1.with_recording_unit(LatencyUnit::MICRO);
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

        let samp_size = 12_800;
        let n_jitter = 7;

        let sigma_lo = *LO_STDEV_LN;
        let sigma_hi = *HI_STDEV_LN;

        let mu_a = 8.;
        let out_a = lognormal_out(&cfg, mu_a, sigma_lo, samp_size);
        let moments_a = lognormal_moments(mu_a, sigma_lo, samp_size);
        let out_aj =
            lognormal_out_jittered(&cfg, mu_a, sigma_hi, samp_size, n_jitter, JITTER_EPSILON);
        let moments_aj =
            lognormal_moments_jittered(mu_a, sigma_hi, samp_size, n_jitter, JITTER_EPSILON);

        let median_ratio_a_b: f64 = 1.01;
        let mu_b = mu_a - median_ratio_a_b.ln();
        let out_bj =
            lognormal_out_jittered(&cfg, mu_b, sigma_hi, samp_size, n_jitter, JITTER_EPSILON);
        let moments_bj =
            lognormal_moments_jittered(mu_b, sigma_hi, samp_size, n_jitter, JITTER_EPSILON);

        struct TestArgs<'a> {
            ratio_medians: f64,
            d0: FpSeconds,
            o1: &'a BenchOut,
            mom1: &'a SampleMoments,
            o2: &'a BenchOut,
            mom2: &'a SampleMoments,
            alt_hyp: AltHyp,
        }

        // Independent reference implementation of the delta-method ratio-of-means CI.
        fn expected_ratio_means_ci(m1: &SampleMoments, m2: &SampleMoments, alpha: f64) -> Ci {
            let x1 = m1.mean().unwrap();
            let x2 = m2.mean().unwrap();
            let s1 = m1.stdev().unwrap();
            let s2 = m2.stdev().unwrap();
            let n1 = m1.n() as f64;
            let n2 = m2.n() as f64;
            let var_ln = s1 * s1 / (n1 * x1 * x1) + s2 * s2 / (n2 * x2 * x2);
            let se = var_ln.sqrt();
            let center = x1.ln() - x2.ln();
            let z = z_alpha(alpha / 2.0).unwrap();
            Ci((center - z * se).exp(), (center + z * se).exp())
        }

        let run_test = |args: TestArgs<'_>| {
            let TestArgs {
                ratio_medians,
                d0,
                o1,
                mom1,
                o2,
                mom2,
                alt_hyp,
            } = args;

            let comp = Comp::new(o1, o2);
            let f1_out = comp.out_f1();
            let f2_out = comp.out_f2();

            assert!(are_eq_bench_out(o1, f1_out));
            assert!(are_eq_bench_out(o2, f2_out));

            // Unchanged descriptive comparisons.
            assert_eq!(
                f1_out.median_r() - f2_out.median_r(),
                comp.diff_medians_f1_f2_r()
            );
            approx_eq!(
                ratio_medians,
                comp.ratio_medians_f1_f2_r(),
                RATIO_MEDIANS_EPSILON
            );
            assert_eq!(f1_out.mean() - f2_out.mean(), comp.mean_diff_f1_f2());
            assert_eq!(
                f1_out.mean().as_f64() / f2_out.mean().as_f64(),
                comp.ratio_means_f1_f2()
            );
            assert_eq!(
                f1_out.mean_ln_r() - f2_out.mean_ln_r(),
                comp.mean_diff_ln_f1_f2()
            );

            // Parametric mean suite vs. basic_stats on the natural-space moments.
            let d0f = d0.as_f64();
            assert_eq!(welch_t(mom1, mom2, d0f).unwrap(), comp.welch_mean_t(d0));
            assert_eq!(welch_df(mom1, mom2).unwrap(), comp.welch_mean_df());
            assert_eq!(
                welch_p(mom1, mom2, d0f, alt_hyp).unwrap(),
                comp.welch_mean_p(d0, alt_hyp)
            );
            let exp_ci = welch_ci(mom1, mom2, ALPHA).unwrap();
            let (lo, hi) = comp.welch_mean_diff_ci(ALPHA);
            assert_eq!(FpSeconds(exp_ci.0), lo);
            assert_eq!(FpSeconds(exp_ci.1), hi);

            // Delta-method ratio-of-means CI vs. the independent reference.
            assert_eq!(
                expected_ratio_means_ci(mom1, mom2, ALPHA),
                comp.ratio_means_ci(ALPHA)
            );

            // Test delegates to basic_stats' Welch test.
            assert_eq!(
                welch_test(mom1, mom2, d0f, alt_hyp, ALPHA)
                    .unwrap()
                    .accepted(),
                comp.welch_mean_test(d0, alt_hyp, ALPHA).accepted()
            );
        };

        // Scenario 1: hypothesized difference of means 0.
        run_test(TestArgs {
            ratio_medians: 1.0,
            d0: FpSeconds::ZERO,
            o1: &out_a,
            mom1: &moments_a,
            o2: &out_aj,
            mom2: &moments_aj,
            alt_hyp: AltHyp::Ne,
        });

        // Scenario 2: hypothesized difference 0, one-sided.
        run_test(TestArgs {
            ratio_medians: median_ratio_a_b,
            d0: FpSeconds::ZERO,
            o1: &out_a,
            mom1: &moments_a,
            o2: &out_bj,
            mom2: &moments_bj,
            alt_hyp: AltHyp::Gt,
        });

        // Scenario 3: hypothesized difference at the observed difference of means → t ≈ 0.
        run_test(TestArgs {
            ratio_medians: median_ratio_a_b,
            d0: out_a.mean() - out_bj.mean(),
            o1: &out_a,
            mom1: &moments_a,
            o2: &out_bj,
            mom2: &moments_bj,
            alt_hyp: AltHyp::Ne,
        });
    }

    #[test]
    fn test_comp_panics_on_empty_sample() {
        let cfg = BenchCfg::default();
        let mut out1 = BenchOut::new(&cfg, None);
        out1.record_from_iter(std::iter::empty::<FpSeconds>());
        let mut src2 = ConstLatencySrc::new([FpSeconds::from_millis(3)], 1);
        let mut out2 = BenchOut::new(&cfg, None);
        out2.record_from_iter(src2.aggregate().take(10));
        let comp = Comp::new(&out1, &out2);

        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || comp.welch_mean_t(FpSeconds::ZERO)
            ))
            .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| comp.welch_mean_df()))
                .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                comp.welch_mean_p(FpSeconds::ZERO, AltHyp::Ne)
            }))
            .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || comp.welch_mean_diff_ci(0.05)
            ))
            .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                comp.welch_mean_test(FpSeconds::ZERO, AltHyp::Ne, 0.05)
            }))
            .is_err()
        );
    }

    #[test]
    fn test_comp_panics_on_singleton_sample() {
        let cfg = BenchCfg::default();
        let mut src1 = ConstLatencySrc::new([FpSeconds::from_millis(3)], 1);
        let mut out1 = BenchOut::new(&cfg, None);
        out1.record_from_iter(src1.aggregate().take(10));
        let mut out2 = BenchOut::new(&cfg, None);
        out2.record_from_iter([FpSeconds::from_millis(1)].into_iter());
        let comp = Comp::new(&out1, &out2);

        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || comp.welch_mean_t(FpSeconds::ZERO)
            ))
            .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| comp.welch_mean_df()))
                .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                comp.welch_mean_p(FpSeconds::ZERO, AltHyp::Ne)
            }))
            .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || comp.welch_mean_diff_ci(0.05)
            ))
            .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                comp.welch_mean_test(FpSeconds::ZERO, AltHyp::Ne, 0.05)
            }))
            .is_err()
        );
    }

    #[test]
    fn test_comp_panics_on_both_stdev_zero() {
        let cfg = BenchCfg::default();
        let mut out1 = BenchOut::new(&cfg, None);
        out1.record_from_iter([FpSeconds::from_millis(5), FpSeconds::from_millis(5)].into_iter());
        let mut out2 = BenchOut::new(&cfg, None);
        out2.record_from_iter([FpSeconds::from_millis(5), FpSeconds::from_millis(5)].into_iter());
        let comp = Comp::new(&out1, &out2);

        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || comp.welch_mean_t(FpSeconds::ZERO)
            ))
            .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| comp.welch_mean_df()))
                .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                comp.welch_mean_p(FpSeconds::ZERO, AltHyp::Ne)
            }))
            .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || comp.welch_mean_diff_ci(0.05)
            ))
            .is_err()
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                comp.welch_mean_test(FpSeconds::ZERO, AltHyp::Ne, 0.05)
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
        let mut out1 = BenchOut::new(&cfg, None);
        out1.record_from_iter(std::iter::empty::<FpSeconds>());
        let mut src2 = ConstLatencySrc::new([FpSeconds::from_millis(3)], 1);
        let mut out2 = BenchOut::new(&cfg, None);
        out2.record_from_iter(src2.aggregate().take(10));
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
        let mu = 8.0;
        let sigma = *LO_STDEV_LN;
        let samp_size = 500;
        let latencies: Vec<FpSeconds> = lognormal_samp(mu, sigma, samp_size).collect();
        let mut out1 = BenchOut::new(&cfg, None);
        out1.record_from_iter(latencies.iter().cloned());
        let mut out2 = BenchOut::new(&cfg, None);
        out2.record_from_iter(latencies.iter().cloned());
        let comp = Comp::new(&out1, &out2);

        let p = comp.wilcoxon_rank_sum_p(AltHyp::Ne);
        assert!(p > 0.05, "expected p > 0.05, got {}", p);
    }
}
