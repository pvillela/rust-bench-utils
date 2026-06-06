//! Show the difference `panic_on_error` makes when displaying bench output with no latency observations.
//!
//! ```
//! cargo run --example panic_on_error
//! ```

use bench_utils::{BenchCfg, BenchOut};

fn print_out(cfg: &BenchCfg) {
    let src = [].into_iter();
    let out = BenchOut::from_iter(&cfg, src);
    println!("*** panic_on_error={}", out.panic_on_error());
    println!("out.summary()={:?}", out.summary());
}

fn main() {
    {
        let cfg = BenchCfg::default();
        print_out(&cfg);
    }

    println!();

    {
        let cfg = BenchCfg::default().with_panic_on_error(true);
        let result = {
            std::panic::catch_unwind(|| {
                print_out(&cfg);
            })
        };
        println!("result with panic_on_error(true): {result:?}");
    }
}
