#!/bin/bash

if ! command -v annotate_test_failures &> /dev/null
then
    # Require the annotate_test_failures utility from the a8c-ci-toolkit plugin
    exit 0
fi

[[ -z "${IS_VM_HOST:-}" ]] || {
    # No need to run on macOS hosts
    exit 0
}

echo "--- :buildkite: Annotate test failures"

test -f ".build/spm-all-tests.xunit.xml" && annotate_test_failures ".build/spm-all-tests.xunit.xml"
