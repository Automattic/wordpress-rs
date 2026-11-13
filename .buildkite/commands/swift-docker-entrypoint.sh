#!/bin/bash

# Entry point for `run-in-swift-docker.sh`. Runs inside the container as root.

set -euo pipefail

# Files the build writes into the mounted checkout belong to this container's root, and the host
# user must be able to delete them, or the next checkout fails.
#
# - Without a user-namespace remap (e.g. Docker Desktop), container root is host root, so hand
#   the files back to the host user.
# - The CI agents remap container root to the agent's own uid (Elastic CI Stack's
#   `bk-configure-docker.sh`), so the files already belong to the agent. Don't chown there:
#   inside the remap, `$HOST_UID` is a different host uid, and the agent couldn't delete the files.
restore_access() {
    if awk '$1 == 0 && $2 == 0 { found = 1 } END { exit !found }' /proc/self/uid_map; then
        chown -R "$HOST_UID:$HOST_GID" /app
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
