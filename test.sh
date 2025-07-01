#!/bin/bash

export RUSTFLAGS="-Awarnings"

cargo nextest run --lib --bins --examples --tests --features _dev_utils --target-dir target/test-target
cargo test --doc
