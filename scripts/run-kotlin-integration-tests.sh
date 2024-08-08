#!/bin/bash -eu

cd ./native/kotlin
./gradlew :api:kotlin:integrationTest
