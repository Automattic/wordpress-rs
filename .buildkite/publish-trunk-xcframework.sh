#!/bin/bash

set -euo pipefail

echo '--- :robot_face: Use bot for Git operations'
source use-bot-for-git

echo "--- :arrow_down: Downloading XCFramework artifacts"
buildkite-agent artifact download target/libwordpressFFI.xcframework.zip . --step "xcframework"
buildkite-agent artifact download target/libwordpressFFI.xcframework.zip.checksum.txt . --step "xcframework"
buildkite-agent artifact download 'native/swift/Sources/wordpress-api-wrapper/*.swift' . --step "xcframework"

echo "--- :rubygems: Setting up Gems"
install_gems

echo "--- :rocket: Publishing trunk build for ${BUILDKITE_COMMIT}"
bundle exec fastlane publish_trunk_xcframework commit_sha:"$BUILDKITE_COMMIT"
