#!/bin/bash

export RUSTFLAGS="-Awarnings"

echo "----- Started: `date +"%Y-%m-%d at %H:%M:%S"` -----"
echo

cargo bench --bench criterion_plain --features _dev_utils,criterion --target-dir target/bench-target

echo ""
echo "Finished at: `date +"%H:%M:%S"`"

