mod bench_out;
mod bench_run;
// mod latency_source;

pub use bench_out::*;
pub use bench_run::*;
// pub use latency_source::*;

// pub trait Status<'a> {
//     fn warmup_status(&'a mut self) -> usize;
//     fn exec_status(&'a mut self) -> usize;
// }

// fn f1<'a, S: Status<'a>>(_s: &mut S) {}
// fn f2<'a, S: Status<'a>>(_s: &mut S) {}

// fn foo<'a, S: Status<'a>>(s: &'a mut S) {
//     f1(s);
//     f2(s);
// }
