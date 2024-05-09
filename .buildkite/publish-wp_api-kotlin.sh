#!/bin/bash

set -euo pipefail

cd ./native/android
./gradlew \
    :wp_api_kotlin:prepareToPublishToS3 "$(prepare_to_publish_to_s3_params)" \
    :wp_api_kotlin:publish

# Add meta-data for the published version so we can use it in subsequent steps
buildkite-agent meta-data set "PUBLISHED_WP_API_KOTLIN_VERSION" < ./fluxc-annotations/build/published-version.txt
