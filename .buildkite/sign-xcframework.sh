#!/bin/bash

# Signs and packages the xcframework that `assemble-xcframework.sh` built on Linux. Signing needs
# the distribution certificate and `codesign`, so this runs on a Mac.

set -euo pipefail

echo "--- :arrow_down: Downloading XCFramework"
buildkite-agent artifact download target/libwordpressFFI.xcframework.tar.gz . --step xcframework-assemble
rm -rf target/libwordpressFFI.xcframework
tar -xzf target/libwordpressFFI.xcframework.tar.gz -C target
rm target/libwordpressFFI.xcframework.tar.gz

# Re-uploaded from this step (see `artifact_paths`), which is where downstream steps fetch them.
buildkite-agent artifact download 'native/swift/Sources/wordpress-api-wrapper/*.swift' .

echo "--- :lock: Signing XCFramework"
make xcframework-sign

echo "--- :package: Packaging XCFramework"
rm -rf target/libwordpressFFI.xcframework.zip
ditto -c -k --sequesterRsrc --keepParent target/libwordpressFFI.xcframework/ target/libwordpressFFI.xcframework.zip
make xcframework-package-checksum
