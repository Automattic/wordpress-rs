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

    if command -v ruby &> /dev/null
    then
        annotate_test_failures "$junit_file"
        return
    fi

    echo "Ruby not found, using docker to run the annotation utility"
    plugin_dir=$(echo "$PATH" | tr ':' '\n' | grep -F a8c-ci-toolkit)
    docker run --rm -it \
        -v "$(pwd):/app:ro" \
        -v "${plugin_dir}:${plugin_dir}:ro" \
        -w /app \
        --entrypoint /bin/sh \
        public.ecr.aws/docker/library/ruby:3.2-alpine \
        -c "export PATH=$plugin_dir:\$PATH; annotate_test_failures '$junit_file'"
}

test -f ".build/spm-all-tests.xunit.xml" && annotate ".build/spm-all-tests.xunit.xml"
