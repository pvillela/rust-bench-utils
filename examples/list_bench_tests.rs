//! ```
//! cargo run --package bench_utils --example list_bench_tests --features "_test_support"
//! ```

use bench_utils::test_support::process_directory_tests;
// use bench_utils::test_support::print_test_functions;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir_path = "tests";
    let path = Path::new(dir_path);
    process_directory_tests(path)
    // let dir_path = "tests/bench_run_validate.rs";
    // let path = Path::new(dir_path);
    // print_test_functions(path)
}
