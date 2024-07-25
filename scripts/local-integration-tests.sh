#!/bin/bash

set -e

export WP_CONTENT_PATH=$(PWD)/.wordpress/wp-content 
export DB_HOSTNAME=localhost 
export API_URL=http://localhost

## Run the integration tests
cargo test --no-fail-fast
