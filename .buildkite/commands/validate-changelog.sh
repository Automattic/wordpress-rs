#!/bin/bash

set -euo pipefail

REPO="Automattic/wordpress-rs"
# HTML-comment marker lets us find & update our own comment on re-runs
# instead of piling up a new one each failed build.
COMMENT_MARKER="<!-- changelog-check -->"

# Only enforce on PR builds — the check is about *new* entries being added
# alongside the change that ships them.
if [[ "${BUILDKITE_PULL_REQUEST:-false}" == "false" ]]; then
  echo "Not a PR build — skipping changelog check"
  exit 0
fi

PR_NUMBER="$BUILDKITE_PULL_REQUEST"

# Dependabot-authored dependency bumps don't map to user-facing changelog entries.
if [[ "${BUILDKITE_BRANCH:-}" == dependabot/* ]]; then
  echo "Dependabot branch — skipping changelog check"
  exit 0
fi

find_existing_comment_id() {
  [[ -z "${GITHUB_TOKEN:-}" ]] && return 0
  curl -sS \
    -H "Authorization: Bearer ${GITHUB_TOKEN}" \
    -H "Accept: application/vnd.github+json" \
    "https://api.github.com/repos/${REPO}/issues/${PR_NUMBER}/comments?per_page=100" \
    | jq -r --arg m "$COMMENT_MARKER" 'first(.[] | select(.body // "" | contains($m)) | .id) // empty'
}

post_or_update_pr_comment() {
  local body="$1"
  if [[ -z "${GITHUB_TOKEN:-}" ]]; then
    echo "GITHUB_TOKEN not set — skipping PR comment" >&2
    return 0
  fi

  local full_body
  full_body="${COMMENT_MARKER}"$'\n\n'"${body}"
  local payload
  payload=$(jq -n --arg body "$full_body" '{body: $body}')

  local existing_id
  existing_id=$(find_existing_comment_id)

  if [[ -n "$existing_id" ]]; then
    curl -sS -X PATCH \
      -H "Authorization: Bearer ${GITHUB_TOKEN}" \
      -H "Accept: application/vnd.github+json" \
      "https://api.github.com/repos/${REPO}/issues/comments/${existing_id}" \
      -d "$payload" > /dev/null
    echo "Updated existing PR comment (id=${existing_id})"
  else
    curl -sS -X POST \
      -H "Authorization: Bearer ${GITHUB_TOKEN}" \
      -H "Accept: application/vnd.github+json" \
      "https://api.github.com/repos/${REPO}/issues/${PR_NUMBER}/comments" \
      -d "$payload" > /dev/null
    echo "Posted PR comment"
  fi
}

delete_pr_comment_if_exists() {
  [[ -z "${GITHUB_TOKEN:-}" ]] && return 0

  local existing_id
  existing_id=$(find_existing_comment_id)
  if [[ -n "$existing_id" ]]; then
    curl -sS -X DELETE \
      -H "Authorization: Bearer ${GITHUB_TOKEN}" \
      -H "Accept: application/vnd.github+json" \
      "https://api.github.com/repos/${REPO}/issues/comments/${existing_id}" > /dev/null
    echo "Deleted stale changelog-check PR comment (id=${existing_id})"
  fi
}

BASE_BRANCH="${BUILDKITE_PULL_REQUEST_BASE_BRANCH:-trunk}"

echo "--- :git: Fetching origin/${BASE_BRANCH}"
git fetch --no-tags origin "$BASE_BRANCH"

if git diff --name-only "origin/${BASE_BRANCH}...HEAD" | grep -qx "CHANGELOG.md"; then
  echo "CHANGELOG.md was updated"
  delete_pr_comment_if_exists
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

post_or_update_pr_comment "$FAILURE_MESSAGE"

exit 1
