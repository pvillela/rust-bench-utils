#!/bin/bash

rm -r target/doc

cargo makedocs -e rand -e rand_distr -e sha2 -e statrs -e basic_stats -e hdrhistogram
cargo doc -p bench_utils --no-deps
