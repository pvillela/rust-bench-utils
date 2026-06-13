#!/bin/bash

## Executes long tests

export RUSTFLAGS="-Awarnings"

# NOCOVER environment variable enables tests that are excluded from test coverage measurement.
export NOCOVER="1"

./check-features.sh || { echo "Error: check-features failed"; exit 1; }

cargo test -r --tests --test '*' --features _ALL_NON_TEST,_bench --no-fail-fast -- --test-threads=1
