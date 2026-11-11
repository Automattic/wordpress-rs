#!/bin/bash

# Entry point for `run-in-swift-docker.sh`. Runs inside the container as root.

set -euo pipefail

# Files the build writes into the mounted checkout belong to this container's root, and the agent
# must be able to delete them, or the next job's checkout on that agent fails.
#
# - Without a user-namespace remap (e.g. Docker Desktop), container root is host root, so hand
#   the files back to the host user.
# - The CI agents remap: container root is an unprivileged host uid that is neither the agent nor
#   allowed to chown to it. Make everything this container created readable, writable and
#   searchable by everyone instead, so the agent can delete it.
restore_access() {
    if awk '$1 == 0 && $2 == 0 { found = 1 } END { exit !found }' /proc/self/uid_map; then
        chown -R "$HOST_UID:$HOST_GID" /app
    else
        find /app -xdev -user 0 -exec chmod a+rwX {} +
    fi
}
trap restore_access EXIT

echo "--- :linux: Installing build tools"
apt-get update -qq
apt-get install -y -qq --no-install-recommends ca-certificates curl gcc libc6-dev jq make >/dev/null

echo "--- :rust: Installing Rust"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y -q --default-toolchain none
# shellcheck source=/dev/null # Created by rustup above.
source "$HOME/.cargo/env"

"$@"
