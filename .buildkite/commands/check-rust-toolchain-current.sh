#!/bin/bash

# Warns when the Rust toolchain pinned in `rust-toolchain.toml` has fallen behind
# the current stable release. Dependabot can't bump the toolchain (it lives in a
# `rust-toolchain.toml` / `Makefile` string, not a package manifest), so this runs
# on a schedule to give us a deliberate nudge to bump it.
# See https://github.com/Automattic/wordpress-rs/issues/1436.
#
# Usage: check-rust-toolchain-current.sh [--notify-slack]
#   --notify-slack  Post to Slack (needs SLACK_WEBHOOK): a reminder when the pin is
#                   behind, or a low-urgency heads-up when the check can't run at all
#                   (so a persistently broken check doesn't just go silently green).
#                   Off by default so local `make` runs stay quiet.
#
# Exit codes:
#   0 - the pin is current, OR we couldn't determine the latest version. We skip
#       rather than fail on a transient network/parse issue; with --notify-slack a
#       skip also posts a heads-up so a persistent failure stays visible.
#   1 - the pin is behind the current stable release; time to bump it.

set -uo pipefail

NOTIFY_SLACK=false
for arg in "$@"; do
    case "$arg" in
        --notify-slack) NOTIFY_SLACK=true ;;
        *) echo "Unknown argument: $arg" >&2; exit 2 ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TOOLCHAIN_FILE="$REPO_ROOT/rust-toolchain.toml"

STABLE_MANIFEST_URL="https://static.rust-lang.org/dist/channel-rust-stable.toml"

pinned_version() {
    # Extract e.g. `1.97.0` from `channel = "1.97.0"`.
    grep -E '^[[:space:]]*channel[[:space:]]*=' "$TOOLCHAIN_FILE" \
        | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' \
        | head -1
}

latest_stable_version() {
    # The `[pkg.rust]` block holds the rustc version. We can't just grep the first
    # version in the manifest — `[pkg.cargo]` comes first and carries cargo's own
    # `0.(minor+1).0` version, which would be misread as the toolchain version.
    # -f: fail (empty output) on an HTTP error instead of parsing an error page.
    # -L: follow redirects so a future 301 doesn't masquerade as an empty manifest.
    curl -fsSL --max-time 30 "$STABLE_MANIFEST_URL" 2>/dev/null \
        | grep -A2 '^\[pkg\.rust\]' \
        | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' \
        | head -1
}

# Post a message to Slack. Best-effort, and gated on --notify-slack + SLACK_WEBHOOK
# so local `make` runs and un-provisioned environments stay quiet. Never fails the
# caller. Keep the message free of double quotes and backslashes — it is embedded
# verbatim into the JSON payload below.
post_slack() {
    # $1 = message text.
    if [ "$NOTIFY_SLACK" != true ]; then
        return
    fi
    if [ -z "${SLACK_WEBHOOK:-}" ]; then
        echo "SLACK_WEBHOOK not set; skipping Slack notification."
        return
    fi
    local payload
    payload="$(printf '{"channel":"#wordpress-rs","username":"wordpress-rs CI","icon_emoji":":rust:","text":"%s"}' "$1")"
    # --max-time so a stalled/blackholed webhook can't hang the nightly job.
    if curl -sS --max-time 15 -X POST -H 'Content-Type: application/json' --data "$payload" "$SLACK_WEBHOOK" >/dev/null; then
        echo "Posted message to Slack."
    else
        echo "Failed to post message to Slack."
    fi
}

# Skip the check (exit 0, so we never fail the build on a transient issue) while
# still making the skip observable: a *persistent* inability to run — a dead
# manifest URL, or a pin this script can't parse — should surface in Slack rather
# than silently going green every night. A rare transient blip emits one
# low-urgency line, which is exactly the dead-man's-switch signal we want.
skip_check() {
    # $1 = human-readable reason (a complete sentence, no double quotes/backslashes).
    echo "$1 Skipping toolchain currency check."
    post_slack "ℹ️ wordpress-rs Rust toolchain currency check could not run: $1 If this keeps happening, the check itself is likely broken. See https://github.com/Automattic/wordpress-rs/issues/1436"
    exit 0
}

if [ ! -f "$TOOLCHAIN_FILE" ]; then
    skip_check "Could not find $TOOLCHAIN_FILE."
fi

PINNED="$(pinned_version)"
if [ -z "$PINNED" ]; then
    skip_check "Could not parse an exact X.Y.Z toolchain version (e.g. 1.97.0) from $TOOLCHAIN_FILE; this check does not understand loose pins like 1.97 or named channels like stable."
fi

LATEST="$(latest_stable_version)"
if [ -z "$LATEST" ]; then
    skip_check "Could not fetch the latest stable Rust version from $STABLE_MANIFEST_URL."
fi

if [ "$PINNED" = "$LATEST" ]; then
    echo "Rust toolchain pin ($PINNED) is up to date with the latest stable release."
    exit 0
fi

# Only treat it as "behind" if the latest release really is newer, so a manually
# pinned pre-release (unusual) doesn't flap this check.
NEWEST="$(printf '%s\n%s\n' "$PINNED" "$LATEST" | sort -V | tail -1)"
if [ "$NEWEST" = "$PINNED" ]; then
    echo "Rust toolchain pin ($PINNED) is ahead of the latest stable release ($LATEST); nothing to do."
    exit 0
fi

cat <<EOF
Rust toolchain pin is behind the latest stable release.

  pinned: $PINNED
  latest: $LATEST

Bump all three in lockstep, then re-run \`make lint-rust\` to catch any newly
surfaced clippy lints:
  - rust-toolchain.toml  (channel = "$LATEST")
  - Makefile             (rust_stable_toolchain := $LATEST)
  - wp_rs_web/Dockerfile (FROM rust:$LATEST)
EOF
post_slack "⚠️ The pinned Rust toolchain ($PINNED) is behind the latest stable release ($LATEST). Bump rust-toolchain.toml, the Makefile, and wp_rs_web/Dockerfile in lockstep, then re-run 'make lint-rust'. See https://github.com/Automattic/wordpress-rs/issues/1436"
exit 1
