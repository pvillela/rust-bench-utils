#!/bin/bash

export RUSTFLAGS="-Awarnings"

cargo nextest run --lib --bins --examples --tests --features _dev_utils,busy_work --target-dir target/test-target
cargo test --doc
