#!/bin/bash

# NOCOVER environment variable enables tests that are excluded from test coverage measurement.
export NOCOVER="1" 

echo "*****  --lib --bins --tests (default feature)"
cargo nextest run --lib --bins --tests --features _dev_utils --target-dir target/test-target

# Can't run publicly without default features.
# echo "***** --no-default-features"
# cargo nextest run --lib --bins --tests --no-default-features --target-dir target/test-target

echo "***** --features _dev_utils,busy_work"
cargo nextest run --lib --bins --tests --features _dev_utils,busy_work --target-dir target/test-target

echo "***** --no-default-features --features _dev_utils,_bench_diff"
cargo nextest run --lib --bins --tests --no-default-features --features _dev_utils,_bench_diff --target-dir target/test-target


echo "***** doc"
cargo test --doc
