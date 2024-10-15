#!/bin/bash

set -e

# This script sets up a WordPress test site on the `wordpress` docker image.
# You might wonder "why not do this work once, then just import the database for each run?"
# We do each step each time for each build because we're trying to get a "mint" condition site
# for each WordPress version – if there are issues with DB migrations, different default themes
# available, etc we don't want to have to deal with them.

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

ADMIN_USERNAME="test@example.com"
ADMIN_ACCOUNT_PASSWORD="strongpassword"

## Install WordPress
wp core install \
	--url=localhost \
	--title=my-test-site \
	--admin_user=$ADMIN_USERNAME \
	--admin_email=$ADMIN_USERNAME \
	--admin_password=$ADMIN_ACCOUNT_PASSWORD \
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

# We need an `author` user for some of the integration tests
wp user create test_author test_author@example.com --role=author

create_test_credentials () {
  local SITE_URL
  local ADMIN_PASSWORD_UUID
  local ADMIN_PASSWORD
  local SUBSCRIBER_USERNAME
  local SUBSCRIBER_PASSWORD
  local SUBSCRIBER_PASSWORD_UUID
  local TRASHED_POST_ID
  local PASSWORD_PROTECTED_POST_ID
  SITE_URL="http://localhost"
  ADMIN_PASSWORD="$(wp user application-password create $ADMIN_USERNAME test --porcelain)"
  ADMIN_PASSWORD_UUID="$(wp user application-password list $ADMIN_USERNAME --fields=uuid --format=csv | sed -n '2 p')"
  SUBSCRIBER_USERNAME="themedemos"
  SUBSCRIBER_PASSWORD="$(wp user application-password create themedemos test --porcelain)"
  SUBSCRIBER_PASSWORD_UUID="$(wp user application-password list themedemos --fields=uuid --format=csv | sed -n '2 p')"
  AUTHOR_USERNAME="test_author"
  AUTHOR_PASSWORD="$(wp user application-password create test_author test --porcelain)"

  PASSWORD_PROTECTED_POST_ID="$(wp post create --post_type=post --post_password=INTEGRATION_TEST --post_title=Password_Protected --porcelain)"
  TRASHED_POST_ID="$(wp post create --post_type=post --post_title=Trashed_Post --porcelain)"

  # Trash the post
  wp post delete "$TRASHED_POST_ID"

  rm -rf /app/test_credentials.json
  jo -p \
    site_url="$SITE_URL" \
    admin_username="$ADMIN_USERNAME" \
    admin_account_password="$ADMIN_ACCOUNT_PASSWORD" \
    admin_password="$ADMIN_PASSWORD" \
    admin_password_uuid="$ADMIN_PASSWORD_UUID" \
    subscriber_username="$SUBSCRIBER_USERNAME" \
    subscriber_password="$SUBSCRIBER_PASSWORD" \
    subscriber_password_uuid="$SUBSCRIBER_PASSWORD_UUID" \
    author_username="$AUTHOR_USERNAME" \
    author_password="$AUTHOR_PASSWORD" \
    password_protected_post_id="$PASSWORD_PROTECTED_POST_ID" \
    password_protected_post_password="INTEGRATION_TEST" \
    password_protected_post_title="Password_Protected" \
    trashed_post_id="$TRASHED_POST_ID" \
    > /app/test_credentials.json
}
create_test_credentials

## Used for integration tests
wp language core install en_CA
wp plugin install hello-dolly --activate
wp plugin install classic-editor

# Update the timezone, so that the `date` & `date_gmt` values will be different
# Otherwise, the integration tests might result in false positives
wp option update timezone_string "America/New_York"

cp -rp wp-content/plugins wp-content/plugins-backup

wp db export --add-drop-table wp-content/dump.sql
