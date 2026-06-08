#!/bin/bash

## Executes long tests

export RUSTFLAGS="-Awarnings"

# NOCOVER environment variable enables tests that are excluded from test coverage measurement.
export NOCOVER="1"

./check-features.sh || { echo "Error: check-features failed"; exit 1; }

cargo test -r --tests --features _ALL_NON_TEST,_bench -- --test-threads=1
