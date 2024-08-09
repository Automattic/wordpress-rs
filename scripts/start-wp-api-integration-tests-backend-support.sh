#!/bin/bash -eu

# The project should be mounted to this location
cd /app

cargo build --release -p wp_api_integration_tests_backend_support
nohup ./target/release/wp_api_integration_tests_backend_support > ./target/release/wp_api_integration_tests_backend_support.log &
