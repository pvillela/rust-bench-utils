use crate::{BenchCfg, BenchOut};
use basic_stats::{core::SampleMoments, normal::normal_detm_samp};
use std::{io::Write, sync::LazyLock};

pub static LO_STDEV_LN: LazyLock<f64> = LazyLock::new(|| 1.2_f64.ln() / 2.);
pub static HI_STDEV_LN: LazyLock<f64> = LazyLock::new(|| 2.4_f64.ln() / 2.);

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
    cfg: &BenchCfg,
    mu: f64,
    sigma: f64,
    k: u64,
    n_jitter: i64,
    jitter_epsilon: f64,
) -> BenchOut {
    let lognormal_samp = lognormal_samp_jittered(mu, sigma, k, n_jitter, jitter_epsilon)
        .map(|d| cfg.recording_unit().latency_from_u64(d));
    let out = BenchOut::from_iter(&cfg, lognormal_samp);
    out
}

pub fn lognormal_out(cfg: &BenchCfg, mu: f64, sigma: f64, k: u64) -> BenchOut {
    lognormal_out_jittered(cfg, mu, sigma, k, 3, 0.)
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

/// Writer backed by a [`Vec<u8>`] that can process backspace characters ("\u{8}") properly like stdeout and stderr do.
///
/// Used for testing of status reporting by this crate and `bench_diff`.
pub struct StringWriter {
    buf: Vec<u8>,
}

impl Write for StringWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for byte in buf {
            if *byte == 8 {
                // backspace character
                let res = self
                    .buf
                    .pop()
                    .ok_or_else(|| {
                        std::io::Error::other("backspace being writen into empty `StringWriter`")
                    })
                    .map(|b| b as usize);
                if res.is_err() {
                    return res;
                }
            } else {
                self.buf.push(*byte);
            }
        }

        // self.buf.write_all(buf).unwrap();
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl StringWriter {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.buf)
    }
}
