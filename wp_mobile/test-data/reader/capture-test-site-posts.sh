#!/bin/bash
#
# Captures our own test posts from the WordPress.com Reader API into `test-site/`.
#
# Most of the posts are on https://jetpack.wpmt.co, a Jetpack site we run for testing, with its
# Shortcode Embeds module on. Each one covers something the Theme Unit Test and the anonymized corpus don't:
# shortcodes, embeds, every core and Jetpack block, image formats and URL forms, the cleanup's
# edge cases, other scripts and directions, and anchors. `test-site/sources/` holds what was
# published, and `test-site/sources/posts.json` lists each post with its title, excerpt, featured
# image and footnotes. They use images 142–149 and media 158–162 in the site's media library.
#
# The Reader API reads this site's posts from its feed, so it takes about 5 minutes for a new post
# to come through rendered. Until then, it returns the post's raw content. It can take much longer
# to notice an edit, so publish a new post rather than editing one.
#
# Re-run this to refresh the fixtures, then regenerate the snapshots:
#
#   ./wp_mobile/test-data/reader/capture-test-site-posts.sh
#   UPDATE_SNAPSHOTS=1 cargo test -p wp_mobile --lib reader::
#
# Two posts aren't captured:
#
# - 153 (`shortcodes-run-by-the-site-without-shortcode-embeds`) was captured once, before the
#   Shortcode Embeds module was turned on, to show those shortcodes left as text. Capturing it now
#   would give the same post as 175.
# - 169 is password-protected, so the Reader API answers 403 for it.
#
# Three more are on https://wpmt.co, a WordPress.com site we run for testing: a Getty Images embed
# as a block, as pasted embed code, and as a bare URL. WordPress.com renders its own sites' posts,
# so these keep the inline script Getty's embed is built from. Jetpack sites' posts lose it in the
# feed.

set -euo pipefail

cd "$(dirname "$0")"

capture() {
  local site=$1 site_type=$2 post_id=$3 name=$4
  local site_id
  site_id=$(curl -fsS "https://public-api.wordpress.com/rest/v1.1/sites/${site}?fields=ID" | jq -r .ID)
  curl -fsS "https://public-api.wordpress.com/rest/v1.2/read/sites/${site_id}/posts/${post_id}?meta=site,likes" \
    | jq --indent 1 --arg site "$site" --arg site_type "$site_type" --argjson post "$post_id" '{
        source: { site: $site, site_type: $site_type, post: $post },
        fields: { title, URL, content, excerpt, featured_image },
        features: []
      }' > "test-site/${name}.json"
  echo "${site} ${post_id} → test-site/${name}.json"
}

# Post ID, then fixture name.
jetpack_posts=(
  "152 shortcodes-left-for-wordpress-com"
  "154 shortcodes-from-plugins"
  "155 brackets-that-arent-shortcodes"
  "156 shortcodes-in-blocks"
  "163 embeds"
  "164 core-blocks"
  "165 jetpack-blocks"
  "166 images"
  "167 cleanup"
  "168 languages-and-anchors"
  "172 jetpack-shortcode-embeds"
  "175 shortcodes-run-by-the-site"
)

wordpress_com_posts=(
  "43 getty-block"
  "44 getty-code"
  "45 getty-url"
)

for entry in "${jetpack_posts[@]}"; do
  read -r post_id name <<< "$entry"
  capture jetpack.wpmt.co jetpack "$post_id" "$name"
done

for entry in "${wordpress_com_posts[@]}"; do
  read -r post_id name <<< "$entry"
  capture wpmt.co simple "$post_id" "$name"
done
