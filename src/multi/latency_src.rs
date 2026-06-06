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
        /// The argument is an array of pairs of medians and sigmas that determine the probability distributions of the
        /// `K` component iterators:
        /// - each median is the median latency for the component;
        /// - each sigma is the standard deviation for the natural logarithm of the component's probability distribution.
        pub fn new(medians_sigmas: [(Duration, f64); K]) -> Self {
            assert!(
                medians_sigmas
                    .iter()
                    .all(|(m, s)| *m > Duration::ZERO && *s > 0.0),
                "all medians and sigmas must be positive"
            );

            let generators = medians_sigmas.map(|(mu, sigma)| {
                let gen_f64 = lognormal_detm_gen(mu.as_secs_f64(), sigma)
                    .expect("mus must be finite and sigmas must be positve");
                let gen_dur = gen_f64.map(|v| Duration::from_secs_f64(v));
                let boxed_gen: Box<dyn Iterator<Item = Duration>> = Box::new(gen_dur);
                boxed_gen
            });

            Self { generators }
        }

        /// Instantiates `Self` with default sigmas for the underlying lognormal distributions.
        ///
        /// The argument is an array of the medians of the probability distributions of the `K` component iterators.
        /// The distributions' sigmas are set to a default value.
        pub fn new_with_default_sigmas(medians: [Duration; K]) -> Self {
            // At 2 standard deviations of the underlying normal distribution, the latency is 1.15 times the median latency.
            let default_sigma = 1.15_f64.ln() / 2.;

            assert!(
                medians.iter().all(|m| *m > Duration::ZERO),
                "all medians must be positive"
            );

            let medians_sigmas = medians.map(|m| (m, default_sigma));
            Self::new(medians_sigmas)
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
}
