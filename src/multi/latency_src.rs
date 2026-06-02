use crate::latency;
use std::time::Duration;

/// Iterator that yields the latency of a single closure on each call to `next()`.
///
/// Each invocation returns a single-element array containing the wall-clock duration
/// of executing the wrapped closure.
pub struct LatencySrc1<F0: FnMut()>(pub F0);

impl<F0: FnMut()> Iterator for LatencySrc1<F0> {
    type Item = [Duration; 1];

    fn next(&mut self) -> Option<Self::Item> {
        Some([latency(&mut self.0)])
    }
}

/// Iterator that measures the latencies of two closures on each call to `next()`.
///
/// Each invocation yields a two-element array containing the wall-clock durations
/// of executing each wrapped closure.
pub struct LatencySrc2<F0: FnMut(), F1: FnMut()>(pub F0, pub F1);

impl<F0: FnMut(), F1: FnMut()> Iterator for LatencySrc2<F0, F1> {
    type Item = [Duration; 2];

    fn next(&mut self) -> Option<Self::Item> {
        Some([latency(|| self.0()), latency(|| self.1())])
    }
}
