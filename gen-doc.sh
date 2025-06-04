#!/bin/bash

rm -r target/doc

cargo makedocs -e sha2
cargo doc -p bench_utils --no-deps
