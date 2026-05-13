#!/bin/bash

echo "***** (default feature)"
cargo check --lib --tests

# Can't run publicly without default features.
# echo "***** --no-default-features"
# cargo check --lib --tests --no-default-features

echo "***** --features busy_work"
cargo check --lib --tests --features busy_work

echo "***** --no-default-features --features _bench_diff"
cargo check --lib --tests --no-default-features --features _bench_diff

echo "***** --no-default-features --features _dev_support"
cargo check --lib --tests --no-default-features --features _dev_support

echo "***** --examples --all-features"
cargo check --examples --all-features

echo "***** --benches --all-features"
cargo check --benches --all-features

