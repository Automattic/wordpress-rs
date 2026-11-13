#!/bin/bash

# Assembles the (unsigned) xcframework from the libraries the `apple-build` steps built. Runs on a
# Linux agent; signing and packaging happen on a Mac in `sign-xcframework.sh`.

set -euo pipefail

if [[ "${INSIDE_SWIFT_DOCKER:-}" != "true" ]]; then
    echo "--- :arrow_down: Downloading pre-built Apple libraries"
    buildkite-agent artifact download 'target/*/release/libwp_mobile.a' .
    buildkite-agent artifact download 'target/*/release/swift-bindings/**' .

    .buildkite/commands/run-in-swift-docker.sh env INSIDE_SWIFT_DOCKER=true "$0"

    echo "--- :package: Archiving XCFramework"
    tar -czf target/libwordpressFFI.xcframework.tar.gz -C target libwordpressFFI.xcframework
    exit 0
fi

# `llvm-lipo` reads the objects inside each library, so it must be at least as new as the LLVM
# rustc built them with; Ubuntu's own LLVM packages lag behind. Install the matching major version
# from apt.llvm.org. The pinned nightly (tvOS and watchOS) must not use a newer LLVM than stable.
echo "--- :linux: Installing llvm-lipo"
llvm_major=$(rustc -vV | sed -n 's/^LLVM version: \([0-9]*\)\..*/\1/p')
if [[ -z "$llvm_major" ]]; then
    echo "Couldn't read the LLVM version from 'rustc -vV'" >&2
    exit 1
fi
# shellcheck source=/dev/null # Provided by the image.
codename=$(. /etc/os-release && echo "$VERSION_CODENAME")
curl -fsSL https://apt.llvm.org/llvm-snapshot.gpg.key -o /etc/apt/trusted.gpg.d/apt.llvm.org.asc
echo "deb http://apt.llvm.org/${codename}/ llvm-toolchain-${codename}-${llvm_major} main" \
    > /etc/apt/sources.list.d/llvm.list
apt-get update -qq
apt-get install -y -qq --no-install-recommends "llvm-${llvm_major}" >/dev/null
export LIPO="llvm-lipo-${llvm_major}"

echo "--- :swift: Assembling XCFramework"
make xcframework-assemble
