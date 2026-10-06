#!/bin/bash

set -euo pipefail

echo "--- :arrow_down: Downloading XCFramework"
buildkite-agent artifact download target/libwordpressFFI.xcframework.zip . --step "xcframework"
mkdir -p ./target/
unzip target/libwordpressFFI.xcframework.zip -d ./target/
rm target/libwordpressFFI.xcframework.zip

echo "--- :arrow_down: Downloading Native WordPress API Wrapper"
buildkite-agent artifact download 'native/swift/Sources/wordpress-api-wrapper/*.swift' . --step "xcframework"
