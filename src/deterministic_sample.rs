/// Generates a deterministic sample of size `2*n*n - 1` for the
/// probability distribution given by the inverse CDF function `inv_cdf`.
///
/// The sample covers the output range evenly throughout the generation process.
pub fn deterministic_sample<'a>(
    inv_cdf: impl Fn(f64) -> f64 + 'a,
    n: u64,
) -> impl Iterator<Item = f64> + 'a {
    let unif_iter = deterministic_uniform_sample(n);
    unif_iter.map(move |unif_item| inv_cdf(unif_item))
}

/// Generates a deterministic sample of size `2*n*n - 1` for the
/// uniform probability distribution in open interval `(0, 1)`.
///
/// The sample covers the output range evenly throughout the generation process.
pub fn deterministic_uniform_sample(n: u64) -> impl Iterator<Item = f64> {
    UnifIter { n, i: 0 }
}

struct UnifIter {
    n: u64,
    i: u64,
}

impl Iterator for UnifIter {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= 2 * self.n * self.n - 1 {
            return None;
        }
        let item = uniform_observation(self.n, self.i);
        self.i += 1;
        Some(item)
    }
}

/// Generates the `i`-th observation for [`deterministic_uniform_sample`].
///
/// The sample covers the output range evenly throughout the generation process.
#[inline(always)]
fn uniform_observation(n: u64, i: u64) -> f64 {
    let side = i % 2;
    let j = i / 2;
    let bucket_idx = j % n;
    let item_idx = j / n;
    let left_idx = bucket_idx * n + item_idx + 1;
    let idx = if side == 0 {
        left_idx
    } else {
        2 * n * n - left_idx
    };
    idx as f64 / (2 * n * n) as f64
}

#[cfg(test)]
mod test {
    use super::{deterministic_sample, deterministic_uniform_sample};
    use statest::ks::KSTest;
    use statrs::distribution::{InverseCDF, Normal, Uniform};

    const EPSILON: f64 = 0.005;

    #[test]
    fn test_uniform() {
        let iter = deterministic_uniform_sample(10);
        let v: Vec<f64> = iter.collect();
        let dist = Uniform::new(0.0, 1.0).unwrap();
        let ks = KSTest::new(&v);
        let (p, _) = ks.ks1(&dist);
        assert!(1. - p < EPSILON)
    }

    #[test]
    fn test_normal() {
        let normal = Normal::new(0., 1.).unwrap();
        let iter = deterministic_sample(|x| normal.inverse_cdf(x), 10);
        let v: Vec<f64> = iter.collect();
        let ks = KSTest::new(&v);
        let (p, _) = ks.ks1(&normal);
        assert!(1. - p < EPSILON)
    }
}
