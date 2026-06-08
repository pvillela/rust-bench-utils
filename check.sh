#!/bin/bash

set -e  # Stop script immediately on any error

## All targets and features

cargo check --all-targets --all-features
