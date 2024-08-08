#!/bin/bash -eu

# The project should be mounted to this location
cd /app

# Run the test suite
cargo test -p wp_api_integration_tests --no-fail-fast
