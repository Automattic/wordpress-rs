#!/bin/bash

set -euo pipefail

PLATFORM=$1

echo "--- :rust: Installing Rust"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -v -y

source "$HOME/.cargo/env"

echo "--- :package: Installing Rust Toolchains"
make setup-rust-$PLATFORM

echo "--- :swift: Restoring Swift Package Manager cache"
restore_swiftpm_cache --package-resolved native/swift/Tools/Package.resolved

echo "--- :rust: Building $PLATFORM targets"
make build-apple-$PLATFORM

echo "--- :swift: Saving Swift Package Manager cache"
save_swiftpm_cache --package-resolved native/swift/Tools/Package.resolved
