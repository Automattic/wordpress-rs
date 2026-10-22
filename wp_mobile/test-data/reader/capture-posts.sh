#!/bin/bash
#
# Captures post HTML from the WordPress.com Reader API into `posts/`.
#
# The posts come from the Theme Unit Test site (https://wpthemetestdata.wordpress.com).
# They must come from the Reader API, not the site itself: the Reader API is what writes
# inline `style=""` attributes and embed `<script>` tags into the post HTML, and those are
# what the cleanup exists to handle.
#
# Re-run this to refresh the fixtures, then regenerate the snapshots:
#
#   ./wp_mobile/test-data/reader/capture-posts.sh
#   UPDATE_SNAPSHOTS=1 cargo test -p wp_mobile --lib reader::
#
# Review the snapshot diff before committing – a change there means the Reader API's output
# changed, not just our code.

set -euo pipefail

cd "$(dirname "$0")"

site="wpthemetestdata.wordpress.com"
site_id=$(curl -fsS "https://public-api.wordpress.com/rest/v1.1/sites/${site}?fields=ID" | jq -r .ID)

# Post ID, then fixture name.
posts=(
  "1730 block-common"
  "1732 block-formatting"
  "1734 block-layout"
  "1736 block-widgets"
  "1738 block-embeds"
  "1743 block-columns"
  "1745 block-cover"
  "1747 block-button"
  "1749 block-quote"
  "1755 block-image"
  "1178 markup-html-tags"
  "1177 markup-image-alignment"
  "1000 nested-lists"
  "1031 gallery-tiled"
  "587 audio"
  "1005 video-videopress"
  "1161 video-youtube"
  "1179 twitter-embeds"
)

for entry in "${posts[@]}"; do
  read -r post_id name <<< "$entry"
  curl -fsS "https://public-api.wordpress.com/rest/v1.2/read/sites/${site_id}/posts/${post_id}?fields=content" \
    | jq -j .content > "posts/${name}.html"
  echo "${post_id} → posts/${name}.html"
done
