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
  exit 0
fi

cat <<'EOF' >&2

CHANGELOG.md was not updated in this PR.

Every PR should add an entry under the '## [Unreleased]' section of
CHANGELOG.md describing the change for our users. The format follows
https://keepachangelog.com/en/1.0.0/ — use one of:

  ### Added       for new features
  ### Changed     for changes in existing functionality (prefix '**BREAKING:**' if breaking)
  ### Deprecated  for soon-to-be removed features
  ### Removed     for now removed features
  ### Fixed       for any bug fixes
  ### Security    for vulnerability fixes

If the change genuinely has no user-visible impact (e.g. CI-only tweaks,
internal refactors), add a short entry under '### Changed' noting that.
EOF

exit 1
