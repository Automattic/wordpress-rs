#!/bin/bash
# Rebuilds prebuilt/<rust target>/libwrapper.a for every Apple target wordpress-rs ships.
# Run on a Mac with Xcode whenever wrapper.c or a deployment target changes, and commit the result.
set -euo pipefail
cd "$(dirname "$0")"

# Deployment targets must match Package.swift; read them from the package so they can't drift.
platform_version() {
    (cd ../.. && swift package dump-package) | jq -r ".platforms[] | select(.platformName==\"$1\") | .version"
}
ios=$(platform_version ios)
macos=$(platform_version macos)
catalyst=$(platform_version maccatalyst)
tvos=$(platform_version tvos)
watchos=$(platform_version watchos)

# rust target | clang target | SDK
targets=(
    "aarch64-apple-darwin|arm64-apple-macos$macos|macosx"
    "x86_64-apple-darwin|x86_64-apple-macos$macos|macosx"
    "aarch64-apple-ios|arm64-apple-ios$ios|iphoneos"
    "x86_64-apple-ios|x86_64-apple-ios$ios-simulator|iphonesimulator"
    "aarch64-apple-ios-sim|arm64-apple-ios$ios-simulator|iphonesimulator"
    "aarch64-apple-ios-macabi|arm64-apple-ios$catalyst-macabi|macosx"
    "x86_64-apple-ios-macabi|x86_64-apple-ios$catalyst-macabi|macosx"
    "aarch64-apple-tvos|arm64-apple-tvos$tvos|appletvos"
    "aarch64-apple-tvos-sim|arm64-apple-tvos$tvos-simulator|appletvsimulator"
    "arm64_32-apple-watchos|arm64_32-apple-watchos$watchos|watchos"
    "x86_64-apple-watchos-sim|x86_64-apple-watchos$watchos-simulator|watchsimulator"
    "aarch64-apple-watchos-sim|arm64-apple-watchos$watchos-simulator|watchsimulator"
)

rm -rf prebuilt && mkdir prebuilt
for entry in "${targets[@]}"; do
    IFS='|' read -r rust_target clang_target sdk <<<"$entry"
    out="prebuilt/$rust_target"
    mkdir -p "$out"
    xcrun --sdk "$sdk" clang -target "$clang_target" -isysroot "$(xcrun --sdk "$sdk" --show-sdk-path)" \
        -O2 -fPIC -c wrapper.c -o "$out/wrapper.o"
    xcrun ar crs "$out/libwrapper.a" "$out/wrapper.o"
    rm "$out/wrapper.o"
    echo "built $out/libwrapper.a ($clang_target)"
done
shasum -a 256 wrapper.c | cut -d' ' -f1 > prebuilt/wrapper.c.sha256
