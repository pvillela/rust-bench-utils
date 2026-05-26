use crate::latency;
use std::time::Duration;

pub struct LatencySrc1<F0: FnMut()>(pub F0);

impl<F0: FnMut()> Iterator for LatencySrc1<F0> {
    type Item = [Duration; 1];

    fn next(&mut self) -> Option<Self::Item> {
        Some([latency(&mut self.0)])
    }
}

pub struct LatencySrc2<F0: FnMut(), F1: FnMut()>(pub F0, pub F1);

impl<F0: FnMut(), F1: FnMut()> Iterator for LatencySrc2<F0, F1> {
    type Item = [Duration; 2];

    fn next(&mut self) -> Option<Self::Item> {
        Some([latency(|| self.0()), latency(|| self.1())])
    }
}

pub fn aggregate(arr: &[Duration; 2]) -> Duration {
    arr.iter().sum()
}
