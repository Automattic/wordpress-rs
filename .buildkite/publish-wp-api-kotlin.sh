#!/bin/bash

set -euo pipefail

echo "--- :arrow_down: Downloading pre-built host library"
buildkite-agent artifact download 'target/release/libwp_mobile.so' . --step "rust-build-host"

cd ./native/kotlin

# Calculate the version that would be published
VERSION=$(./gradlew -q -x cargoBuildLibraryRelease :api:kotlin:calculateVersionName $(prepare_to_publish_to_s3_params))

# Check if this version is already in S3
IS_PUBLISHED=$(./gradlew -q -x cargoBuildLibraryRelease :api:kotlin:isVersionPublishedToS3 \
    --published-group-id=rs.wordpress.api \
    --published-artifact-id=kotlin \
    --version-name="$VERSION")

if [ "$IS_PUBLISHED" = "true" ]; then
    echo "Version '$VERSION' is already published to S3. Skipping."
else
    ./gradlew \
        -x cargoBuildLibraryRelease \
        :api:kotlin:prepareToPublishToS3 $(prepare_to_publish_to_s3_params) \
        :api:kotlin:publish
fi

# Add meta-data for the published version so we can use it in subsequent steps
buildkite-agent meta-data set "PUBLISHED_WP_API_KOTLIN_VERSION" "$VERSION"
