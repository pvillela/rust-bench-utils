#!/bin/bash

## Executes long tests

export RUSTFLAGS="-Awarnings"

# NOCOVER environment variable enables tests that are excluded from test coverage measurement.
export NOCOVER="1"

# ./check-features.sh || { echo "Error: check-features failed"; exit 1; }

cargo run --example list_bench_tests --features "_test_support" | while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    part1="${line%%::*}"
    part2="${line#*::}"
    # ./processor "$part1" "$part2"
    echo "*** part1=$part1, part2=$part2"
    cargo test -r --test "$part1" --features _ALL_NON_TEST,_bench -- --test-threads=1 --exact "$part2"
done
