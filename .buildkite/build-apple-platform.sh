#!/bin/bash

set -euo pipefail

PLATFORM=$1

echo "--- :rust: Installing Rust"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -v -y

source "$HOME/.cargo/env"

echo "--- :package: Installing Rust Toolchains"
make setup-rust

echo "--- :rust: Building $PLATFORM targets"
make build-apple-$PLATFORM
