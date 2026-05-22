#!/bin/bash

set -e  # Stop script immediately on any error

### With default features

echo "***** (default feature)"
cargo check --lib --tests

echo "***** --features busy_work"
cargo check --lib --tests --features busy_work

echo "*****  --features _bench"
cargo check --lib --tests --features _bench

echo "***** --features _bench_diff"
cargo check --lib --tests --features _bench_diff

echo "*****  --features _experimental"
cargo check --lib --tests --features _experimental

echo "*****  --features _test"
cargo check --lib --tests --features _test

echo "*****  --features _experimental,_test"
cargo check --lib --tests --features _experimental,_test

## All targets and features

cargo check --all-targets --all-features

### Without default features

# Can't run any code without default features.

# Any `cargo check --lib --tests --no-default-features --features <whatever>` fails because there are ungated 
# modules that depend on the dependencies brought in by "default".

### Benches

echo "***** benches"
cargo check --benches --features _bench
