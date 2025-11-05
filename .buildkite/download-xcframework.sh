#!/bin/bash

set -euo pipefail

echo "--- :arrow_down: Downloading xcframework"
buildkite-agent artifact download libwordpressFFI.xcframework.zip . --step "xcframework"
buildkite-agent artifact download 'native/swift/Sources/wordpress-api-wrapper/*.swift' . --step "xcframework"
unzip libwordpressFFI.xcframework.zip -d .
rm libwordpressFFI.xcframework.zip
