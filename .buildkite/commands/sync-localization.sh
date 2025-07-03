#!/bin/bash -euo pipefail

echo "--- :git: Checking out the current branch"
BRANCH="${BUILDKITE_BRANCH:-trunk}"
git checkout "${BRANCH}"
git pull origin "${BRANCH}"

echo '--- :robot_face: Use bot for Git operations'
source use-bot-for-git

echo "--- :rubygems: Setting up Gems"
install_gems

echo "--- :globe_with_meridians: :arrows_counterclockwise: Synchronizing localization files with GlotPress"
bundle exec fastlane sync_localization skip_confirm:true
