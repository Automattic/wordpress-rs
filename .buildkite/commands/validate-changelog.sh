#!/bin/bash

set -euo pipefail

# Prints non-empty contents of every Unreleased section with whitespace normalized.
unreleased_section() {
  awk '
    /^## \[Unreleased\][[:space:]]*$/ { in_unreleased = 1; next }
    in_unreleased && /^## / { in_unreleased = 0 }
    in_unreleased {
      line = $0
      gsub(/[[:space:]]+/, " ", line)
      sub(/^ /, "", line)
      sub(/ $/, "", line)
      if (line != "") print line
    }
  '
}

pass() {
  echo "$1"
  # Delete any warning or failure comment left by a prior run.
  comment_on_pr --id changelog-check --if-exist delete "" || true
  exit 0
}

fail() {
  printf '\n%s\n\n' "$1" >&2
  comment_on_pr --id changelog-check "$1" || true
  exit 1
}

warn() {
  printf '\n%s\n\n' "$1" >&2
  # Keep the advisory visible in Buildkite if posting it to GitHub fails.
  buildkite-agent annotate "$1" --style warning --context changelog-check || true
  comment_on_pr --id changelog-check "$1" || true
  exit 0
}

# Only enforce on PR builds, where there is a base branch to compare against.
if [[ "${BUILDKITE_PULL_REQUEST:-false}" == "false" ]]; then
  echo "Not a PR build — skipping changelog check"
  exit 0
fi

# Dependabot-authored dependency bumps don't map to user-facing changelog entries.
if [[ "${BUILDKITE_BRANCH:-}" == dependabot/* ]]; then
  echo "Dependabot branch — skipping changelog check"
  exit 0
fi

# Localization sync PRs only update generated translation files, which don't map to changelog entries.
if [[ "${BUILDKITE_BRANCH:-}" == localization/* ]]; then
  echo "Localization branch — skipping changelog check"
  exit 0
fi

BASE_BRANCH="${BUILDKITE_PULL_REQUEST_BASE_BRANCH:-trunk}"

echo "--- :git: Fetching origin/${BASE_BRANCH}"
git fetch --no-tags origin "$BASE_BRANCH"

MERGE_BASE="$(git merge-base "origin/${BASE_BRANCH}" HEAD)"
CHANGED_FILES="$(git diff --name-only "$MERGE_BASE" HEAD)"

if [[ -z "$CHANGED_FILES" ]]; then
  pass "PR has no changed files — skipping changelog check"
fi

if [[ ! -f CHANGELOG.md ]]; then
  fail ':warning: **`CHANGELOG.md` is missing.**

Restore the file so releases and future pull requests can continue updating it.'
fi

if ! grep -Fxq "CHANGELOG.md" <<< "$CHANGED_FILES"; then
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

  fail "$FAILURE_MESSAGE"
fi

if ! grep -Eq '^## \[Unreleased\][[:space:]]*$' CHANGELOG.md; then
  fail ':warning: **`CHANGELOG.md` has no `## [Unreleased]` section.**

Restore the section; the release flow relies on it to collect the next release notes.'
fi

# Dedicated changelog PRs may intentionally correct an already released entry.
if [[ "$CHANGED_FILES" == "CHANGELOG.md" ]]; then
  pass "Only CHANGELOG.md changed — allowing a dedicated changelog correction"
fi

BASE_UNRELEASED=""
if git cat-file -e "${MERGE_BASE}:CHANGELOG.md" 2>/dev/null; then
  # Consume the whole file so git show cannot receive SIGPIPE under pipefail.
  BASE_UNRELEASED="$(git show "${MERGE_BASE}:CHANGELOG.md" | unreleased_section)"
fi
CURRENT_UNRELEASED="$(unreleased_section < CHANGELOG.md)"

if [[ "$CURRENT_UNRELEASED" != "$BASE_UNRELEASED" ]]; then
  pass "CHANGELOG.md changes the ## [Unreleased] section"
fi

WARNING_MESSAGE=$(cat <<'EOF'
:warning: **No substantive change under `## [Unreleased]` was detected.**

`CHANGELOG.md` was updated, but its `## [Unreleased]` section was not. If this PR changes code, please add or update an entry there. If the changelog update is intentionally correcting an older release, no action is needed.
EOF
)

warn "$WARNING_MESSAGE"
