#!/bin/bash

set -euo pipefail

cd ./native/kotlin
./gradlew \
    :api:kotlin:prepareToPublishToS3 $(prepare_to_publish_to_s3_params) \
    :api:kotlin:publish

# Add meta-data for the published version so we can use it in subsequent steps
buildkite-agent meta-data set "PUBLISHED_WP_API_KOTLIN_VERSION" < ./api/kotlin/build/published-version.txt
