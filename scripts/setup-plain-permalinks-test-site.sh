#!/bin/bash

set -e

# Minimal setup for the plain-permalinks WordPress instance used by
# `test_login_plain_permalinks.rs`. We need just enough state to log in and
# fetch an authenticated REST endpoint:
#   - a running WordPress
#   - empty `permalink_structure` (so WP advertises the `?rest_route=/` form
#     in the API root Link header — this is the case the test exercises)
#   - an admin user with an application password
#
# Credentials are written to `test_credentials_plain_permalinks.json` at the
# repo root for the integration test to read at runtime.

su -s /bin/bash www-data

tries=0
while true; do
	code=0
	wp db check --skip-ssl || code=$?
	if [ $code == 0 ]; then
		echo 'Database Ready'
		break
	fi
	if [ $tries -gt 5 ]; then
		echo 'Unable to connect to database'
		exit 1
	fi
	echo 'The database is not ready yet – waiting 5 seconds'
	sleep 5
	tries=$(( $tries + 1 ))
done

echo "--- Setting up WordPress (plain permalinks)"

ADMIN_USERNAME="test@example.com"
ADMIN_ACCOUNT_PASSWORD="strongpassword"

# Configure WordPress at `http://localhost` (port 80). The integration test runs
# *inside* this container — matching the main `test-rust-integration` flow — so the
# site must be reachable on the container's own Apache port, not the host-mapped
# `:8081`. The `8081:80` mapping in docker-compose.plain-permalinks.yml exists only
# so a developer can browse the instance from the host while debugging.
wp core install \
	--url=http://localhost \
	--title=plain-permalinks-test-site \
	--admin_user="$ADMIN_USERNAME" \
	--admin_email="$ADMIN_USERNAME" \
	--admin_password="$ADMIN_ACCOUNT_PASSWORD" \
	--skip-email

# Default permalink_structure is empty, but be explicit so the intent is clear.
wp rewrite structure ''

ADMIN_PASSWORD="$(wp user application-password create "$ADMIN_USERNAME" test --porcelain)"

rm -f /app/test_credentials_plain_permalinks.json
jo -p \
	site_url="http://localhost" \
	admin_username="$ADMIN_USERNAME" \
	admin_password="$ADMIN_PASSWORD" \
	> /app/test_credentials_plain_permalinks.json
