#!/bin/bash -eu

# The project should be mounted to this location
cd /app/native/kotlin

./gradlew :api:kotlin:integrationTest
