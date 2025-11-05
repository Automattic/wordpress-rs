#!/bin/bash

set -euo pipefail

echo "--- :arrow_down: Downloading xcframework"
buildkite-agent artifact download libwordpressFFI.xcframework.zip . --step "xcframework"
unzip libwordpressFFI.xcframework.zip -d .
rm libwordpressFFI.xcframework.zip

buildkite-agent artifact download 'native/swift/Sources/wordpress-api-wrapper/*.swift' . --step "xcframework"

# Temporarily also download to target folder because some parts still expect it there
mkdir -p ./target
pushd target
buildkite-agent artifact download libwordpressFFI.xcframework.zip . --step "xcframework"
unzip libwordpressFFI.xcframework.zip -d .
rm libwordpressFFI.xcframework.zip
