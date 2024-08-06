#!/bin/bash

set -e

# This script sets up a WordPress test site on the `wordpress` docker image. 
# You might wonder "why not do this work once, then just import the database for each run?"
# We do each step each time for each build because we're trying to get a "mint" condition site
# for each WordPress version – if there are issues with DB migrations, different default themes
# available, etc we don't want to have to deal with them.

# Install wp-cli
curl -L https://github.com/wp-cli/wp-cli/releases/download/v2.6.0/wp-cli-2.6.0.phar --output /usr/bin/wp
chmod +x /usr/bin/wp

# Install `mysqlcheck` – needed for `wp db check`
apt update && apt install -y default-mysql-client less libssl-dev

# Install `rustup` – needed to run tests
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Create wpcli working directory (it can't be created by the `www-data` user`)
mkdir -p /var/www/.wp-cli
chown -R www-data:www-data /var/www/.wp-cli/

# Run all the commands below as `www-data` (because that's what WordPress uses itself, so there shouldn't
# be any weird permissions issues)
su -s /bin/bash www-data

## Wait for the DB to be ready before attempting install – Docker can do this for us, but we get way better
## diagnostic information from `wp db check`, whereas if `wp core install` fails it won't tell us about issues
## like incompatible SSL cipher suites (which is a problem in the WP 5.7 image when used with MySQL 8+)
tries=0
while true; do

	code=0
	wp db check || code=$?

	if [ $code == 0 ]; then
		echo 'Database Ready'
		break;
	fi

	if [ $tries -gt 5 ]; then
		echo 'Unable to connect to database'
		exit 1
	fi

	echo 'The database is not ready yet – waiting 5 seconds'
	sleep 5

	tries=$(( $tries + 1 ))
done

echo "--- :wordpress: Setting up WordPress"
wp core version --extra
wp --info

## Install WordPress
wp core install \
	--url=localhost \
	--title=my-test-site \
	--admin_user=test@example.com \
	--admin_email=test@example.com \
	--admin_password=strongpassword \
	--skip-email

## Ensure URLs work as expected
wp rewrite structure '/%year%/%monthnum%/%postname%/'

## Work around https://core.trac.wordpress.org/ticket/61638
mkdir -p wp-content/uploads/fonts

echo "--- :card_file_box: Importing Data"

## Download the sample data (https://codex.wordpress.org/Theme_Unit_Test)
curl https://raw.githubusercontent.com/WPTT/theme-unit-test/master/themeunittestdata.wordpress.xml -C - -o /tmp/testdata.xml

## Then install the importer plugin
wp plugin install wordpress-importer --activate

## Then install the test data (https://developer.wordpress.org/cli/commands/import/)
wp import /tmp/testdata.xml --authors=create

## Then clean up the importer plugin
wp plugin deactivate wordpress-importer
wp plugin delete wordpress-importer

rm -rf /app/test_credentials && touch /app/test_credentials
{
  printf "http://localhost\ntest@example.com\n"
  ## Create an Application password for the admin user, and store it where it can be used by the test suite
  wp user application-password create test@example.com test --porcelain 
  wp user application-password list test@example.com --fields=uuid --format=csv | sed -n '2 p'
  printf "themedemos\n"
  ## Create an Application password for a subscriber user, and store it where it can be used by the test suite
  wp user application-password create themedemos test --porcelain
  wp user application-password list themedemos --fields=uuid --format=csv | sed -n '2 p'
} >> /app/test_credentials

## Used for integration tests
wp plugin install hello-dolly --activate
wp plugin install classic-editor

cp -rp wp-content/plugins wp-content/plugins-backup

wp db export --add-drop-table wp-content/dump.sql

