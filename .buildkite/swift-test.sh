#!/bin/bash

set -euo pipefail

.buildkite/download-xcframework.sh

export SKIP_PACKAGE_WP_API=true

function run_tests() {
    local platform; platform=$1
    echo "--- :swift: Testing on $platform simulator"
    make "test-swift-$platform"
}

function build_for_real_device() {
    local platform; platform=$1

    echo "--- :swift: Building for $platform device"
    export NSUnbufferedIO=YES
    xcodebuild -destination "generic/platform=$platform" \
        -scheme WordPress \
        -derivedDataPath DerivedData \
        -skipPackagePluginValidation \
        build | xcbeautify
}

func=$1

for platform in "iOS" "macOS" "tvOS" "watchOS"; do
    $func $platform
done
