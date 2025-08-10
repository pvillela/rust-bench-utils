#!/bin/bash

# Environment variables and their defaults:
#
# TARGET_RATIO="1.1"
# LATENCY_UNIT="micro" 
# BASE_MEDIAN="100"
# NREPEATS="10"

# TO_FILE="true"

# Command line arguments:
#
# $1  # TARGET_RATIO
# $2  # NREPEATS

export RUSTFLAGS="-Awarnings"

export LATENCY_UNIT="${LATENCY_UNIT-"micro"}"
export BASE_MEDIAN="${BASE_MEDIAN-"100"}"
export TARGET_RATIO="$1"
export NREPEATS="$2"

output_target="/dev/stdout" # Default to stdout
tag="${BASE_MEDIAN}_${LATENCY_UNIT}_x${TARGET_RATIO}"

if [[ -z "$TO_FILE" || "${TO_FILE,,}" == "true" ]]; then
    timestamp=$(date +"%Y%m%d_%H%M")
    output_target="out/crit-comp-${tag}-${timestamp}.txt"
fi

echo "Started ${tag} at: `date +"%H:%M:%S"`, output_target=$output_target" | tee /dev/stderr > $output_target

cargo criterion --bench criterion_comp --features _dev_utils,criterion,busy_work --target-dir target/bench-target 2>> $output_target

echo "Finished ${tag} at: `date +"%H:%M:%S"`" | tee /dev/stderr >> $output_target

