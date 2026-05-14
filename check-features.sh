#!/bin/bash

set -e  # Stop script immediately on any error

### With default feature

echo "***** --all-targets --all-features"
cargo check --all-targets --all-features

echo "***** (default feature)"
cargo check --lib --tests

echo "***** --features busy_work"
cargo check --lib --tests --features busy_work

echo "***** --features _dev_support"
cargo check --lib --tests --features _dev_support

echo "***** --features _benches"
cargo check --lib --tests --features _benches

echo "***** --features _experimental"
cargo check --lib --tests --features _experimental

echo "***** --features _bench_diff"
cargo check --lib --tests --features _bench_diff

### Without default feature

# Can't run publicly without default features.
# echo "***** --no-default-features"
# cargo check --lib --tests --no-default-features
