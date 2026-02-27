#!/bin/bash
set -euo pipefail

WP_ENV_DIR="${1:?Usage: wp-env-setup.sh <wp-env-dir>}"
PORT="${2:?Usage: wp-env-setup.sh <wp-env-dir> <port>}"
WP_ENV_URL="http://localhost:${PORT}"

echo "Waiting for WordPress at ${WP_ENV_URL} to be ready..."

attempts=0
max_attempts=30
until curl -s -o /dev/null -w "%{http_code}" "${WP_ENV_URL}/?rest_route=/" | grep -q "200"; do
	attempts=$((attempts + 1))
	if [ "$attempts" -ge "$max_attempts" ]; then
		echo "WordPress did not become ready after ${max_attempts} attempts."
		exit 1
	fi
	echo "  Attempt ${attempts}/${max_attempts}..."
	sleep 2
done

echo "WordPress is ready."

# Enable pretty permalinks so /wp-json/ works
echo "Flushing rewrite rules..."
cd "${WP_ENV_DIR}"
npx wp-env run cli wp rewrite structure '/%postname%/' --hard

echo "wp-env setup complete. WordPress is running at ${WP_ENV_URL}"
