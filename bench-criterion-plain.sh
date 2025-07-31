#!/bin/bash

export RUSTFLAGS="-Awarnings"

echo "----- Started: `date +"%Y-%m-%d at %H:%M:%S"` -----"
echo

cargo criterion --bench criterion_plain --features _dev_utils,criterion --target-dir target/bench-target --message-format=json

echo ""
echo "Finished at: `date +"%H:%M:%S"`"

