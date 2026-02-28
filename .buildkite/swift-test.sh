#!/bin/bash

set -euo pipefail

.buildkite/download-xcframework.sh

export SKIP_PACKAGE_WP_API=true

# Create and unlock a temporary keychain for SecPKCS12Import (used by MockWebServer TLS tests).
# On CI, the default keychain may be locked, causing errSecInteractionNotAllowed (-25308).
if [ "${BUILDKITE:-}" = "true" ]; then
    echo "--- :key: Setting up keychain for TLS tests"
    security delete-keychain ci-test.keychain-db 2>/dev/null || true
    security create-keychain -p "" ci-test.keychain-db
    security default-keychain -s ci-test.keychain-db
    security unlock-keychain -p "" ci-test.keychain-db
    security set-keychain-settings ci-test.keychain-db
fi

function run_tests() {
    local platform; platform=$1

    if [ "$platform" = "iOS" ]; then
        echo "--- :lock: Trusting test CA certificate"
        make trust-test-ca || echo "⚠️ Could not trust test CA — spec 19 (custom CA cert) will be skipped"
    fi

    echo "--- :swift: Testing on $platform simulator"
    make "test-swift-$platform"
}

function build_for_real_device() {
    local platform; platform=$1

    echo "--- :swift: Building for $platform device"
    export NSUnbufferedIO=YES
    xcodebuild -destination "generic/platform=$platform" \
        -scheme WordPressAPI-Package \
        -derivedDataPath DerivedData \
        -skipPackagePluginValidation \
        build | xcbeautify
}

func=$1

for platform in "iOS" "macOS" "tvOS" "watchOS"; do
    $func $platform
done
