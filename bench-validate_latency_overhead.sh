#!/bin/bash

export RUSTFLAGS="-Awarnings"

echo "----- Started: `date +"%Y-%m-%d at %H:%M:%S"` -----"
echo

cargo bench --bench validate_latency_overhead --features _bench --target-dir target/bench-target

echo ""
echo "Finished at: `date +"%H:%M:%S"`"

