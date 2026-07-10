//! Module defining the key data structure produced by the [`multi::bench_run`](super::bench_run) and
//! related benchmarking functions.

use crate::{BenchCfg, FpSeconds, LatencyUnit};
use std::{
    array,
    fmt::Debug,
    ops::{Deref, Index},
};

/// Contains the data resulting from benchmarking a group of closures.
///
/// This struct holds an array of [`crate::BenchOut`] objects which is returned
/// by [`multi::bench_run`](super::bench_run) and related benchmarking functions
///
/// Its methods provide descriptive and inferential statistics about the latency samples of the
/// benchmarked closures.
///
/// The `*_ln_*` methods provide statistics for `mean(ln(latency(f)))`, where `ln` is the natural logarithm.
/// Under the assumption that `latency(f)` is approximately log-normal, `mean(ln(latency(f))) == ln(median(latency(f)))`.
/// This assumption is widely supported by performance analysis theory and empirical data.
/// Thus, the `*_ln_*` methods are useful for the analysis of median latencies.
#[derive(Debug)]
pub struct BenchOut<const K: usize> {
    pub(crate) arr: [crate::BenchOut; K],
}

impl<const K: usize> Index<usize> for BenchOut<K> {
    type Output = crate::BenchOut;

    fn index(&self, index: usize) -> &Self::Output {
        &self.arr[index]
    }
}

//--- Impls for BenchOut<1>

impl Deref for BenchOut<1> {
    type Target = crate::BenchOut;

    fn deref(&self) -> &Self::Target {
        &self.arr[0]
    }
}

impl From<crate::BenchOut> for BenchOut<1> {
    fn from(value: crate::BenchOut) -> Self {
        Self { arr: [value] }
    }
}

impl BenchOut<1> {
    /// Unwraps the single-element array and returns the contained [`BenchOut`](crate::BenchOut).
    pub fn flatten(self) -> crate::BenchOut {
        self.into()
    }
}

//--- Impls for BenchOut<2>
// See module `duo`.

//--- Impls for BenchOut<K>

impl<const K: usize> BenchOut<K> {
    #[doc(hidden)]
    pub fn new(cfg: &BenchCfg, batch: Option<usize>) -> Self {
        Self {
            arr: array::from_fn(|_| crate::BenchOut::new(cfg, batch)),
        }
    }

    /// Updates `self` from a **finite** iterator of `[FpSeconds; K]` arrays.
    ///
    /// Each item from the iterator is recorded as a latency value for each of the array components.
    ///
    /// # May hang
    /// Hangs if the iterator is not finite.
    pub fn record_from_iter(&mut self, src: impl Iterator<Item = [FpSeconds; K]>) {
        self.record_from_iter_with_counts(src.map(|x| (x, 1)));
    }

    /// Updates `self` from a **finite** iterator of `([FpSeconds; K], usize)`` pairs.
    ///
    /// Each item from the iterator is recorded as `count` latency values for each of the array components,
    /// where `count` is the second component of the pair.
    ///
    /// ## May hang
    /// Hangs if the iterator is not finite.
    pub fn record_from_iter_with_counts(
        &mut self,
        src: impl Iterator<Item = ([FpSeconds; K], usize)>,
    ) {
        for (ltncy_arr, count) in src {
            for (b, fps) in self.arr.iter_mut().zip(ltncy_arr.iter()) {
                b.capture_data_with_counts((*fps, count));
            }
        }
    }

    /// Resets `self` to an empty instance.
    pub(crate) fn reset(&mut self) {
        for b in &mut self.arr {
            b.reset();
        }
    }

    #[inline(always)]
    /// Updates `self` with an elapsed time observation for the functions.
    pub(crate) fn capture_data(&mut self, batch_avgs: [FpSeconds; K]) {
        for (i, out) in &mut self.arr.iter_mut().enumerate() {
            out.capture_data(batch_avgs[i]);
        }
    }

    /// Returns the number of benchmarked closures (`K`).
    #[inline(always)]
    pub fn arity(&self) -> usize {
        K
    }

    #[inline(always)]
    fn first(&self) -> &crate::BenchOut {
        &self.arr[0]
    }

    /// Latency unit used in data collection.
    pub fn recording_unit(&self) -> LatencyUnit {
        self.first().recording_unit()
    }

    /// Batching used in data collection.
    ///
    /// - `None` means no batching;
    /// - `Some(b)` means batches of size `b`.
    #[inline(always)]
    pub fn batch(&self) -> Option<usize> {
        self.first().batch()
    }

    /// Batch size used in data collection. Returns `1` for `batch` values of `None`, `Some(0)`, and `Some(1)`.
    #[inline(always)]
    pub fn bsz(&self) -> usize {
        self.first().bsz()
    }

