#!/bin/bash

echo "***** --all-targets --all-features"
cargo check --all-targets --all-features

echo "***** --lib --bins --tests (default feature)"
cargo check --lib --bins --tests

# Can't run publicly without default features.
# echo "***** --no-default-features"
# cargo check --lib --bins --tests --no-default-features

echo "***** --features busy_work"
cargo check --lib --bins --tests --features busy_work

echo "***** --no-default-features --features _bench_diff"
cargo check --lib --bins --tests --no-default-features --features _bench_diff
