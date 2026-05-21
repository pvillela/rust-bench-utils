#!/bin/bash

## Runs all benches whose names start with the string passed as this script's argument.

set -e

if [ $# -ne 1 ]; then
    echo "Usage: $0 <prefix>"
    echo ""
    echo "Runs all benches whose names start with <prefix>."
    echo "Example: $0 validate"
    exit 1
fi

prefix="$1"
bench_dir="$(dirname "$0")/benches"

# Find matching bench files (exclude the support/ subdirectory)
mapfile -t bench_files < <(find "$bench_dir" -maxdepth 1 -name "${prefix}*.rs" -printf '%f\n' | sort)

if [ ${#bench_files[@]} -eq 0 ]; then
    echo "No bench files found matching prefix '$prefix'"
    exit 0
fi

echo "Found ${#bench_files[@]} bench(es) matching '$prefix':"
for f in "${bench_files[@]}"; do
    echo "  - ${f%.rs}"
done
echo ""

for f in "${bench_files[@]}"; do
    bench_name="${f%.rs}"
    echo "=== Running bench: $bench_name ==="
    cargo bench --features _bench --bench "$bench_name"
done

echo ""
echo "All matching benches completed."
