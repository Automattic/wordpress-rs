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

function annotate {
    local junit_file; junit_file=$1

    if ! command -v ruby &> /dev/null
    then
        echo "Installing ruby"
        # Presume we are running on a Debian-based Linux, because all macOS agents have ruby installed.
        apt-get install -y ruby
    fi

}

test -f ".build/spm-all-tests.xunit.xml" && annotate ".build/spm-all-tests.xunit.xml"
