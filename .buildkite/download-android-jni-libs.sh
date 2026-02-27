#!/bin/bash

set -euo pipefail

echo "--- :arrow_down: Downloading pre-built Android JNI libraries"
buildkite-agent artifact download 'native/kotlin/api/android/build/rustJniLibs/android/**/*' .
