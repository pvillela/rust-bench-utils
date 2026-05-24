use crate::BenchCfg;
use std::{io::Write, time::Duration};

pub struct LatSrc;

impl LatSrc {
    pub fn general<const K: usize>(
        cfg: &BenchCfg,
        fs: &mut [impl FnMut(); K],
        warmup_status: Option<impl FnMut(usize, usize, Duration, usize)>,
        exec_status: Option<impl FnMut(usize, usize, Duration, usize)>,
    ) -> impl Iterator<Item = [Duration; K]> {
    }
}
