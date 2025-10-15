#!/bin/bash

set -euo pipefail

# Requires a parameter whose value is the os and version part of simulator
# runtime identifier returned by `xcrun simctl list runtimes`.
platform=$1

device_id=$(xcrun simctl list --json devices available | jq -re ".devices.\"com.apple.CoreSimulator.SimRuntime.${platform}\" | last.udid")
device_name=$(xcrun simctl list --json devices available | jq -re ".devices.\"com.apple.CoreSimulator.SimRuntime.iOS-18-5\" | last.name")

echo "Runing on device: ${device_name}"

export NSUnbufferedIO=YES

xcodebuild \
    -scheme WordPressAPI-Package \
    -derivedDataPath DerivedData \
    -destination "id=${device_id}" \
    -skipPackagePluginValidation \
    test \
    | xcbeautify
