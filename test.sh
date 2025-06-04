#!/bin/bash

export RUSTFLAGS="-Awarnings"

cargo nextest run --lib --bins --examples --tests --target-dir target/test-target
cargo test --doc
