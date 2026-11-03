#!/bin/bash -eu

# Runs the Swift integration tests on a macOS agent, against a test server running directly in the VM
# rather than in Docker – the VMs can't run Docker.
#
# The Docker-based step runs these tests on Linux, so this is the only step that runs the ones that
# are only compiled on macOS: upload progress and cancellation.

.buildkite/download-xcframework.sh

echo "--- :rust: Installing Rust"
# The integration tests backend is built from source. No default toolchain – `rust-toolchain.toml`
# picks the one to install.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -v -y --default-toolchain none

source "$HOME/.cargo/env"

echo "--- :beer: Installing Test Server Dependencies"
make test-server-native-deps

make test-server-native

echo "--- 🧪 Running Swift Integration Tests"
make test-swift-integration-native
