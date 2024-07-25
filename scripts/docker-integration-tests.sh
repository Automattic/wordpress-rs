#!/bin/bash

set -e

export WP_CONTENT_PATH=/app/.wordpress/wp-content
export DB_HOSTNAME=host.docker.internal 
export API_URL=http://host.docker.internal

## Run the integration tests
cargo test --no-fail-fast -p wp_api_integration_tests
