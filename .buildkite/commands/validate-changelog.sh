#!/bin/bash

set -euo pipefail

# The top-level "- " entries under "## [Unreleased]", read from stdin and
# sorted so comm can diff two of these sets.
unreleased_entries() {
  awk '
    $0 == "## [Unreleased]" { in_unreleased = 1; next }
    in_unreleased && /^## / { exit }
    in_unreleased && /^- / { print }
  ' | LC_ALL=C sort -u
}

# Pass, deleting any failure comment from a prior run so a green run leaves no
# residue. The delete no-ops if the comment does not exist.
pass() {
  printf '%s\n' "$1"
  comment_on_pr --id changelog-check --if-exist delete "" || true
  exit 0
}

fail() {
  printf '\n%s\n\n' "$1" >&2
  comment_on_pr --id changelog-check "$1"
  exit 1
}

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

MERGE_BASE="$(git merge-base "origin/${BASE_BRANCH}" HEAD)"
CHANGELOG_DIFF="$(git diff "$MERGE_BASE" HEAD -- CHANGELOG.md)"

if ! grep -Fqx '## [Unreleased]' CHANGELOG.md; then
  fail ':warning: **`CHANGELOG.md` has no `## [Unreleased]` section.**

Restore the section and add this pull request’s changelog entry beneath it.'
fi

# Release PRs open a new versioned section instead of adding entries to
# Unreleased. Detect this from the diff, as detect-release.sh does, so it works
# regardless of the branch name. The here-string avoids a pipefail/SIGPIPE when
# grep -q exits after finding an early match in a large diff.
if grep -qE '^\+## \[[0-9]+\.[0-9]+\.[0-9]+' <<< "$CHANGELOG_DIFF"; then
  pass "CHANGELOG.md opens a new release section"
fi

# Compare only Unreleased entries so edits to an already released section
# cannot satisfy the check.
ADDED_ENTRIES="$(
  comm -13 \
    <(git show "${MERGE_BASE}:CHANGELOG.md" | unreleased_entries) \
    <(unreleased_entries < CHANGELOG.md)
)"

if [[ -n "$ADDED_ENTRIES" ]]; then
  pass "CHANGELOG.md adds an entry under ## [Unreleased]:
$ADDED_ENTRIES"
fi

FAILURE_MESSAGE=$(cat <<'EOF'
:warning: **No changelog entry was added under `## [Unreleased]`.**

Every PR should add an entry under the `## [Unreleased]` section of `CHANGELOG.md` describing the change for our users. Entries added under an already released version do not satisfy this check. The format follows [Keep a Changelog 1.0.0](https://keepachangelog.com/en/1.0.0/) — use one of:

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
