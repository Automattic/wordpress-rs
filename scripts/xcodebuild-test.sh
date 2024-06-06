#!/bin/bash

set -euo pipefail

# Requires a parameter whose value is the os and version part of simulator
# runtime identifier returned by `xcrun simctl list runtimes`.
platform=$1

device_id=$(xcrun simctl list --json devices available | jq -re ".devices.\"com.apple.CoreSimulator.SimRuntime.${platform}\" | last.udid")

export NSUnbufferedIO=YES

xcodebuild \
    -scheme WordPress \
    -derivedDataPath DerivedData \
    -destination "id=${device_id}" \
    -skipPackagePluginValidation \
    test \
    | xcbeautify
