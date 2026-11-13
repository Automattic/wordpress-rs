#!/bin/bash

if [ $# -eq 0 ]; then
  echo "No release version specified. Skipping release step."
  exit 0
fi

set -euo pipefail

echo '--- :robot_face: Use bot for Git operations'
source use-bot-for-git

echo "--- :rust: Installing Rust"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -v -y

source "$HOME/.cargo/env"

echo "--- :package: Installing Rust Toolchains"
make setup-rust

echo "--- :rubygems: Setting up Gems"
install_gems

echo "--- :closed_lock_with_key: Setting up Code Signing"
bundle exec fastlane set_up_signing_release

echo "--- :rust: Building XCFramework"
make xcframework-package-sign
make xcframework-package-checksum

# Re-resolve Swift package dependencies while Package.swift still points at the
# local xcframework. The release lane commits Package.resolved alongside the
# bindings, but resolving after Package.swift points at the GitHub release asset
# would 404: that asset is only uploaded once the tag exists. Binary targets are
# not pinned in Package.resolved, so resolving against the local xcframework
# produces the same file.
echo "--- :swift: Resolving Swift package dependencies"
swift package resolve

echo "--- :wastebasket: Removing XCFramework build intermediates"
rm -rf \
  target/*-apple-* \
  target/debug \
  target/release \
  target/libwordpressFFI.xcframework

release_version="$1"
echo "--- :rocket: Publish release $release_version"
bundle exec fastlane release "version:$release_version"
