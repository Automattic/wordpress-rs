# Releasing wordpress-rs

A release is published automatically when a version bump PR is merged to `trunk`.

## Steps

1. **Open a version bump PR** that only edits `CHANGELOG.md`:
   - Rename `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD`. Use today's
     UTC date. `X.Y.Z` is strict semver, optionally with an `-alpha.N` /
     `-beta.N` / `-rc.N` suffix for pre-releases.
   - Add a fresh empty `## [Unreleased]` section above the new version.
   - Apply the `Release` GitHub label.
   - PR title: `Release X.Y.Z`.
2. **Review and merge** the PR. The `trunk` build detects the new version and
   publishes the release automatically.

## Requirements

The release is detected from the version bump commit, so:

- PRs must be **squash-merged** to `trunk`.
- The header must be exactly `## [X.Y.Z] - YYYY-MM-DD`, with `## [Unreleased]`
  kept above it.

## Manual release

To re-run or force a release (for example after a failed publish), start a
[Buildkite][bk] build with **New Build**, branch `trunk`, and an env var
`NEW_VERSION=X.Y.Z`.

[bk]: https://buildkite.com/automattic/wordpress-rs
