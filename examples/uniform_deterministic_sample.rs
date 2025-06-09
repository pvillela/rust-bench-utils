use bench_utils  ::deterministic_sample::deterministic_uniform_sample;

fn main() {
    let iter = deterministic_uniform_sample(10);
    for item in iter {
        println!("{item}");
    }
}
