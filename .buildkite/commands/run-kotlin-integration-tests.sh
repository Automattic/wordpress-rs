#!/bin/bash -eu

test_results_dir="../native/kotlin/api/kotlin/build/test-results"
test_log_dir="${test_results_dir}/*/*.xml"
results_file="$test_results_dir/merged-test-results.xml"

# Give read/write permissions to `./` for all users
chmod -R a+rw ./

echo "--- :docker: Setting up Test Server"
make test-server

echo "--- 🧪 Running Kotlin Integration Tests"
make test-kotlin-integration

pwd

echo "--- 🚦 Report Tests Status"
merge_junit_reports -d "${test_log_dir%/*}" -o $results_file

echo "--- 🧪 Copying test logs for test collector"
mkdir buildkite-test-analytics
cp $results_file buildkite-test-analytics
