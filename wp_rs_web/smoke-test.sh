#!/bin/bash

# Starts a wp_rs_web Docker image and checks that it serves its home page.
# Building the image doesn't prove it runs: a binary built against a newer glibc
# than the runtime stage has builds fine, then exits at launch.
#
# Usage: wp_rs_web/smoke-test.sh <image>

set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 <image>" >&2
    exit 2
fi

IMAGE="$1"
TIMEOUT_SECONDS=30

CONTAINER=$(docker run --detach --publish 127.0.0.1::8000 "$IMAGE")
trap 'docker rm --force "$CONTAINER" >/dev/null' EXIT

fail() {
    echo "$1" >&2
    echo "--- Container logs ---" >&2
    docker logs "$CONTAINER" >&2
    exit 1
}

for _ in $(seq 1 "$TIMEOUT_SECONDS"); do
    if [ "$(docker inspect --format '{{.State.Running}}' "$CONTAINER")" != "true" ]; then
        fail "The wp_rs_web container exited before it served a request."
    fi

    PORT=$(docker port "$CONTAINER" 8000/tcp | head -n 1 | sed 's/.*://')
    if [ -n "$PORT" ] && curl --silent --fail --output /dev/null "http://127.0.0.1:$PORT/"; then
        echo "The wp_rs_web container served GET / successfully."
        exit 0
    fi

    sleep 1
done

fail "The wp_rs_web container didn't serve GET / within $TIMEOUT_SECONDS seconds."
