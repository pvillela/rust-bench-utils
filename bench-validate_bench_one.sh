#!/bin/bash

export RUSTFLAGS="-Awarnings"

echo "----- Started: `date +"%Y-%m-%d at %H:%M:%S"` -----"
echo

cargo bench --bench validate_bench_one --features _dev_utils --target-dir target/bench-target

echo ""
echo "Finished at: `date +"%H:%M:%S"`"

