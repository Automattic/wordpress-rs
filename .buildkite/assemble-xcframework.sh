#!/bin/bash

set -euo pipefail

echo "--- :arrow_down: Downloading pre-built Apple libraries"
buildkite-agent artifact download 'target/*/release/libwp_mobile.a' .
buildkite-agent artifact download 'target/*/release/swift-bindings/**' .
buildkite-agent artifact download 'native/swift/Sources/wordpress-api-wrapper/*.swift' .

echo "--- :swift: Assembling XCFramework"
make xcframework-assemble

echo "--- :package: Packaging XCFramework"
rm -rf target/libwordpressFFI.xcframework.zip
ditto -c -k --sequesterRsrc --keepParent target/libwordpressFFI.xcframework/ target/libwordpressFFI.xcframework.zip
