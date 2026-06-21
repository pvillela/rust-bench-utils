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
    /// The closure receives `(est_time, est_count, i)` where:
    /// - `est_time` is the estimated warm-up duration.
    /// - `est_count` is the estimated number of warm-up iterations.
    /// - `i` is the current warm-up iteration.
    fn warmup_status<'b>(&'b mut self) -> Option<impl FnMut(Duration, usize, usize) + 'b>
    where
        'a: 'b;

    /// Returns an optional closure to end status reporting for the warm-up phase.
    ///
    /// The closure receives `(est_time, est_count, i)` where:
    /// - `est_time` is the estimated warm-up duration.
    /// - `est_count` is the estimated number of warm-up iterations.
    /// - `i` is the current warm-up iteration.
    fn end_warmup_status<'b>(&'b mut self) -> Option<impl FnOnce() + 'b>
    where
        'a: 'b;

    /// Returns an optional status closure for the execution phase.
    ///
    /// The closure receives `(est_time, est_count, i)` where:
    /// - `est_time` is the estimated execution duration.
    /// - `est_count` is the estimated number of execution iterations.
    /// - `i` is the current execution iteration.
    fn exec_status<'b>(&'b mut self) -> Option<impl FnMut(Duration, usize, usize) + 'b>
    where
        'a: 'b;

    /// Returns an optional closure to end status reporting for the execution phase.
    ///
    /// The closure receives `(est_time, est_count, i)` where:
    /// - `est_time` is the estimated warm-up duration.
    /// - `est_count` is the estimated number of warm-up iterations.
    /// - `i` is the current warm-up iteration.
    fn end_exec_status<'b>(&'b mut self) -> Option<impl FnOnce() + 'b>
    where
        'a: 'b;

    /// Partially applies `(est_time, est_count, i)` to a status closure,
    /// yielding an `FnMut(usize)` closure
    ///
    /// The returned closure only receives the iteration index `i`; the estimated duration
    /// and count are captured at construction time.
    fn part_apply(
        status: Option<impl FnMut(Duration, usize, usize)>,
        est_time: Duration,
        est_count: usize,
    ) -> Option<impl FnMut(usize)> {
        status.map(|mut s| move |i| s(est_time, est_count, i))
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

    fn end_warmup_status<'b>(&'b mut self) -> Option<impl FnOnce() + 'b>
    where
        'a: 'b,
    {
        None::<fn()>
    }

    fn exec_status<'b>(&'b mut self) -> Option<impl FnMut(Duration, usize, usize) + 'b>
    where
        'a: 'b,
    {
        None::<fn(Duration, usize, usize)>
    }

    fn end_exec_status<'b>(&'b mut self) -> Option<impl FnOnce() + 'b>
    where
        'a: 'b,
    {
        None::<fn()>
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

        move |est_time: Duration, est_count: usize, i: usize| {
            if status_len == 0 {
                write!(
                    w,
                    "{} for (approx.) {} millis: ",
                    preamble.clone(),
                    est_time.as_millis()
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

    fn make_end_status<'b>(w: &'b mut W) -> impl FnMut() + 'b
    where
        'a: 'b,
    {
        || {
            write!(w, "\n").expect("unexpected error writing to `Write` object `w`");
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

    fn end_warmup_status<'b>(&'b mut self) -> Option<impl FnOnce() + 'b>
    where
        'a: 'b,
    {
        Some(Self::make_end_status(self.w))
    }

    fn exec_status<'b>(&'b mut self) -> Option<impl FnMut(Duration, usize, usize) + 'b>
    where
        'a: 'b,
    {
        Some(Self::make_status::<'b>(self.w, self.exec_preamble.clone()))
    }

    fn end_exec_status<'b>(&'b mut self) -> Option<impl FnOnce() + 'b>
    where
        'a: 'b,
    {
        Some(Self::make_end_status(self.w))
    }
}

#[cfg(test)]
#[cfg(feature = "_test")]
mod test {
    use super::*;
    use crate::test_support::StringWriter;
    use std::time::Duration;

    #[test]
    fn test_no_status_returns_none() {
        let mut ns = NoStatus;
        assert!(ns.warmup_status().is_none());
        assert!(ns.exec_status().is_none());
    }

    #[test]
    fn test_part_apply() {
        // None case
        assert!(
            NoStatus::part_apply(
                None::<fn(Duration, usize, usize)>,
                Duration::from_secs(1),
                100,
            )
            .is_none()
        );

        // Some case: verify captured values are forwarded correctly
        let mut captured = (Duration::ZERO, 0, 0);
        let result = NoStatus::part_apply(
            Some(|t, c, i| captured = (t, c, i)),
            Duration::from_secs(1),
            100,
        );
        assert!(result.is_some());
        let mut closure = result.unwrap();
        closure(42);
        drop(closure);
        assert_eq!(captured, (Duration::from_secs(1), 100, 42));
    }

    #[test]
    fn test_default_status_new() {
        let mut w = StringWriter::new();
        let ds = DefaultStatus::new(&mut w, "Warm".to_owned(), "Exec".to_owned());
        assert_eq!(ds.warmup_preamble, "Warm");
        assert_eq!(ds.exec_preamble, "Exec");
    }

    #[test]
    fn test_default_status_returns_some() {
        let mut w = StringWriter::new();
        let mut ds = DefaultStatus::new(&mut w, "Warm".to_owned(), "Exec".to_owned());
        assert!(ds.warmup_status().is_some());
        assert!(ds.end_warmup_status().is_some());
        assert!(ds.exec_status().is_some());
        assert!(ds.end_exec_status().is_some());
    }

    #[test]
    fn test_default_status_output() {
        // Test warmup status
        {
            let mut w = StringWriter::new();
            {
                let mut ds = DefaultStatus::new(&mut w, "Warm".to_owned(), "Exec".to_owned());
                let mut warmup_fn = ds.warmup_status().unwrap();
                warmup_fn(Duration::from_millis(500), 1000, 600);
            }
            let output = w.as_str().unwrap();
            assert!(
                output.contains("Warm"),
                "output should contain 'Warm': {output}"
            );
            assert!(
                output.contains("500 millis"),
                "output should contain '500 millis': {output}"
            );
            assert!(
                output.contains("600 of"),
                "output should contain '600 of': {output}"
            );
            assert!(
                output.contains("1000 executions"),
                "output should contain '1000 executions': {output}"
            );
        }

        // Test exec_status
        {
            let mut w = StringWriter::new();
            {
                let mut ds = DefaultStatus::new(&mut w, "Warm".to_owned(), "Exec".to_owned());
                let mut exec_fn = ds.exec_status().unwrap();
                exec_fn(Duration::from_millis(5000), 10000, 6000);
            }
            let output = w.as_str().unwrap();
            assert!(
                output.contains("Exec"),
                "output should contain 'Exec': {output}"
            );
            assert!(
                output.contains("5000 millis"),
                "output should contain '5000 millis': {output}"
            );
            assert!(
                output.contains("6000 of"),
                "output should contain '6000 of': {output}"
            );
            assert!(
                output.contains("10000 executions"),
                "output should contain '10000 executions': {output}"
            );
        }
    }
}
