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

echo "--- :rust: Preparing XCFramework"
.buildkite/download-xcframework.sh
make xcframework-package-only
make xcframework-package-checksum

release_version="$1"
echo "--- :rocket: Publish release $release_version"
bundle exec fastlane release "version:$release_version"
