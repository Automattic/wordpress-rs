#!/bin/bash

# Runs a command in the official Swift image, with the repo mounted at /app and the tools the
# Apple builds need: Rust (installed by `rustup` from `rust-toolchain.toml` and the Makefile),
# a host C compiler for build scripts, `jq` and `make`. Swift is needed because Swift binding
# generation runs `native/swift/Tools`.
#
# No Apple SDK is involved: see "Building on Linux" in Documentation/local-development.md.
#
# Usage: run-in-swift-docker.sh <command> [args...]

set -euo pipefail

SWIFT_IMAGE="public.ecr.aws/docker/library/swift:6.2"

docker run --rm \
    -v "$PWD":/app \
    -w /app \
    -e BUILDKITE \
    -e HOST_UID="$(id -u)" \
    -e HOST_GID="$(id -g)" \
    "$SWIFT_IMAGE" \
    .buildkite/commands/swift-docker-entrypoint.sh "$@"
