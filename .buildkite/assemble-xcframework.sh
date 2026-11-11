#!/bin/bash

# Assembles the (unsigned) xcframework from the libraries the `apple-build` steps built. Runs on a
# Linux agent; signing and packaging happen on a Mac in `sign-xcframework.sh`.

set -euo pipefail

if [[ "${INSIDE_SWIFT_DOCKER:-}" != "true" ]]; then
    echo "--- :arrow_down: Downloading pre-built Apple libraries"
    buildkite-agent artifact download 'target/*/release/libwp_mobile.a' .
    buildkite-agent artifact download 'target/*/release/swift-bindings/**' .

    .buildkite/commands/run-in-swift-docker.sh env INSIDE_SWIFT_DOCKER=true "$0"

    echo "--- :package: Archiving XCFramework"
    tar -czf target/libwordpressFFI.xcframework.tar.gz -C target libwordpressFFI.xcframework
    exit 0
fi

echo "--- :swift: Assembling XCFramework"
make xcframework-assemble
