#!/bin/bash

set -euo pipefail

# Download pre-built Android JNI libraries from the cross-compile step
.buildkite/download-android-jni-libs.sh

# Retrieve data from previous steps
PUBLISHED_WP_API_KOTLIN_VERSION=$(buildkite-agent meta-data get "PUBLISHED_WP_API_KOTLIN_VERSION")

cd ./native/kotlin

# Calculate the version that would be published
VERSION=$(./gradlew -q -x cargoBuild :api:android:calculateVersionName $(prepare_to_publish_to_s3_params))

# Check if this version is already in S3
IS_PUBLISHED=$(./gradlew -q -x cargoBuild :api:android:isVersionPublishedToS3 \
    --published-group-id=rs.wordpress.api \
    --published-artifact-id=android \
    --version-name="$VERSION")

if [ "$IS_PUBLISHED" = "true" ]; then
    echo "Version '$VERSION' is already published to S3. Skipping."
else
    ./gradlew \
        -PwpApiKotlinVersion="$PUBLISHED_WP_API_KOTLIN_VERSION" \
        -x cargoBuild \
        :api:android:prepareToPublishToS3 $(prepare_to_publish_to_s3_params) \
        :api:android:publish
fi

# Add meta-data for the published version so we can use it in subsequent steps
buildkite-agent meta-data set "PUBLISHED_WP_API_ANDROID_VERSION" < ./api/android/build/published-version.txt
