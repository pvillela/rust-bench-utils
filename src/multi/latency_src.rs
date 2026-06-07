use crate::latency;
use std::time::Duration;

/// An infinite iterator that encapsulates `K` closures and, for each invocation
/// of `next()`, yields an array of size `K` with the wall-clock latency durations from one execution
/// of each of the `K` closures.
///
/// The iterator doesn't have to be infinite, though normally it would be. If a finite iterator is used
/// with the benchmarking functions, the benchmark will complete if the iterator is exhausted before
/// the configured benchmark run length.
pub trait LatencySrc<const K: usize>: Iterator<Item = [Duration; K]> {}

impl<const K: usize, T: LatencySrc<K>> LatencySrc<K> for &mut T {}

/// Infinite iterator that yields the latency of a single closure on each call to `next()`.
///
/// Each invocation returns a single-element array containing the wall-clock duration
/// of executing the wrapped closure.
pub struct LatencySrc1<F0: FnMut()>(pub F0);

impl<F0: FnMut()> Iterator for LatencySrc1<F0> {
    type Item = [Duration; 1];

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        Some([latency(&mut self.0)])
    }
}

impl<F0: FnMut()> LatencySrc<1> for LatencySrc1<F0> {}

/// Infinite iterator that measures the latencies of two closures on each call to `next()`.
///
/// Each invocation yields a two-element array containing the wall-clock durations
/// of executing each wrapped closure.
pub struct LatencySrc2<F0: FnMut(), F1: FnMut()>(pub F0, pub F1);

impl<F0: FnMut(), F1: FnMut()> Iterator for LatencySrc2<F0, F1> {
    type Item = [Duration; 2];

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        Some([latency(|| self.0()), latency(|| self.1())])
    }
}

impl<F0: FnMut(), F1: FnMut()> LatencySrc<2> for LatencySrc2<F0, F1> {}

#[cfg(test)]
#[cfg(feature = "_test")]
pub mod test_support {
    use super::*;
    use basic_stats::normal::lognormal_detm_gen;
    use std::array;

    pub struct EmptyLatencySrc<const K: usize>;

    impl<const K: usize> Iterator for EmptyLatencySrc<K> {
        type Item = [Duration; K];

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    impl<const K: usize> LatencySrc<K> for EmptyLatencySrc<K> {}

    pub struct ConstLatencySrc<const K: usize> {
        latencies: [Duration; K],
    }

    impl<const K: usize> ConstLatencySrc<K> {
        pub fn new(latencies: [Duration; K]) -> Self {
            Self { latencies }
        }
    }

    impl<const K: usize> Iterator for ConstLatencySrc<K> {
        type Item = [Duration; K];

        fn next(&mut self) -> Option<Self::Item> {
            Some(self.latencies)
        }
    }

    impl<const K: usize> LatencySrc<K> for ConstLatencySrc<K> {}

    /// An infinite iterator that encapsulates `K` iterators, each of which emits [`Duration`] values according to
    /// a lognormal distribution.
    ///
    /// This iterator's `next()` method yields a `[Duration; K]`, each component of which draws from the corresonding
    /// encapsulated iterator.
    pub struct LognormalLatencySrc<const K: usize> {
        generators: [Box<dyn Iterator<Item = Duration>>; K],
    }

    impl<const K: usize> LognormalLatencySrc<K> {
        /// Instantiates `Self`.
        ///
        /// The argument is an array of pairs of targets and sigmas that determine the probability distributions of the
        /// `K` component iterators:
        /// - each target is the target median latency for the component;
        /// - each sigma is the standard deviation for the natural logarithm of the component's probability distribution.
        pub fn new(targets_sigmas: [(Duration, f64); K]) -> Self {
            assert!(
                targets_sigmas
                    .iter()
                    .all(|(m, s)| *m > Duration::ZERO && *s > 0.0),
                "all medians and sigmas must be positive"
            );

            let generators = targets_sigmas.map(|(target, sigma)| {
                let gen_f64 = lognormal_detm_gen(target.as_secs_f64().ln(), sigma)
                    .expect("`target medians` and `sigmas` must all be `> 0`");
                let gen_dur = gen_f64.map(|v| Duration::from_secs_f64(v));
                let boxed_gen: Box<dyn Iterator<Item = Duration>> = Box::new(gen_dur);
                boxed_gen
            });

            Self { generators }
        }

        /// Instantiates `Self` with default sigmas for the underlying lognormal distributions.
        ///
        /// The argument is an array of the target medians of the `K` component iterators.
        /// The distributions' sigmas are set to a default value.
        pub fn new_with_default_sigmas(targets: [Duration; K]) -> Self {
            // At 2 standard deviations of the underlying normal distribution, the latency is `multiplier` times the median latency.
            let multiplier: f64 = 1.10;
            let default_sigma = multiplier.ln() / 2.;

            assert!(
                targets.iter().all(|m| *m > Duration::ZERO),
                "all `target medians` must be positive"
            );

            let targets_sigmas = targets.map(|m| (m, default_sigma));
            Self::new(targets_sigmas)
        }
    }

    impl<const K: usize> Iterator for LognormalLatencySrc<K> {
        type Item = [Duration; K];

        fn next(&mut self) -> Option<Self::Item> {
            let durations = array::from_fn(|i| {
                self.generators[i]
                    .next()
                    .expect("`lognormal_detm_gen(,,).next()` can't return `None`")
            });
            Some(durations)
        }
    }

    impl<const K: usize> LatencySrc<K> for LognormalLatencySrc<K> {}

    mod test {
        use super::*;
        use crate::{BenchCfg, multi::BenchOut, rel_approx_eq_dur};

        const EPSILON: f64 = 0.002;
        const SAMP_SIZE: usize = 100;

        #[test]
        // cargo test --package bench_utils --lib --all-features -- multi::latency_src::test_support::test::test_lognormal_src --exact --nocapture --include-ignored
        fn test_lognormal_src() {
            let cfg = BenchCfg::default();
            let targets = [
                Duration::from_nanos(10),
                Duration::from_micros(20),
                Duration::from_millis(30),
            ];
            let src = LognormalLatencySrc::new_with_default_sigmas(targets);
            let out = BenchOut::from_iter(&cfg, src.take(SAMP_SIZE));
            let out_medians = out.medians();

            println!(
                "*** src[0]={:?}",
                LognormalLatencySrc::new_with_default_sigmas(targets)
                    .take(20)
                    .collect::<Vec<_>>()
            );

            rel_approx_eq_dur!(targets[0], out_medians[0], EPSILON);
            rel_approx_eq_dur!(targets[1], out_medians[1], EPSILON);
            rel_approx_eq_dur!(targets[2], out_medians[2], EPSILON);
        }
    }
}
