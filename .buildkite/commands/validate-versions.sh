#!/bin/bash
# Checks that the crate version matches the newest release in CHANGELOG.md.
#
# The version lives in three places:
#   1. The newest "## [X.Y.Z]" header in CHANGELOG.md. This is what a release
#      publishes (see .buildkite/commands/detect-release.sh).
#   2. [workspace.package] version in the root Cargo.toml. Every crate inherits
#      it, and wp_api sends it in the default User-Agent.
#   3. The workspace crates' entries in Cargo.lock.
#
# A release PR bumps all three, so they must always agree. Needs no Rust
# toolchain: it only reads the files.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

errors=()

# 1. Newest released version in CHANGELOG.md. "## [Unreleased]" has no semver,
# so the pattern skips it.
changelog_version="$(
  grep -oE '^## \[[0-9]+\.[0-9]+\.[0-9]+[0-9A-Za-z.+-]*\]' CHANGELOG.md \
    | head -n 1 \
    | tr -d '#[] ' \
    || true
)"
if [[ -z "$changelog_version" ]]; then
  echo "No '## [X.Y.Z]' release header found in CHANGELOG.md." >&2
  exit 1
fi

# 2. [workspace.package] version in the root Cargo.toml.
workspace_version="$(
  awk '
    /^\[/ { in_section = ($0 == "[workspace.package]") ; next }
    in_section && /^version[[:space:]]*=/ {
      gsub(/^version[[:space:]]*=[[:space:]]*"|"[[:space:]]*$/, "")
      print
      exit
    }
  ' Cargo.toml
)"
if [[ -z "$workspace_version" ]]; then
  errors+=("The root \`Cargo.toml\` has no \`version\` under \`[workspace.package]\`.")
elif [[ "$workspace_version" != "$changelog_version" ]]; then
  errors+=("The root \`Cargo.toml\` has version \`$workspace_version\`, but the newest release in \`CHANGELOG.md\` is \`$changelog_version\`.")
fi

# Every crate must inherit the workspace version rather than set its own.
for manifest in */Cargo.toml; do
  if ! grep -Eq '^version\.workspace[[:space:]]*=[[:space:]]*true' "$manifest"; then
    errors+=("\`$manifest\` must use \`version.workspace = true\`.")
  fi
done

# 3. Cargo.lock. Workspace crates are the [[package]] entries with no
# "source" line; dependencies from crates.io or git always have one.
stale_lock_entries=()
while IFS=' ' read -r name version; do
  if [[ "$version" != "$changelog_version" ]]; then
    stale_lock_entries+=("$name $version")
  fi
done < <(
  awk '
    function flush() { if (name != "" && !has_source) print name, version }
    /^\[\[package\]\]/ { flush(); name = ""; version = ""; has_source = 0; next }
    /^name = /    { name = $3;    gsub(/"/, "", name) }
    /^version = / { version = $3; gsub(/"/, "", version) }
    /^source = /  { has_source = 1 }
    END { flush() }
  ' Cargo.lock
)
if (( ${#stale_lock_entries[@]} > 0 )); then
  errors+=("\`Cargo.lock\` has ${#stale_lock_entries[@]} workspace crate(s) not at \`$changelog_version\` (for example \`${stale_lock_entries[0]}\`). Run \`cargo update -w\`.")
fi

if (( ${#errors[@]} > 0 )); then
  echo "Version numbers are out of sync:" >&2
  printf '  - %s\n' "${errors[@]}" >&2
  echo >&2
  echo "See Documentation/RELEASING.md for where a release sets the version." >&2
  exit 1
fi

echo "CHANGELOG.md, Cargo.toml and Cargo.lock all use version $changelog_version."
