#!/bin/bash -eu

if [ ! -f ./test_credentials ]; then
    echo "'test_credentials' file is not found. Make sure to run 'make test-server' before running this script."
    exit 1
fi

ADMIN_TOKEN=$(sed '3!d' ./test_credentials)

curl --user test@example.com:"$ADMIN_TOKEN" "http://localhost/wp-json$1"
