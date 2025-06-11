use std::hint::black_box;

use bench_utils::latency;

fn main() {
    let x = latency(|| black_box(()));
    println!("{}", x.as_millis());
}
