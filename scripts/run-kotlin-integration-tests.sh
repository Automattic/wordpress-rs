#!/bin/bash -eu

# The project should be mounted to this location
cd /app/native/kotlin

./gradlew --init-script /app/scripts/reposilite-mirror.gradle.kts :api:kotlin:integrationTest
