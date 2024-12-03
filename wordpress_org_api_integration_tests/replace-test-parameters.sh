#!/bin/bash

set -euo pipefail

save_plugin_slug() {
    url=$1
    curl -s "$url" | jq -r '.plugins[].slug' | xargs -I {} touch "$SLUG_DIR/{}"
}

OUTPUT_FILE="wordpress_org_api_integration_tests/tests/plugin_directory_parameters.rs"

echo "Prepare integration tests parameters"

API_URL="https://api.wordpress.org/plugins/info/1.2/"
PAGE_SIZE=200
PARALLEL_JOBS=20

current_page=1
response=$(curl -s "$API_URL?action=query_plugins&request%5Bpage%5D=$current_page&request%5Bper_page%5D=$PAGE_SIZE")
total_pages=$(echo "$response" | jq -r '.info.pages')
if [[ -z "$total_pages" || "$total_pages" -eq 0 ]]; then
    echo "Failed to fetch pagination info or no pages available."
    exit 1
fi
echo "Total pages to process: $total_pages"

query_plugins_api_urls=()
current_page=1
while [[ $current_page -le $total_pages ]]; do
    query_plugins_api_urls+=("$API_URL?action=query_plugins&request%5Bpage%5D=$current_page&request%5Bper_page%5D=$PAGE_SIZE")
    current_page=$((current_page + 1))
done

export -f save_plugin_slug
export API_URL
export SLUG_DIR="target/wordpress-org-plugin-directory/slugs"
echo "Saving plugin slugs to $SLUG_DIR"
rm -rf "$SLUG_DIR" && mkdir -p "$SLUG_DIR"
echo "${query_plugins_api_urls[@]}" | xargs -n 1 -P "$PARALLEL_JOBS" bash -c 'save_plugin_slug "$@"' _

slugs=()
while IFS='' read -r line; do slugs+=("$line"); done < <(ls -1 "$SLUG_DIR")

{
    echo '#![allow(unused_macros)]'
    echo ''
    echo '#[rstest_reuse::template]'
    echo '#[rstest::rstest]'
    for url in "${query_plugins_api_urls[@]}"; do
        echo "#[case(\"$url\")]"
    done
    echo 'pub fn query_plugins_api_url(#[case] url: &str) {}'
    echo ''

    midpoint=$((${#slugs[@]} / 2))
    first_half=("${slugs[@]:0:midpoint}")
    second_half=("${slugs[@]:midpoint}")

    echo '#[rstest_reuse::template]'
    echo '#[rstest::rstest]'
    for slug in "${first_half[@]}"; do
        echo "#[case(\"$slug\")]"
    done
    echo 'pub fn plugin_information_slug_1(#[case] slug: &str) {}'
    echo ''

    echo '#[rstest_reuse::template]'
    echo '#[rstest::rstest]'
    for slug in "${second_half[@]}"; do
        echo "#[case(\"$slug\")]"
    done
    echo 'pub fn plugin_information_slug_2(#[case] slug: &str) {}'
    echo ''
} > "$OUTPUT_FILE"
