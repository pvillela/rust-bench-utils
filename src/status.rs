use std::{io::Write, time::Duration};

pub trait Status<'a> {
    fn warmup_status<'b>(&'b mut self) -> Option<impl FnMut(Duration, usize, usize) + 'b>
    where
        'a: 'b;
    fn exec_status<'b>(&'b mut self) -> Option<impl FnMut(Duration, usize, usize) + 'b>
    where
        'a: 'b;

    fn curry(
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

pub struct DefaultStatus<'a, W: Write> {
    pub w: &'a mut W,
    pub warmup_preamble: String,
    pub exec_preamble: String,
}

impl<'a, W: Write> DefaultStatus<'a, W> {
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
