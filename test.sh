#!/bin/bash

export RUSTFLAGS="-Awarnings"

# NOCOVER environment variable enables tests that are excluded from test coverage measurement.
export NOCOVER="1"

./check-features.sh || { echo "Error: check-features failed"; exit 1; }

# echo "***** test non-bench tests"
cargo nextest run --tests --features _ALL_NON_TEST,_test --no-fail-fast

echo "***** test doc"
cargo test --doc --features load --target-dir target/test-target
