#!/bin/bash

set -e

# Load the Rust toolchain into this shell
source $HOME/.cargo/env

# The project should be mounted to this location
cd /app

# Run the test suite
cargo test -p wp_api_integration_tests -q --no-fail-fast --target-dir /target
