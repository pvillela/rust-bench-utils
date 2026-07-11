use crate::RunLength;

/// Returns the batch size of a batch specification.
pub fn bsz(batch: Option<usize>) -> usize {
    batch.unwrap_or(1).max(1)
}

/// Adjusts a [`RunLength`] for batching by up-dividing the count by batch size.
pub fn batched_run_length(run_length: RunLength, batch: Option<usize>) -> RunLength {
    let bsz = bsz(batch);
    match run_length {
        RunLength::Count(count) => RunLength::Count(count.div_ceil(bsz)),
        RunLength::Time(_) => run_length,
        RunLength::CountWithTimeout(count, time) => {
            RunLength::CountWithTimeout(count.div_ceil(bsz), time)
        }
    }
}

pub fn memoized_value<T: Clone>(opt: &mut Option<T>, f: impl FnOnce() -> T) -> T {
    match opt {
        Some(value) => value.clone(),
        None => {
            let value = f();
            *opt = Some(value.clone());
            value
        }
    }
}
