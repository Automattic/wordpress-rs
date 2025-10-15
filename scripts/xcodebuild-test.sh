#!/bin/bash

set -euo pipefail

# Requires a parameter whose value is the os and version part of simulator
# runtime identifier returned by `xcrun simctl list runtimes`.
platform=$1

runtime="com.apple.CoreSimulator.SimRuntime.${platform}"

simcount=$(xcrun simctl list --json devices available | jq -re ".devices.\"${runtime}\" | length")

if [ $simcount -eq 0 ]; then
    echo "No simulators found for ${platform}. Creating simulator"
    if [ $platform == "iOS"* ]; then
        xcrun simctl create "iPhone 17 Pro Test Device" "com.apple.CoreSimulator.SimDeviceType.iPhone-17-Pro"
    elif [ $platform == "tvOS"* ]; then
        xcrun simctl create "Apple TV 4K at 1080p Test Device" "com.apple.CoreSimulator.SimDeviceType.Apple-TV-4K-1080p"
    elif [ $platform == "watchOS"* ]; then
        xcrun simctl create "Apple Watch Series 11 Test Device" "com.apple.CoreSimulator.SimDeviceType.Apple-Watch-Series-11-46mm"
    fi
fi

device_id=$(xcrun simctl list --json devices available | jq -re ".devices.\"${runtime}\" | last.udid")
device_name=$(xcrun simctl list --json devices available | jq -re ".devices.\"${runtime}\" | last.name")

echo "Runing on device: ${device_name}"

export NSUnbufferedIO=YES

xcodebuild \
    -scheme WordPressAPI-Package \
    -derivedDataPath DerivedData \
    -destination "id=${device_id}" \
    -skipPackagePluginValidation \
    test \
    | xcbeautify
