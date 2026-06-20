//! Example displaying a range of [`FpSeconds`] values.
//!
//! ```
//! cargo run --package bench_utils --example fpsecs_debug --all-features
//! ```

use bench_utils::FpSeconds;

fn print_fps(f64: f64) {
    let fps = FpSeconds(f64);
    println!("f64-display={f64}, f64-debug={f64:?}, fps-debug={fps:?}");
}

fn main() {
    let f64 = 12345.1234567890e0;
    print_fps(f64);

    let f64 = f64 * 1e-1;
    print_fps(f64);

    let f64 = f64 * 1e-1;
    print_fps(f64);

    let f64 = f64 * 1e-1;
    print_fps(f64);

    let f64 = f64 * 1e-1;
    print_fps(f64);

    let f64 = f64 * 1e-1;
    print_fps(f64);

    let f64 = f64 * 1e-1;
    print_fps(f64);

    let f64 = f64 * 1e-1;
    print_fps(f64);

    let f64 = f64 * 1e-1;
    print_fps(f64);

    let f64 = f64 * 1e-1;
    print_fps(f64);

    let f64 = f64 * 1e-1;
    print_fps(f64);

    let f64 = f64 * 1e-1;
    print_fps(f64);

    let f64 = f64 * 1e-1;
    print_fps(f64);

    let f64 = f64 * 1e-1;
    print_fps(f64);

    let f64 = f64 * 1e-1;
    print_fps(f64);

    let f64 = f64 * 1e-1;
    print_fps(f64);

    let f64 = f64 * 1e-1;
    print_fps(f64);

    let f64 = f64 * 1e-1;
    print_fps(f64);

    let f64 = f64 * 1e-1;
    print_fps(f64);
}
