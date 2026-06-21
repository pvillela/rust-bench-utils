//! ```
//! cargo run --package bench_utils --example list_integr_bench_tests --features "_test_support"
//! ```

use bench_utils::test_support::process_directory_tests;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir_path = "tests";
    let path = Path::new(dir_path);
    process_directory_tests(path)
}
