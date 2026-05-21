#!/bin/bash

set -euo pipefail

# Deletes `pr-build/<n>` branches whose PR is closed (merged or rejected).
# Runs on trunk pushes so the just-merged PR's branch gets cleaned up
# immediately, and any orphans accumulated from prior failures get swept too.

if [[ "${BUILDKITE_BRANCH:-}" != "trunk" ]]; then
  echo "Not a trunk build (branch=${BUILDKITE_BRANCH:-unset}), skipping"
  exit 0
fi

if [[ -z "${GITHUB_TOKEN:-}" ]]; then
  echo "GITHUB_TOKEN not set, cannot reach GitHub API" >&2
  exit 1
fi

API="https://api.github.com/repos/automattic/wordpress-rs"

# Calls the GitHub API. Sets GH_STATUS and GH_BODY globals.
gh_request() {
  local response
  response=$(
    curl --silent --show-error \
      --request "$1" \
      --write-out $'\n%{http_code}' \
      --header "Authorization: Bearer ${GITHUB_TOKEN}" \
      --header "Accept: application/vnd.github+json" \
      --header "X-GitHub-Api-Version: 2022-11-28" \
      "${API}$2"
  )
  GH_STATUS=$(printf '%s' "$response" | tail -n1)
  GH_BODY=$(printf '%s' "$response" | sed '$d')
}

echo "--- :mag: Listing pr-build/* branches via GitHub API"
branches=()
page=1
while :; do
  gh_request GET "/branches?per_page=100&page=${page}"
  if [[ "$GH_STATUS" != "200" ]]; then
    echo "Failed to list branches (HTTP $GH_STATUS): $GH_BODY" >&2
    exit 1
  fi

  count=$(printf '%s' "$GH_BODY" | jq 'length')
  [[ "$count" -eq 0 ]] && break

  mapfile -t page_branches < <(
    printf '%s' "$GH_BODY" | jq -r '.[].name | select(test("^pr-build/[0-9]+$"))'
  )
  branches+=("${page_branches[@]}")

  [[ "$count" -lt 100 ]] && break
  page=$((page + 1))
done

echo "Found ${#branches[@]} pr-build branches"
[[ ${#branches[@]} -eq 0 ]] && exit 0

echo "--- :github: Checking PR state and deleting closed-PR branches"
deleted=0
kept=0
skipped=0
for branch in "${branches[@]}"; do
  pr_number="${branch#pr-build/}"

  gh_request GET "/pulls/${pr_number}"
  if [[ "$GH_STATUS" != "200" ]]; then
    echo "Skipping $branch (HTTP $GH_STATUS from /pulls/${pr_number})"
    skipped=$((skipped + 1))
    continue
  fi

  state=$(printf '%s' "$GH_BODY" | jq -r '.state')
  if [[ "$state" != "closed" ]]; then
    echo "Keeping $branch (PR #$pr_number is $state)"
    kept=$((kept + 1))
    continue
  fi

  gh_request DELETE "/git/refs/heads/${branch}"
  if [[ "$GH_STATUS" == "204" ]]; then
    echo "Deleted $branch (PR #$pr_number was closed)"
    deleted=$((deleted + 1))
  else
    echo "Failed to delete $branch (HTTP $GH_STATUS): $GH_BODY" >&2
    skipped=$((skipped + 1))
  fi
done

echo "--- :white_check_mark: Done: deleted=$deleted kept=$kept skipped=$skipped"
