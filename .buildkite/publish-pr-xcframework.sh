#!/bin/bash

set -euo pipefail

# Only run for PR builds
if [[ "${BUILDKITE_PULL_REQUEST:-false}" == "false" ]]; then
  echo "Not a PR build, skipping PR xcframework publish"
  exit 0
fi

PR_NUMBER="$BUILDKITE_PULL_REQUEST"

echo '--- :robot_face: Use bot for Git operations'
source use-bot-for-git

echo "--- :arrow_down: Downloading XCFramework artifacts"
buildkite-agent artifact download target/libwordpressFFI.xcframework.zip . --step "xcframework"
buildkite-agent artifact download target/libwordpressFFI.xcframework.zip.checksum.txt . --step "xcframework"
buildkite-agent artifact download 'native/swift/Sources/wordpress-api-wrapper/*.swift' . --step "xcframework"

echo "--- :rubygems: Setting up Gems"
install_gems

echo "--- :rocket: Publishing PR build for PR #${PR_NUMBER}"
bundle exec fastlane publish_pr_xcframework pr_number:"$PR_NUMBER"
