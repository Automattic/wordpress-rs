#!/bin/bash

set -euo pipefail

# Commit a podspec file to the repo may give a fasle sense of CocoaPods support.
# This script validates the podspec file generated by the build script.
# We may need to tweak the podspec content if the project structure changes.
#
# Here are a few potential fixes if this script fails on CI.
# - There is no "WordPressAPIInternal" module when building using CocoaPods.
#   Be sure to add `#if canImport(WordPressAPIInternal)` before `import WordPressAPIInternal`.

echo "--- :hammer: Generating a podspec file"
cat <<EOT | tee WordPressAPI.podspec
Pod::Spec.new do |spec|
  spec.name         = "WordPressAPI"
  spec.version      = "0.0.1"
  spec.summary      = "WordPressAPI."
  spec.description  = "WordPress API in Swift."
  spec.homepage     = "https://github.com/automattic/wordpress-rs"
  spec.license      = "MIT"
  spec.author       = { 'The WordPress Mobile Team' => 'mobile@wordpress.org' }

  spec.ios.deployment_target = '13.0'
  spec.osx.deployment_target = '11.0'

  # zip -r swift-source-archive.zip native/swift target/libwordpressFFI.xcframework
  spec.source       = { :http => "http://s3.com/WordPressAPI.zip" }

  spec.swift_version = '5.10'
  spec.source_files  = 'native/swift/Sources/**/*.{swift}'
  spec.vendored_frameworks = 'target/libwordpressFFI.xcframework'

  spec.pod_target_xcconfig = {
    'SWIFT_PACKAGE_NAME' => 'WordPressAPI'
  }

  spec.test_spec 'Tests' do |test_spec|
    test_spec.source_files = 'native/swift/Tests/**/*.{swift}'
    test_spec.resource_bundles = { 'Resources' => ['native/swift/Tests/wordpress-api/Resources/*'] }
  end
end
EOT

.buildkite/download-xcframework.sh

export SKIP_PACKAGE_WP_API=true

echo "--- :rubygems: Setting up Gems"
install_gems

echo "--- :cocoapods: Validating Podspec"
validate_podspec --allow-warnings
