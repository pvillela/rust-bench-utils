use bench_utils::test_support::process_directory_tests;
use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Collect command line arguments (index 0 is the binary name, index 1 is $1)
    let args: Vec<String> = env::args().collect();

    // Check if the argument was provided
    if args.len() < 2 {
        eprintln!("Error: Missing directory path argument.");
        eprintln!("Usage: {} <directory_path>", args[0]);
        std::process::exit(1);
    }

    let dir_path = &args[1];
    let path = Path::new(dir_path);

    // Validate that the path actually exists and is a directory
    if !path.exists() {
        eprintln!("Error: The path '{}' does not exist.", dir_path);
        std::process::exit(1);
    }
    if !path.is_dir() {
        eprintln!("Error: The path '{}' is not a directory.", dir_path);
        std::process::exit(1);
    }

    // Call your directory processing function
    process_directory_tests(path)?;

    Ok(())
}
