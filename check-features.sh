#!/bin/bash

echo "***** --all-targets --all-features"
cargo check --all-targets --all-features

echo "***** --lib --tests (default feature)"
cargo check --lib --tests

# Can't run publicly without default features.
# echo "***** --no-default-features"
# cargo check --lib --tests --no-default-features

echo "***** --features busy_work"
cargo check --lib --tests --features busy_work

echo "***** --no-default-features --features _bench_diff"
cargo check --lib --tests --no-default-features --features _bench_diff
