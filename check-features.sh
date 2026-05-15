#!/bin/bash

set -e  # Stop script immediately on any error

### With default feature

## Externally exposed feature combinations

echo "***** (default feature)"
cargo check --lib --tests

echo "***** --features busy_work"
cargo check --lib --tests --features busy_work

echo "***** --features _bench_diff"
cargo check --lib --tests --features _bench_diff

## All targets and features

echo "***** --all-targets --all-features"
cargo check --all-targets --all-features

## Benches

echo "***** --features _bench"
cargo check --lib --tests --benches --features _bench

### Without default feature

# Can't run publicly without default features.

echo "***** --no-default-features --features _bench_diff"
cargo check --lib --tests --no-default-features --features _bench_diff
