#!/bin/bash -eu

echo "--- :rust: Installing Rust"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -v -y

source "/Users/builder/.cargo/env"

CACHE_DATE=$(date +%V-%y)

echo "--- :package: Installing Rust Toolchains"
make setup-rust

BUILD_CACHE_KEY="wprs-xcframework-cache-${CACHE_DATE}"
HAS_CACHE=false

echo "--- :swift: Building xcframework"
echo "Using cache key: $BUILD_CACHE_KEY"
restore_cache "${BUILD_CACHE_KEY}"

# Use Apple Archiver because it's way faster
if [[ -f "wprs-build-cache" ]]; then
    HAS_CACHE=true

    echo "Extracting from build cache"
    aa extract -i wprs-build-cache.aar
fi

make xcframework
zip -r target/libwordpressFFI.xcframework.zip target/libwordpressFFI.xcframework

# Remove huge files that we can rebuild quickly.
# This brings the cache size from 49GB down to 33GB.
find "." -type f -name "libwp_api.*" -exec rm -v {} +
find "." -type f -name "libwordpress.a" -exec rm -v {} +

# if [ "$HAS_CACHE" = false ]; then
echo "Building Cache"
# Use Apple Archiver because it's way faster
aa archive -D target -o wprs-build-cache.aar
stat wprs-build-cache.aar
save_cache ./wprs-build-cache.aar "${BUILD_CACHE_KEY}" --force
rm wprs-build-cache.aar
# fi
