use crate::RunLength;

/// Adjusts a [`RunLength`] for batching by up-dividing the count by batch size.
pub fn batched_run_length(run_length: RunLength, batch: Option<usize>) -> RunLength {
    let bsz = batch.unwrap_or(1).max(1);
    match run_length {
        RunLength::Count(count) => RunLength::Count(count.div_ceil(bsz)),
        RunLength::Time(_) => run_length,
        RunLength::CountWithTimeout(count, time) => {
            RunLength::CountWithTimeout(count.div_ceil(bsz), time)
        }
    }
}
