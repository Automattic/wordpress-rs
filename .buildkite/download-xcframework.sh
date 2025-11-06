#!/bin/bash

set -euo pipefail

echo "--- :arrow_down: Downloading XCFramework"
mkdir -p target
pushd target
buildkite-agent artifact download libwordpressFFI.xcframework.zip . --step "xcframework"
unzip libwordpressFFI.xcframework.zip -d .
rm libwordpressFFI.xcframework.zip
popd

echo "--- :arrow_down: Downloading Native WordPress API Wrapper"
buildkite-agent artifact download 'native/swift/Sources/wordpress-api-wrapper/*.swift' . --step "xcframework"
