#!/bin/bash

# Entry point for `run-in-swift-docker.sh`. Runs inside the container as root.

set -euo pipefail

# Files the build writes into the mounted checkout would otherwise be owned by root, which the
# agent can't clean up or overwrite in later jobs.
trap 'chown -R "$HOST_UID:$HOST_GID" /app' EXIT

echo "--- :linux: Installing build tools"
apt-get update -qq
apt-get install -y -qq --no-install-recommends ca-certificates curl gcc libc6-dev jq make >/dev/null

echo "--- :rust: Installing Rust"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y -q --default-toolchain none
# shellcheck source=/dev/null # Created by rustup above.
source "$HOME/.cargo/env"

"$@"
