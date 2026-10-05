#!/bin/bash -eu

# Runs the Rust integration tests on a macOS agent, against a test server running directly in the VM
# rather than in Docker – the VMs can't run Docker.

echo "--- :rust: Installing Rust"
# No default toolchain – `rust-toolchain.toml` picks the one to install.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -v -y --default-toolchain none

source "$HOME/.cargo/env"

echo "--- :homebrew: Installing Test Server Dependencies"
make test-server-native-deps

make test-server-native

echo "--- 🧪 Running Rust Integration Tests"
make test-rust-integration-native
