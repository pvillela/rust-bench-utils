//! Trait and types that support default and custom progress status reporting for benchmarks.

use std::{io::Write, time::Duration};

/// Provides optional status reporting closures for warm-up and execution phases.
///
/// Implementors return either `Some(closure)` to report progress or `None` to skip reporting.
/// The returned closures receive the estimated duration, the estimated total execution count,
/// and the current iteration index.
pub trait Status<'a> {
    /// Returns an optional status closure for the warm-up phase.
    ///
    /// The closure receives `(est_dur, est_count, i)` where:
    /// - `est_dur` is the estimated warm-up duration.
    /// - `est_count` is the estimated number of warm-up iterations.
    /// - `i` is the current warm-up iteration.
    fn warmup_status<'b>(&'b mut self) -> Option<impl FnMut(Duration, usize, usize) + 'b>
    where
        'a: 'b;

    /// Returns an optional status closure for the execution phase.
    ///
    /// The closure receives `(est_dur, est_count, i)` where:
    /// - `est_dur` is the estimated execution duration.
    /// - `est_count` is the estimated number of execution iterations.
    /// - `i` is the current execution iteration.
    fn exec_status<'b>(&'b mut self) -> Option<impl FnMut(Duration, usize, usize) + 'b>
    where
        'a: 'b;

    /// Partially applies `(est_dur, est_count, i)` to a status closure,
    /// yielding an `FnMut(usize)` closure
    ///
    /// The returned closure only receives the iteration index `i`; the estimated duration
    /// and count are captured at construction time.
    fn part_apply(
        status: Option<impl FnMut(Duration, usize, usize)>,
        est_dur: Duration,
        est_count: usize,
    ) -> Option<impl FnMut(usize)> {
        status.map(|mut s| move |i| s(est_dur, est_count, i))
    }
}

pub(crate) struct NoStatus;

impl<'a> Status<'a> for NoStatus {
    fn warmup_status<'b>(&'b mut self) -> Option<impl FnMut(Duration, usize, usize) + 'b>
    where
        'a: 'b,
    {
        None::<fn(Duration, usize, usize)>
    }

    fn exec_status<'b>(&'b mut self) -> Option<impl FnMut(Duration, usize, usize) + 'b>
    where
        'a: 'b,
    {
        None::<fn(Duration, usize, usize)>
    }
}

/// Default implementation of [`Status`] that writes status messages to a [`Write`] target.
///
/// Warm-up and execution progress are reported as inline status lines with backspace
/// characters ("\\u{8}") so that the cursor position is updated in-place on terminals
/// and stderr-like writers.
pub struct DefaultStatus<'a, W: Write> {
    /// Writer to which status output is sent.
    pub w: &'a mut W,
    /// Preamble string printed before warm-up progress.
    pub warmup_preamble: String,
    /// Preamble string printed before execution progress.
    pub exec_preamble: String,
}

impl<'a, W: Write> DefaultStatus<'a, W> {
    /// Creates a new `DefaultStatus` that writes status messages to `w`.
    ///
    /// # Arguments
    ///
    /// - `w` - the writer (typically stderr or a `StringWriter` from `test_support` for testing).
    /// - `warmup_preamble` - text printed before warm-up progress (e.g. `"Warming up"`).
    /// - `exec_preamble` - text printed before execution progress (e.g. `"Executing bench_run"`).
    pub fn new(w: &'a mut W, warmup_preamble: String, exec_preamble: String) -> Self {
        Self {
            w,
            warmup_preamble,
            exec_preamble,
        }
    }

    fn make_status<'b>(w: &'b mut W, preamble: String) -> impl FnMut(Duration, usize, usize) + 'b
    where
        'a: 'b,
    {
        let mut status_len: usize = 0;

        move |est_dur: Duration, est_count: usize, i: usize| {
            if status_len == 0 {
                write!(
                    w,
                    "{} for (approx.) {} millis: ",
                    preamble.clone(),
                    est_dur.as_millis()
                )
                .expect("unexpected error writing to `Write` object `w`");
                w.flush().expect("unexpected I/O error");
            }
            write!(w, "{}", "\u{8}".repeat(status_len))
                .expect("unexpected error writing to `Write` object `w`");
            let status = format!("{} of (approx.) {} executions.", i, est_count);
            status_len = status.len();
            write!(w, "{status}").expect("unexpected error writing to `Write` object `w`");
            w.flush().expect("unexpected I/O error");
        }
    }
}

impl<'a, W: Write> Status<'a> for DefaultStatus<'a, W> {
    fn warmup_status<'b>(&'b mut self) -> Option<impl FnMut(Duration, usize, usize) + 'b>
    where
        'a: 'b,
    {
        Some(Self::make_status::<'b>(
            self.w,
            self.warmup_preamble.clone(),
        ))
    }

    fn exec_status<'b>(&'b mut self) -> Option<impl FnMut(Duration, usize, usize) + 'b>
    where
        'a: 'b,
    {
        Some(Self::make_status::<'b>(self.w, self.exec_preamble.clone()))
    }
}
