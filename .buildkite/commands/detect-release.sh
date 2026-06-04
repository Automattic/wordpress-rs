#!/bin/bash
# Decides whether the current trunk build should publish a Swift release and, if
# so, uploads the release step. Two trigger paths, checked in this order:
#
#   1. Manual: NEW_VERSION is set (manual recovery / forced release). Kept
#      first and deliberately trivial so recovery stays usable even if the
#      changelog-diff logic below ever breaks.
#   2. Auto: this commit added a new "## [X.Y.Z]" header to CHANGELOG.md and no
#      tag for that version exists yet.
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Prints the version from the first *added* "## [X.Y.Z]" header in a CHANGELOG
# diff read on stdin, e.g. "+## [0.4.0] - 2026-05-29" -> "0.4.0". Prints nothing
# when no versioned header was added. The first grep keeps only added header
# lines; the second extracts the semver token, which also skips "[Unreleased]"
# (no semver) and the trailing date (no dots). grep exits 1 on no match (the
# normal "nothing to release" case), so the `|| true` keeps it off pipefail.
extract_added_version() {
  grep -E '^\+## \[' \
    | grep -oE '[0-9]+\.[0-9]+\.[0-9]+[0-9A-Za-z.+-]*' \
    | head -n 1 \
    || true
}

if [ -n "${NEW_VERSION:-}" ]; then
  version="$NEW_VERSION"
  echo "--- :sos: Manual release requested: $version"
else
  commit="${BUILDKITE_COMMIT:-HEAD}"

  # On a shallow CI clone the first parent may be absent; fetch one more level so
  # the diff has a base to compare against. ^1 is the first parent, which for a
  # squash commit is the previous trunk tip and for a merge commit is the trunk
  # side, so the diff equals the merged PR's net changes either way. Deepen via
  # the branch ref (always fetchable) rather than the raw SHA, which not every
  # git server allows fetching directly.
  if ! git rev-parse --verify --quiet "${commit}^1" >/dev/null; then
    git fetch --no-tags --deepen=1 origin "${BUILDKITE_BRANCH:-trunk}"
  fi
  version="$(git diff "${commit}^1" "${commit}" -- CHANGELOG.md | extract_added_version)"
  if [ -z "$version" ]; then
    echo "No new CHANGELOG version added by $commit; nothing to release."
    exit 0
  fi
  echo "--- :mag: Detected new release version: $version"

  # Auto path only: a successful prior release leaves the tag, so skip re-offering.
  # (A manual run intentionally bypasses this to push through a partial failure.)
  if git ls-remote --tags --exit-code origin "refs/tags/$version" >/dev/null 2>&1; then
    echo "Tag $version already exists; nothing to release."
    exit 0
  fi
fi

# Both trigger paths feed $version into a tag and a release command, so reject
# anything that is not a semver (e.g. a typo'd NEW_VERSION) before it reaches
# them. The auto path's extraction already enforces this shape, but a manual
# NEW_VERSION is taken verbatim, so validate here to cover both. Pattern is
# X.Y.Z with an optional pre-release/build suffix.
if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([0-9A-Za-z.+-]*)$ ]]; then
  echo "Version '$version' is not a valid semantic version; aborting." >&2
  exit 1
fi

# shared-pipeline-vars defines CI_TOOLKIT (and IMAGE_ID) for interpolation below;
# it is also sourced before the top-level pipeline upload.
# shellcheck source=.buildkite/shared-pipeline-vars
source "$REPO_ROOT/.buildkite/shared-pipeline-vars"

echo "--- :pipeline: Uploading release step for $version"
cat <<YAML | buildkite-agent pipeline upload
steps:
  - label: ":rocket: Publish release $version"
    command: .buildkite/release.sh "$version"
    depends_on: "swift"
    agents:
      queue: mac
    plugins: [$CI_TOOLKIT]
    notify:
      - slack:
          channels:
            - "#wordpress-rs"
          message: "Release $version published."
        if: build.state == "passed"
      - slack:
          channels:
            - "#wordpress-rs"
          message: "Release $version failed."
        if: build.state == "failed"
YAML
