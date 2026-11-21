#!/bin/bash

set -euo pipefail

.buildkite/download-xcframework.sh

export SKIP_PACKAGE_WP_API=true

echo "--- :swift: Restoring Swift Package Manager cache"
restore_swiftpm_cache --package-resolved Package.resolved

function run_tests() {
    local platform; platform=$1
    echo "--- :swift: Testing on $platform"
    local refused; refused=$(scripts/badssl-server.sh refused-handshakes)
    make "test-swift-$platform"
    echo "The badssl.com copy saw $(( $(scripts/badssl-server.sh refused-handshakes) - refused )) refused TLS handshakes"
}

# The TLS tests use a copy of badssl.com on this machine rather than the live site.
function start_badssl() {
    echo "--- :lock: Starting a copy of badssl.com"
    command -v nginx > /dev/null || brew install nginx
    scripts/badssl-server.sh prepare
    scripts/badssl-server.sh start

    # `scripts/xcodebuild-test.sh` trusts its root in each simulator, so the tests there see the
    # name mismatches it serves rather than an unknown root. The macOS and Mac Catalyst runs don't
    # trust it: the CI image refuses `security add-trusted-cert`, as it needs someone to confirm it.
    BADSSL_ROOT_CERT=$(scripts/badssl-server.sh root-cert)
    export BADSSL_ROOT_CERT

    export BADSSL_DOMAIN=badssl.localhost
    # `xcodebuild test` passes a `TEST_RUNNER_` variable to the tests without its prefix.
    export TEST_RUNNER_BADSSL_DOMAIN=badssl.localhost
}

function build_for_real_device() {
    local platform; platform=$1

    local destination="generic/platform=$platform"
    if [ "$platform" == "Catalyst" ]; then
        destination="generic/platform=macOS,variant=Mac Catalyst"
    fi

    echo "--- :swift: Building for $platform device"
    export NSUnbufferedIO=YES
    xcodebuild -destination "$destination" \
        -scheme WordPressAPI-Package \
        -derivedDataPath DerivedData \
        -skipPackagePluginValidation \
        build | xcbeautify
}

func=$1

if [ "$func" == "run_tests" ]; then
    start_badssl
fi

for platform in "iOS" "macOS" "Catalyst" "tvOS" "watchOS"; do
    $func $platform
done

echo "--- :swift: Saving Swift Package Manager cache"
save_swiftpm_cache --package-resolved Package.resolved
