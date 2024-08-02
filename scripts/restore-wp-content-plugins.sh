#!/bin/bash

set -e

# Load the Rust toolchain into this shell
source $HOME/.cargo/env

# The project should be mounted to this location
cd /app

# Run the test suite
cargo run --bin wp_api_integration_tests --target-dir /target restore-wp-content-plugins
