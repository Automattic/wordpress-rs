#!/bin/bash

set -euo pipefail

# Retrieve data from previous steps
PUBLISHED_WP_API_BINDINGS_VERSION=$(buildkite-agent meta-data get "PUBLISHED_WP_API_BINDINGS_VERSION")

cd ./native/kotlin
./gradlew \
    -PwpApiBindingsVersion="$PUBLISHED_WP_API_BINDINGS_VERSION" \
    :api:kotlin:prepareToPublishToS3 $(prepare_to_publish_to_s3_params) \
    :api:kotlin:publish

# Add meta-data for the published version so we can use it in subsequent steps
buildkite-agent meta-data set "PUBLISHED_WP_API_KOTLIN_VERSION" < ./api/kotlin/build/published-version.txt
