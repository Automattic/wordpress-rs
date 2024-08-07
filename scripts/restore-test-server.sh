#!/bin/bash

set -e

# Load the Rust toolchain into this shell
source $HOME/.cargo/env

# The project should be mounted to this location
cd /app

# Use the `restore-wp-content-plugins` option of `wp_api_integration_tests` binary
cargo run --bin wp_api_integration_tests restore-wp-content-plugins
