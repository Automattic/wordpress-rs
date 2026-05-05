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
  echo "GITHUB_TOKEN not set, cannot query PR state" >&2
  exit 1
fi

GITHUB_REPO="automattic/wordpress-rs"

echo '--- :robot_face: Use bot for Git operations'
source use-bot-for-git

echo "--- :mag: Listing pr-build/* branches on origin"
mapfile -t branches < <(
  git ls-remote --heads origin 'refs/heads/pr-build/*' \
    | awk '{print $2}' \
    | sed 's|^refs/heads/||'
)

echo "Found ${#branches[@]} pr-build branches"

if [[ ${#branches[@]} -eq 0 ]]; then
  exit 0
fi

echo "--- :github: Checking PR state for each branch"
to_delete=()
for branch in "${branches[@]}"; do
  pr_number="${branch#pr-build/}"
  if ! [[ "$pr_number" =~ ^[0-9]+$ ]]; then
    echo "Skipping $branch (unexpected suffix)"
    continue
  fi

  response=$(
    curl --silent --show-error \
      --write-out $'\n%{http_code}' \
      --header "Authorization: Bearer ${GITHUB_TOKEN}" \
      --header "Accept: application/vnd.github+json" \
      --header "X-GitHub-Api-Version: 2022-11-28" \
      "https://api.github.com/repos/${GITHUB_REPO}/pulls/${pr_number}"
  )
  http_code=$(printf '%s' "$response" | tail -n1)
  body=$(printf '%s' "$response" | sed '$d')

  if [[ "$http_code" != "200" ]]; then
    echo "Skipping $branch (HTTP $http_code from GitHub)"
    continue
  fi

  state=$(printf '%s' "$body" | jq -r '.state')

  if [[ "$state" == "closed" ]]; then
    echo "Marking $branch for deletion (PR #$pr_number is closed)"
    to_delete+=("$branch")
  else
    echo "Keeping $branch (PR #$pr_number is $state)"
  fi
done

if [[ ${#to_delete[@]} -eq 0 ]]; then
  echo "No closed PR branches to delete"
  exit 0
fi

echo "--- :wastebasket: Deleting ${#to_delete[@]} stale branches"
chunk_size=50
for ((i=0; i<${#to_delete[@]}; i+=chunk_size)); do
  chunk=("${to_delete[@]:i:chunk_size}")
  refspecs=()
  for branch in "${chunk[@]}"; do
    refspecs+=(":refs/heads/${branch}")
  done
  git push origin "${refspecs[@]}"
done
