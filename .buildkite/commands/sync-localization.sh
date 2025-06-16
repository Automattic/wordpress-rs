#!/bin/bash -euo pipefail

echo '--- :robot_face: Use bot for Git operations'
source use-bot-for-git

echo "--- :rubygems: Setting up Gems"
install_gems

echo "--- :globe_with_meridians: :arrow_up: Generate the source language PO file for GlotPress based on `wp_localization/localization/en-US/main.ftl`"
bundle exec fastlane generate_source_po_file commit_and_push_changes:true

echo "--- :globe_with_meridians: :arrow_down: Download and update translations from GlotPress and update the local Fluent files"
bundle exec fastlane download_translations commit_and_push_changes:true
