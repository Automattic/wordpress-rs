#!/bin/bash -eu

# A helper script to kick start the implementation of a new endpoint by creating files that
# are likely to be used.

NAME=$1

touch ./wp_api/src/"$NAME".rs
touch ./wp_api/src/request/endpoint/"$NAME"_endpoint.rs
touch ./wp_api_integration_tests/tests/test_"$NAME"_err.rs
touch ./wp_api_integration_tests/tests/test_"$NAME"_immut.rs
touch ./wp_api_integration_tests/tests/test_"$NAME"_mut.rs
