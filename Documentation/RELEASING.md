# Releasing wordpress-rs

A release is triggered manually after a version bump PR is merged to `trunk`.

## Steps

1. **Open a version bump PR** that only edits `CHANGELOG.md`:
   - Rename `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD`. Use today's
     UTC date. `X.Y.Z` is strict semver, optionally with an `-alpha.N` /
     `-beta.N` / `-rc.N` suffix for pre-releases.
   - Add a fresh empty `## [Unreleased]` section above the new version.
   - Apply the `Release` GitHub label.
   - PR title: `Release X.Y.Z`.

2. **Review and merge** the PR.

3. **Trigger the release** from the [Buildkite pipeline page][bk]: click
   **New Build**, leave the branch as `trunk`, and add an env var
   `NEW_VERSION=X.Y.Z` matching the version header you just added.

[bk]: https://buildkite.com/automattic/wordpress-rs
