#!/bin/bash

set -euo pipefail

# Prints the number of Unreleased entries and versioned release headings.
changelog_counts() {
  awk '
    $0 == "## [Unreleased]" { in_unreleased = 1; next }
    in_unreleased && /^## / { in_unreleased = 0 }
    in_unreleased && /^- / { entries++ }
    /^## \[[0-9]+\.[0-9]+\.[0-9]+/ { versions++ }
    END { print entries + 0, versions + 0 }
  '
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

read -r BASE_ENTRIES BASE_VERSIONS < <(
  git show "${MERGE_BASE}:CHANGELOG.md" | changelog_counts
)
read -r ENTRIES VERSIONS < <(changelog_counts < CHANGELOG.md)

# Normal PRs add an Unreleased entry; release PRs add a version heading.
if (( ENTRIES > BASE_ENTRIES || VERSIONS > BASE_VERSIONS )); then
  echo "CHANGELOG.md update is valid"
  # Delete any failure comment from a prior run so a green run leaves no residue.
  comment_on_pr --id changelog-check --if-exist delete "" || true
  exit 0
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

printf '\n%s\n\n' "$FAILURE_MESSAGE" >&2
comment_on_pr --id changelog-check "$FAILURE_MESSAGE"
exit 1
