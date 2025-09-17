#!/bin/bash

set -euo pipefail

echo "--- :arrow_down: Downloading xcframework"
buildkite-agent artifact download target/libwordpressFFI.xcframework.zip . --step "xcframework"
buildkite-agent artifact download 'native/swift/Sources/wordpress-api-wrapper/*.swift' . --step "xcframework"
unzip target/libwordpressFFI.xcframework.zip -d .
rm target/libwordpressFFI.xcframework.zip
