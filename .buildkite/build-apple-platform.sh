#!/bin/bash

# Builds the Rust libraries and Swift bindings for one Apple platform. Runs on a Linux agent: see
# `commands/run-in-swift-docker.sh`.

set -euo pipefail

PLATFORM=$1

if [[ "${INSIDE_SWIFT_DOCKER:-}" != "true" ]]; then
    exec .buildkite/commands/run-in-swift-docker.sh env INSIDE_SWIFT_DOCKER=true "$0" "$PLATFORM"
fi

echo "--- :package: Installing Rust Toolchains"
make setup-rust-"$PLATFORM"

echo "--- :rust: Building $PLATFORM targets"
make build-apple-"$PLATFORM"
