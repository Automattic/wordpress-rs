#!/bin/bash

set -euo pipefail

# Only enforce on PR builds — the check is about *new* entries being added
# alongside the change that ships them.
if [[ "${BUILDKITE_PULL_REQUEST:-false}" == "false" ]]; then
  echo "Not a PR build — skipping changelog check"
  exit 0
fi

# Dependabot-authored dependency bumps don't map to user-facing changelog entries.
if [[ "${BUILDKITE_BRANCH:-}" == dependabot/* ]]; then
  echo "Dependabot branch — skipping changelog check"
  exit 0
fi

BASE_BRANCH="${BUILDKITE_PULL_REQUEST_BASE_BRANCH:-trunk}"

echo "--- :git: Fetching origin/${BASE_BRANCH}"
git fetch --no-tags origin "$BASE_BRANCH"

if git diff --name-only "origin/${BASE_BRANCH}...HEAD" | grep -qx "CHANGELOG.md"; then
  echo "CHANGELOG.md was updated"
  # If a prior run posted a failure comment, clean it up so a green run
  # leaves no residue. No-ops if the comment doesn't exist.
  comment_on_pr --id changelog-check --if-exist delete "" || true
  exit 0
fi

FAILURE_MESSAGE=$(cat <<'EOF'
:warning: **`CHANGELOG.md` was not updated in this PR.**

Every PR should add an entry under the `## [Unreleased]` section of `CHANGELOG.md` describing the change for our users. The format follows [Keep a Changelog 1.0.0](https://keepachangelog.com/en/1.0.0/) — use one of:

- `### Added` — for new features
- `### Changed` — for changes in existing functionality (prefix `**BREAKING:**` if breaking)
- `### Deprecated` — for soon-to-be-removed features
- `### Removed` — for now-removed features
- `### Fixed` — for any bug fixes
- `### Security` — for vulnerability fixes

If the change genuinely has no user-visible impact (e.g. CI-only tweaks, internal refactors), add a short entry under `### Changed` noting that.
EOF
)

echo "" >&2
echo "$FAILURE_MESSAGE" >&2
echo "" >&2

comment_on_pr --id changelog-check "$FAILURE_MESSAGE"

exit 1
