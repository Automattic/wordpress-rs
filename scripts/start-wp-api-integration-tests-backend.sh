#!/bin/bash -eu

# The project should be mounted to this location
cd /app

cargo build --release -p wp_api_integration_tests_backend
nohup ./target/release/wp_api_integration_tests_backend > ./target/release/wp_api_integration_tests_backend.log &