    /// Number of recorded values. In case of batching, each group (batch) contributes one recorded value.
    #[inline(always)]
    pub fn n_r(&self) -> u64 {
        self.first().n_r()
    }

    /// Total number of function executions accounting for batching (`= self.groups() * self.bsz()`).
    #[inline(always)]
    pub fn n(&self) -> u64 {
        self.first().n()
    }

    /// Returns an iterator that yields `self`'s components
    pub fn iter(&self) -> impl Iterator<Item = &crate::BenchOut> {
        self.arr.iter()
    }

    /// Applies `f` to each component of `self` and collects the results in an array.
    pub fn map<T>(&self, mut f: impl FnMut(&crate::BenchOut) -> T) -> [T; K] {
        array::from_fn(|k| f(&self[k]))
    }
}

#[cfg(test)]
#[cfg(feature = "_test")]
mod test {
    use super::*;
    use crate::rel_approx_eq_fpsecs;
    use crate::{
        BenchCfg,
        test_support::{LO_STDEV_LN, lognormal_samp},
    };
    use basic_stats::{
        approx_eq,
        core::{AcceptedHyp, PositionWrtCi, SampleMoments},
        normal::{
            normal_detm_samp, student_1samp_ci, student_1samp_df, student_1samp_p, student_1samp_t,
        },
        rel_approx_eq,
    };
    use statrs::distribution::{ContinuousCDF, Normal};
    use std::panic::catch_unwind;

    const ALPHA: f64 = 0.05;

    fn lognormal_samp2(
        rec_mu: f64,
        sigma: f64,
        samp_size: usize,
    ) -> impl Iterator<Item = [FpSeconds; 2]> {
        lognormal_samp(rec_mu, sigma, samp_size).map(|x| [x, x])
    }

    #[test]
    fn test_deref() {
        let cfg = &BenchCfg::default().with_recording_unit(LatencyUnit::NANO);
        let mut out1 = BenchOut::<1>::new(&cfg, None);
        out1.record_from_iter(
            [[FpSeconds::from_millis(5)], [FpSeconds::from_millis(7)]].into_iter(),
        );

        assert_eq!(out1.mean(), FpSeconds::from_millis(6));
    }

    #[test]
    fn test_bench_out_2_comp() {
        let mu = 8.0;
        let sigma = *LO_STDEV_LN;
        let samp_size = 200;

        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(lognormal_samp2(mu, sigma, samp_size));

        let comp = out.comp();
        // Both outputs are fed the same data (`[y, y]`), so medians are equal
        assert_eq!(comp.out_f1().median_r(), comp.out_f2().median_r());
        // Verify both outputs have the expected sample size: 2*k*k - 1
        assert_eq!(comp.out_f1().n_r() as usize, samp_size);
        assert_eq!(comp.out_f2().n_r() as usize, samp_size);
    }

    #[test]
    fn test_bench_out_1_flatten() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::<1>::new(&cfg, None);
        out.record_from_iter(
            [[FpSeconds::from_millis(5)], [FpSeconds::from_millis(7)]].into_iter(),
        );

        let flat: crate::BenchOut = out.flatten();
        assert_eq!(flat.n_r(), 2);
    }

    #[test]
    fn test_bench_out_reset() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::<2>::new(&cfg, None);
        out.record_from_iter(
            [
                [FpSeconds::from_millis(1), FpSeconds::from_millis(2)],
                [FpSeconds::from_millis(3), FpSeconds::from_millis(4)],
            ]
            .into_iter(),
        );
        assert!(out.n() > 0);
        out.reset();
        assert_eq!(out.n(), 0);
    }

    #[test]
    fn test_bench_out_2_panics_on_empty() {
        let cfg = BenchCfg::default();
        let mut out = BenchOut::new(&cfg, None);
        out.record_from_iter(std::iter::empty::<[FpSeconds; 2]>());

        assert_eq!(out.n(), 0);
        assert_eq!(out[0].n_r(), 0);

        assert!(catch_unwind(std::panic::AssertUnwindSafe(|| out[0].mean())).is_err());
        assert!(catch_unwind(std::panic::AssertUnwindSafe(|| out[0].stdev())).is_err());
        assert!(catch_unwind(std::panic::AssertUnwindSafe(|| out[0].median_r())).is_err());
        assert!(catch_unwind(std::panic::AssertUnwindSafe(|| out[0].mean_ln_r())).is_err());
        assert!(catch_unwind(std::panic::AssertUnwindSafe(|| out[0].stdev_ln_r())).is_err());
    }
}
