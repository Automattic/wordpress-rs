#!/bin/bash

set -euo pipefail

# Retrieve data from previous steps
PUBLISHED_WP_API_KOTLIN_VERSION=$(buildkite-agent meta-data get "PUBLISHED_WP_API_KOTLIN_VERSION")

cd ./native/kotlin
./gradlew \
    -PwpApiKotlinVersion="$PUBLISHED_WP_API_KOTLIN_VERSION" \
    :api:android:prepareToPublishToS3 $(prepare_to_publish_to_s3_params) \
    :api:android:publish
