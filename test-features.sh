#!/bin/bash

# NOCOVER environment variable enables tests that are excluded from test coverage measurement.
export NOCOVER="1" 

echo "***** (default feature)"
cargo nextest run --lib --tests --target-dir target/test-target

# Can't run publicly without default features.
# echo "***** --no-default-features"
# cargo nextest run --lib --tests --no-default-features --target-dir target/test-target

echo "***** --features busy_work"
cargo nextest run --lib --tests --features busy_work --target-dir target/test-target

echo "***** --no-default-features --features _bench_diff"
cargo nextest run --lib --tests --no-default-features --features _bench_diff --target-dir target/test-target

echo "***** --no-default-features --features _dev_support"
cargo nextest run --lib --tests --no-default-features --features _dev_support --target-dir target/test-target

echo "***** --examples --all-features"
cargo nextest run --examples --all-features --target-dir target/test-target


echo "***** --bench (selected benches)"
cargo bench --bench validate_latency_overhead --all-features


echo "***** doc"
cargo test --doc
