#!/bin/bash

set -euo pipefail

OUTPUT_DIR="target/wordpress-org-plugin-directory"

fetch_plugin_data() {
    slug=$1
    curl -s --output "$OUTPUT_DIR/$slug.json" "$API_URL?action=plugin_information&fields=icons%2Cbanners&slug=$slug"
}

download_from_wp_org() {
    echo "Downloading plugin data from WordPress.org."
    echo "(Run this script within sandbox to get a much faster download speed.)"

    rm -rf "$OUTPUT_DIR" && mkdir -p "$OUTPUT_DIR"

    API_URL="https://api.wordpress.org/plugins/info/1.2/"
    PARALLEL_JOBS=10

    current_page=1

    echo "Fetching the total number of pages..."
    response=$(curl -s "$API_URL?action=query_plugins&request%5Bpage%5D=$current_page&request%5Bper_page%5D=100")
    total_pages=$(echo "$response" | jq -r '.info.pages')
    if [[ -z "$total_pages" || "$total_pages" -eq 0 ]]; then
        echo "Failed to fetch pagination info or no pages available."
        exit 1
    fi

    echo "Total pages to process: $total_pages"

    export -f fetch_plugin_data
    export OUTPUT_DIR
    export API_URL

    while [[ $current_page -le $total_pages ]]; do
        echo "[$(date)] Processing page: $current_page"

        response=$(curl -s "$API_URL?action=query_plugins&request%5Bpage%5D=$current_page&request%5Bper_page%5D=100")
        plugin_slugs=$(echo "$response" | jq -r '.plugins[].slug')

        echo "$plugin_slugs" | xargs -n 1 -P "$PARALLEL_JOBS" bash -c 'fetch_plugin_data "$@"' _

        current_page=$((current_page + 1))
    done

    echo "All plugin data has been saved to the directory: $OUTPUT_DIR"
}

S3_URI="s3://a8c-ci-cache/wordpress-rs-wordpress-org-plugin-directory.tar.gz"

upload_to_s3() {
    echo "Uploading $(ls -1 $OUTPUT_DIR | wc -l) plugins in $OUTPUT_DIR to S3..."
    tar -czvf plugin-directory.tar.gz -C "$OUTPUT_DIR" .
    aws s3 cp plugin-directory.tar.gz "$S3_URI"
}

download_from_s3() {
    echo "Downloading plugin data from S3 cache..."
    aws s3 cp "$S3_URI" plugin-directory.tar.gz
    rm -rf "$OUTPUT_DIR" && mkdir -p "$OUTPUT_DIR"
    tar -xzvf plugin-directory.tar.gz -C "$OUTPUT_DIR"
}

${1:-download_from_wp_org}
