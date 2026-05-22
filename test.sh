#!/bin/bash

export RUSTFLAGS="-Awarnings"

# NOCOVER environment variable enables tests that are excluded from test coverage measurement.
export NOCOVER="1"

./check-features.sh || { echo "Error: check-features failed"; exit 1; }

# echo "***** test non-bench tests"
cargo nextest run --tests --features _all_non_test,_test

# echo "***** test bench tests"
cargo test -r --tests --features _all_non_test,_bench -- --test-threads=1
# cargo test -r --tests --all-features -- --test-threads=1

echo "***** test doc"
cargo test --doc --features busy_work --target-dir target/test-target
