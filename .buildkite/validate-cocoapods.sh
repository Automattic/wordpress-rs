#!/bin/bash

set -euo pipefail

echo "--- :arrow_down: Downloading xcframework"
buildkite-agent artifact download target/libwordpressFFI.xcframework.zip . --step "xcframework"
buildkite-agent artifact download native/swift/Sources/wordpress-api-wrapper/wp_api.swift . --step "xcframework"
unzip target/libwordpressFFI.xcframework.zip -d .
rm target/libwordpressFFI.xcframework.zip
export SKIP_PACKAGE_WP_API=true

echo "--- :rubygems: Setting up Gems"
install_gems

echo "--- :cocoapods: Validating Podspec"
validate_podspec
