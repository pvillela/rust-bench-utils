#!/bin/bash

### With default feature

echo "***** (default feature)"
cargo check --lib --tests

echo "***** --features busy_work"
cargo check --lib --tests --features busy_work

echo "***** --features _dev_support"
cargo check --lib --tests --features _dev_support

### Without default feature

# Can't run publicly without default features.
# echo "***** --no-default-features"
# cargo check --lib --tests --no-default-features

echo "***** --no-default-features --features _bench_diff"
cargo check --lib --tests --no-default-features --features _bench_diff

echo "***** --no-default-features --features _dev_support (there should be no modules)"
cargo check --lib --tests --no-default-features --features _dev_support

### Examples

echo "***** --examples --all-features"
cargo check --examples --all-features

### Benches

echo "***** --benches --all-features"
cargo check --benches --all-features
