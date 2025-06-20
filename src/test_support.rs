use crate::{BenchOut, LatencyUnit};
use basic_stats::{core::SampleMoments, normal::deterministic_normal_sample};
use std::sync::LazyLock;

pub static LO_STDEV_LN: LazyLock<f64> = LazyLock::new(|| 1.2_f64.ln() / 2.);
pub static HI_STDEV_LN: LazyLock<f64> = LazyLock::new(|| 2.4_f64.ln() / 2.);

impl BenchOut {
    pub fn collect_data(&mut self, src: impl Iterator<Item = u64>) {
        for item in src {
            self.capture_data(item);
        }
    }

    pub fn print(&self) {
        println!(
            "BenchOut {{ unit={:?}, n={}, sum={}, sum2={}, n_ln={}, sum_ln={}, sum2_ln={}, summary={:?} }}",
            self.unit,
            self.n(),
            self.sum,
            self.sum2,
            self.n_ln,
            self.sum_ln,
            self.sum2_ln,
            self.summary()
        );
    }
}

fn jitter(v: f64, i: i64, n_jitter: i64, epsilon: f64) -> f64 {
    assert!(n_jitter >= 3, "n_jitter must be >= 3");
    let max_jitter = (n_jitter - 1) / 2;
    let delta = (i % n_jitter - max_jitter) as f64 / max_jitter as f64 * epsilon;
    v + delta
}

pub fn lognormal_samp_jittered(
    mu: f64,
    sigma: f64,
    k: u64,
    n_jitter: i64,
    jitter_epsilon: f64,
) -> impl Iterator<Item = u64> {
    let normal_samp = deterministic_normal_sample(mu, sigma, k).unwrap();
    normal_samp
        .enumerate()
        .map(move |(i, v)| jitter(v, i as i64, n_jitter, jitter_epsilon))
        .map(|x| x.exp() as u64)
}

pub fn lognormal_samp(mu: f64, sigma: f64, k: u64) -> impl Iterator<Item = u64> {
    lognormal_samp_jittered(mu, sigma, k, 3, 0.)
}

pub fn lognormal_out_jittered(
    mu: f64,
    sigma: f64,
    k: u64,
    n_jitter: i64,
    jitter_epsilon: f64,
) -> BenchOut {
    let lognormal_samp = lognormal_samp_jittered(mu, sigma, k, n_jitter, jitter_epsilon);
    let mut out = BenchOut::new(LatencyUnit::Micro);
    out.collect_data(lognormal_samp);
    out
}

pub fn lognormal_out(mu: f64, sigma: f64, k: u64) -> BenchOut {
    lognormal_out_jittered(mu, sigma, k, 3, 0.)
}

pub fn lognormal_moments_ln_jittered(
    mu: f64,
    sigma: f64,
    k: u64,
    n_jitter: i64,
    jitter_epsilon: f64,
) -> SampleMoments {
    let dataset = lognormal_samp_jittered(mu, sigma, k, n_jitter, jitter_epsilon)
        .map(|v| (v.max(1) as f64).ln());
    SampleMoments::from_iterator(dataset)
}

pub fn lognormal_moments_ln(mu: f64, sigma: f64, k: u64) -> SampleMoments {
    lognormal_moments_ln_jittered(mu, sigma, k, 3, 0.)
}
