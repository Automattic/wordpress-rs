#!/bin/bash

set -euo pipefail

GRADLE_TARGET=$1

echo "--- :rust: Installing Rust"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -v -y

source "$HOME/.cargo/env"

echo "--- :package: Installing Rust Target"

# Map Gradle target name to Rust target triple
case "$GRADLE_TARGET" in
    arm)    RUST_TARGET="armv7-linux-androideabi" ;;
    arm64)  RUST_TARGET="aarch64-linux-android" ;;
    x86)    RUST_TARGET="i686-linux-android" ;;
    x86_64) RUST_TARGET="x86_64-linux-android" ;;
    *)      echo "Unknown target: $GRADLE_TARGET"; exit 1 ;;
esac

# Only install the single target needed — no Apple targets or nightly toolchain
rustup target add "$RUST_TARGET"

echo "--- :package: Installing cargo-ndk"
cargo install cargo-ndk --locked

# This is a temporary step until we implement a more graceful way to handle missing credentials
printf "site_url\nadmin_username\nadmin_password\nadmin_password_uuid\nsubscriber_username\nsubscriber_password\nsubscriber_password_uuid\n" > test_credentials

echo "--- :rust: Cross-compiling for $GRADLE_TARGET"
./native/kotlin/gradlew -p native/kotlin -PcargoTarget="$GRADLE_TARGET" :api:android:cargoBuild
