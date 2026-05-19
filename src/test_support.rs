use crate::{BenchCfg, BenchOut};
use basic_stats::{core::SampleMoments, normal::normal_detm_samp};
use std::sync::{LazyLock, Mutex};

pub static LO_STDEV_LN: LazyLock<f64> = LazyLock::new(|| 1.2_f64.ln() / 2.);
pub static HI_STDEV_LN: LazyLock<f64> = LazyLock::new(|| 2.4_f64.ln() / 2.);

static BENCH_CFG_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// To wrap test logic that modifies the global bench config to prevent race conditions and ensure restoration of the
/// pre-existing global config.
/// The test logic wrapped by this MUST NOT have code that can panic, otherwise the Mutex can be poisoned.
///
/// WARNING: USE OF THIS FUNCTION CAN CAUSE DEADLOCKS AS RUST MUTEXES ARE NOT REENTRANT.
pub fn with_safe_bench_cfg<T>(f: impl Fn() -> T) -> T {
    println!(">>> ENTERED with_safe_bench_cfg");
    let lock = BENCH_CFG_TEST_LOCK.lock().unwrap();
    let saved_cfg = BenchCfg::get();
    println!("saved_cfg={saved_cfg:?}");
    let res = f();
    saved_cfg.set();
    drop(lock);
    println!("<<< EXITING with_safe_bench_cfg");
    res
}

impl BenchOut {
    pub fn collect_data(&mut self, src: impl Iterator<Item = u64>) {
        for item in src {
            self.capture_data(item);
        }
    }

    pub fn print(&self) {
        println!(
            "BenchOut {{ recording_unit={:?}, reporting_unit={:?}, n={}, sum={}, sum2={}, n_ln={}, sum_ln={}, sum2_ln={}, summary={:?} }}",
            self.recording_unit,
            self.reporting_unit,
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
    let normal_samp = normal_detm_samp(mu, sigma, k).unwrap();
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
    let mut out = BenchOut::new(&BenchCfg::get());
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
