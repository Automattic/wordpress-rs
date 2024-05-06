#!/bin/bash

set -euo pipefail

echo "--- :arrow_down: Downloading xcframework"
buildkite-agent artifact download target/libwordpressFFI.xcframework.zip . --step "xcframework"
buildkite-agent artifact download native/swift/Sources/wordpress-api-wrapper/wp_api.swift . --step "xcframework"
unzip target/libwordpressFFI.xcframework.zip -d .
rm target/libwordpressFFI.xcframework.zip
export SKIP_PACKAGE_WP_API=true

function run_tests() {
    local platform; platform=$1
    echo "--- :swift: Testing on $platform simulator"
    make "test-swift-$platform"
}

function build_for_real_device() {
    local platform; platform=$1

    # See https://github.com/Automattic/wordpress-rs/issues/48
    if [[ $platform == "watchOS" ]]; then
        echo "~~~ watchOS is not supported yet"
        return
    fi

    echo "--- :swift: Building for $platform device"
    export NSUnbufferedIO=YES
    xcodebuild -destination "generic/platform=$platform" \
        -scheme WordPress \
        -derivedDataPath DerivedData \
        build | xcbeautify
}

func=$1

for platform in "iOS" "macOS" "tvOS" "watchOS"; do
    $func $platform
done
