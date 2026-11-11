#!/bin/bash

# Entry point for `run-in-swift-docker.sh`. Runs inside the container as root.

set -euo pipefail

# Without a user-namespace remap, files the build writes into the mounted checkout are owned by
# the host's root, which the agent can't clean up or overwrite in later jobs, so hand them back.
#
# The CI agents remap: root here is already the agent user on the host, the files are already
# its own, and changing their owner to a host uid isn't permitted. Detect that from the uid map,
# where container uid 0 maps to a host uid other than 0.
restore_ownership() {
    if awk '$1 == 0 && $2 == 0 { found = 1 } END { exit !found }' /proc/self/uid_map; then
        chown -R "$HOST_UID:$HOST_GID" /app
    fi
}
trap restore_ownership EXIT

echo "--- :linux: Installing build tools"
apt-get update -qq
apt-get install -y -qq --no-install-recommends ca-certificates curl gcc libc6-dev jq make >/dev/null

echo "--- :rust: Installing Rust"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y -q --default-toolchain none
# shellcheck source=/dev/null # Created by rustup above.
source "$HOME/.cargo/env"

"$@"
