#!/bin/bash -eu

# Runs an integration test suite on a macOS agent, against a test server running directly in the VM
# rather than in Docker – the VMs can't run Docker.

SUITE=$1

echo "--- :rust: Installing Rust"
# No default toolchain – `rust-toolchain.toml` picks the one to install.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -v -y --default-toolchain none

source "$HOME/.cargo/env"

echo "--- :homebrew: Installing Test Server Dependencies"
make test-server-native-deps

make test-server-native

echo "--- 🧪 Running Integration Tests"
make "test-$SUITE-integration-native"
