#!/bin/bash

export RUSTFLAGS="-Awarnings"

# NOCOVER environment variable enables tests that are excluded from test coverage measurement.
export NOCOVER="1"

./check-features.sh || { echo "Error: check-features failed"; exit 1; }

echo "***** test all except benches, all features"
# cargo nextest run  --lib --bins --examples --tests --all-features --target-dir target/test-target
# cargo test -r  --lib --bins --examples --tests --all-features -- --nocapture --test-threads=1
cargo test -r  --lib --bins --examples --tests --all-features -- --test-threads=1

echo "***** test doc"
cargo test --doc --features busy_work --target-dir target/test-target
