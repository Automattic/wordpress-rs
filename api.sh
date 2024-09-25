#!/bin/bash -eu

if [ ! -f ./test_credentials.json ]; then
    echo "'test_credentials.json' file is not found. Make sure to run 'make test-server' before running this script."
    exit 1
fi

ADMIN_TOKEN=$(jq .admin_password test_credentials.json)

curl --user test@example.com:"$ADMIN_TOKEN" "http://localhost/wp-json$1"
