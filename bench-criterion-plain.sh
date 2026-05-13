#!/bin/bash

# Environment variables and their defaults:
#
# LATENCY_UNIT="micro" 
# BASE_MEDIAN="100"
# NREPEATS="10"

# TO_FILE="true"

# Command line arguments:
#
# $1  # NREPEATS

export RUSTFLAGS="-Awarnings"

export LATENCY_UNIT="${LATENCY_UNIT-"micro"}"
export BASE_MEDIAN="${BASE_MEDIAN-"100"}"
export NREPEATS="$1"

output_target="/dev/stdout" # Default to stdout
tag="${BASE_MEDIAN}_${LATENCY_UNIT}"

if [[ -z "$TO_FILE" || "${TO_FILE,,}" == "true" ]]; then
    timestamp=$(date +"%Y%m%d_%H%M")
    output_target="out/crit-plain-${tag}-${timestamp}.txt"
fi

echo "Started ${tag} at: `date +"%H:%M:%S"`, output_target=$output_target" | tee /dev/stderr > $output_target

cargo criterion --bench criterion_plain --features _dev_support,_benches,busy_work --target-dir target/bench-target \
2>> $output_target # target/bench-target --message-format=json

echo "Finished ${tag} at: `date +"%H:%M:%S"`" | tee /dev/stderr >> $output_target

