#!/bin/bash

set -euo pipefail

# TEMP: this build has no xcframework step, so these download the one from trunk build 6536 – nothing
# that goes into it has changed since. Restore before merging.

echo "--- :arrow_down: Downloading XCFramework"
buildkite-agent artifact download target/libwordpressFFI.xcframework.zip . --step "xcframework" --build "01a110f9-2059-4193-9a44-5bf16526421d"
mkdir -p ./target/
unzip target/libwordpressFFI.xcframework.zip -d ./target/
rm target/libwordpressFFI.xcframework.zip

echo "--- :arrow_down: Downloading Native WordPress API Wrapper"
buildkite-agent artifact download 'native/swift/Sources/wordpress-api-wrapper/*.swift' . --step "xcframework" --build "01a110f9-2059-4193-9a44-5bf16526421d"
