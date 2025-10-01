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
	wp db check --skip-ssl || code=$?

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

wp plugin install gutenberg --activate

# Install custom must-use plugins for integration tests
mkdir -p wp-content/mu-plugins
cp -v /app/scripts/test-site-mu-plugins/*.php wp-content/mu-plugins/
cp -v /app/scripts/test-site-plugins/*.php wp-content/plugins/

wp plugin activate books-plugin

# We need an `author` user for some of the integration tests
wp user create test_author test_author@example.com --role=author

# Switch to `twentytwentyfour` which supports post templates
# This is used in `/posts` integration tests that updates the `template` field
wp theme activate twentytwentyfour

# Create menus for integration tests
# The mu-plugin register-menu-locations.php registers primary and footer menu locations
PRIMARY_MENU_LOCATION="primary"
FOOTER_MENU_LOCATION="footer"
PRIMARY_MENU_ID="$(wp menu create "Primary Menu" --porcelain)"
FOOTER_MENU_ID="$(wp menu create "Footer Menu" --porcelain)"

# Assign menus to their locations
wp menu location assign "$PRIMARY_MENU_ID" "$PRIMARY_MENU_LOCATION"
wp menu location assign "$FOOTER_MENU_ID" "$FOOTER_MENU_LOCATION"

wp comment trash 22
wp comment spam 23

create_post_revision() {
  local revision_number="$1"
  local post_id="$2"

  curl --silent --user "$ADMIN_USERNAME":"$ADMIN_PASSWORD" -H "Content-Type: application/json" -d "{\"content\":\"content_revision_$revision_number\", \"author\": $ADMIN_USER_ID}" "http://localhost/wp-json/wp/v2/posts/$post_id" > /dev/null
}

create_post_autosave() {
  local autosave_number="$1"
  local post_id="$2"

  curl --silent --user "$ADMIN_USERNAME":"$ADMIN_PASSWORD" -H "Content-Type: application/json" -d "{\"content\":\"content_autosave_$autosave_number\", \"author\": $ADMIN_USER_ID}" "http://localhost/wp-json/wp/v2/posts/$post_id/autosaves"
}

create_page_revision() {
  local revision_number="$1"
  local page_id="$2"

  curl --silent --user "$ADMIN_USERNAME":"$ADMIN_PASSWORD" -H "Content-Type: application/json" -d "{\"content\":\"content_revision_$revision_number\", \"author\": $ADMIN_USER_ID}" "http://localhost/wp-json/wp/v2/pages/$page_id" > /dev/null
}

create_page_autosave() {
  local autosave_number="$1"
  local page_id="$2"

  curl --silent --user "$ADMIN_USERNAME":"$ADMIN_PASSWORD" -H "Content-Type: application/json" -d "{\"content\":\"content_autosave_$autosave_number\", \"author\": $ADMIN_USER_ID}" "http://localhost/wp-json/wp/v2/pages/$page_id/autosaves"
}

create_nav_menu_item_autosave() {
  local autosave_number="$1"
  local nav_menu_item_id="$2"

  curl --silent --user "$ADMIN_USERNAME":"$ADMIN_PASSWORD" -H "Content-Type: application/json" -d "{\"title\":\"nav_menu_item_autosave_$autosave_number\", \"author\": $ADMIN_USER_ID}" "http://localhost/wp-json/wp/v2/menu-items/$nav_menu_item_id/autosaves"
}

create_test_credentials () {
  local SITE_URL
  local ADMIN_USERNAME
  local ADMIN_PASSWORD_UUID
  local ADMIN_PASSWORD
  local SUBSCRIBER_USERNAME
  local SUBSCRIBER_PASSWORD
  local SUBSCRIBER_PASSWORD_UUID
  local TRASHED_POST_ID
  local PASSWORD_PROTECTED_POST_ID
  local PASSWORD_PROTECTED_COMMENT_ID
  local PASSWORD_PROTECTED_COMMENT_AUTHOR
  local REVISIONED_POST_ID
  local AUTOSAVED_POST_ID
  local AUTOSAVE_ID_FOR_AUTOSAVED_POST_ID
  local FIRST_POST_DATE_GMT
  local WORDPRESS_VERSION
  local INTEGRATION_TEST_CUSTOM_TEMPLATE_ID
  SITE_URL="http://localhost"
  ADMIN_USERNAME="test@example.com"
  ADMIN_USER_ID="$(wp user get "$ADMIN_USERNAME" --field=ID)"
  ADMIN_PASSWORD="$(wp user application-password create test@example.com test --porcelain)"
  ADMIN_PASSWORD_UUID="$(wp user application-password list test@example.com --fields=uuid --format=csv | sed -n '2 p')"
  SUBSCRIBER_USERNAME="themedemos"
  SUBSCRIBER_PASSWORD="$(wp user application-password create themedemos test --porcelain)"
  SUBSCRIBER_PASSWORD_UUID="$(wp user application-password list themedemos --fields=uuid --format=csv | sed -n '2 p')"
  AUTHOR_USERNAME="test_author"
  AUTHOR_PASSWORD="$(wp user application-password create test_author test --porcelain)"

  PASSWORD_PROTECTED_POST_ID="$(wp post create --post_type=post --post_password=INTEGRATION_TEST --post_title=Password_Protected --porcelain)"
  TRASHED_POST_ID="$(wp post create --post_type=post --post_title=Trashed_Post --porcelain)"

  # Create test pages
  PASSWORD_PROTECTED_PAGE_ID="$(wp post create --post_type=page --post_password=INTEGRATION_TEST --post_title=Password_Protected_Page --porcelain)"
  TRASHED_PAGE_ID="$(wp post create --post_type=page --post_title=Trashed_Page --porcelain)"
  FIRST_PAGE_ID="$(wp post list --post_type=page --posts_per_page=1 --orderby=ID --order=ASC --format=ids)"

  PASSWORD_PROTECTED_COMMENT_AUTHOR="setup-test-site.sh"
  PASSWORD_PROTECTED_COMMENT_ID="$(wp comment create --comment_post_ID="$PASSWORD_PROTECTED_POST_ID" --comment_content="test_comment_for_password_protected_post" --comment_author="$PASSWORD_PROTECTED_COMMENT_AUTHOR" --porcelain)"

  FIRST_POST_DATE_GMT=$(wp post get 1 --fields=post_date_gmt --format=csv | sed -n '2 p' | cut -d ',' -f 2 | cut -d'"' -f 2)

  WORDPRESS_VERSION="$(wp core version)"

  INTEGRATION_TEST_CUSTOM_TEMPLATE_SLUG="integration_test_custom_template"

  # Trash the post and page
  wp post delete "$TRASHED_POST_ID"
  wp post delete "$TRASHED_PAGE_ID"

  echo "Creating a custom template for integration tests.."
  curl --silent --user "$ADMIN_USERNAME":"$ADMIN_PASSWORD" -H "Content-Type: application/json" -d '{"slug":"INTEGRATION_TEST_CUSTOM_TEMPLATE", "content": "Integration test custom template content"}' http://localhost/wp-json/wp/v2/templates > /dev/null
  INTEGRATION_TEST_CUSTOM_TEMPLATE_ID="twentytwentyfour//integration_test_custom_template"

  echo "Setting up a post with 10 revisions for integration tests.."
  REVISIONED_POST_ID="$(wp post create --post_type=post --post_title=Revisioned_POST_FOR_INTEGRATION_TESTS --porcelain)"
  # Create revisions
  for i in {1..10};
  do
    create_post_revision "$i" "$REVISIONED_POST_ID"
  done
  # Generating revisions don't return an id, but since we just created the `REVISIONED_POST_ID`, we can use it to calculate the revision id
  REVISION_ID_FOR_REVISIONED_POST_ID=$((REVISIONED_POST_ID + 1))

  echo "Setting up a post with autosave for integration tests.."
  # Autosaves require special setup: WordPress only creates separate autosave revisions when
  # the autosave is made by a different user than the post author. If the same user creates
  # an autosave, WordPress just updates the original post instead of creating a revision.
  # See: https://github.com/WordPress/WordPress/blob/c90da2b67323e36b0390a9efa95f2b4bbb3be31b/wp-includes/rest-api/endpoints/class-wp-rest-autosaves-controller.php#L235-L244
  # Create post as author user to enable proper autosave behavior
  AUTHOR_USER_ID="$(wp user get "$AUTHOR_USERNAME" --field=ID)"
  AUTOSAVED_POST_ID="$(wp post create --post_type=post --post_title=Autosaved_POST_FOR_INTEGRATION_TESTS --post_author="$AUTHOR_USER_ID" --porcelain)"
  # Create autosave as admin user (different from post author) and capture its ID
  AUTOSAVE_RESPONSE="$(create_post_autosave "1" "$AUTOSAVED_POST_ID")"
  AUTOSAVE_ID_FOR_AUTOSAVED_POST_ID="$(echo "$AUTOSAVE_RESPONSE" | jq -r '.id')"

  echo "Setting up a page with 10 revisions for integration tests.."
  REVISIONED_PAGE_ID="$(wp post create --post_type=page --post_title=Revisioned_PAGE_FOR_INTEGRATION_TESTS --porcelain)"
  # Create revisions
  for i in {1..10};
  do
    create_page_revision "$i" "$REVISIONED_PAGE_ID"
  done
  # Generating revisions don't return an id, but since we just created the `REVISIONED_PAGE_ID`, we can use it to calculate the revision id
  REVISION_ID_FOR_REVISIONED_PAGE_ID=$((REVISIONED_PAGE_ID + 1))

  echo "Setting up a page with autosave for integration tests.."
  # Create page as author user to enable proper autosave behavior (same requirement as posts)
  AUTOSAVED_PAGE_ID="$(wp post create --post_type=page --post_title=Autosaved_PAGE_FOR_INTEGRATION_TESTS --post_author="$AUTHOR_USER_ID" --porcelain)"
  # Create autosave as admin user (different from page author) and capture its ID
  AUTOSAVE_PAGE_RESPONSE="$(create_page_autosave "1" "$AUTOSAVED_PAGE_ID")"
  AUTOSAVE_ID_FOR_AUTOSAVED_PAGE_ID="$(echo "$AUTOSAVE_PAGE_RESPONSE" | jq -r '.id')"

  echo "Creating a nav menu item for integration tests.."
  NAV_MENU_ITEM_RESPONSE="$(curl --silent --user "$ADMIN_USERNAME":"$ADMIN_PASSWORD" -H "Content-Type: application/json" -d '{"title":"Integration Test Nav Menu Item","menus":179,"type":"custom","url":"https://example.com"}' http://localhost/wp-json/wp/v2/menu-items)"
  NAV_MENU_ITEM_ID="$(echo "$NAV_MENU_ITEM_RESPONSE" | jq -r '.id')"

  echo "Setting up a nav menu item with autosave for integration tests.."
  NAV_MENU_ITEM_AUTOSAVE_RESPONSE="$(create_nav_menu_item_autosave "1" "$NAV_MENU_ITEM_ID")"
  AUTOSAVE_ID_FOR_NAV_MENU_ITEM_ID="$(echo "$NAV_MENU_ITEM_AUTOSAVE_RESPONSE" | jq -r '.id')"

  rm -rf /app/test_credentials.json
  jo -p \
    site_url="$SITE_URL" \
    admin_username="$ADMIN_USERNAME" \
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
    password_protected_comment_id="$PASSWORD_PROTECTED_COMMENT_ID" \
    password_protected_comment_author="$PASSWORD_PROTECTED_COMMENT_AUTHOR" \
    trashed_post_id="$TRASHED_POST_ID" \
    first_post_date_gmt="$FIRST_POST_DATE_GMT" \
    wordpress_core_version="\"$WORDPRESS_VERSION\"" \
    integration_test_custom_template_id="$INTEGRATION_TEST_CUSTOM_TEMPLATE_ID" \
    revisioned_post_id="$REVISIONED_POST_ID" \
    revision_id_for_revisioned_post_id="$REVISION_ID_FOR_REVISIONED_POST_ID" \
    autosaved_post_id="$AUTOSAVED_POST_ID" \
    autosave_id_for_autosaved_post_id="$AUTOSAVE_ID_FOR_AUTOSAVED_POST_ID" \
    password_protected_page_id="$PASSWORD_PROTECTED_PAGE_ID" \
    password_protected_page_password="INTEGRATION_TEST" \
    password_protected_page_title="Password_Protected_Page" \
    trashed_page_id="$TRASHED_PAGE_ID" \
    first_page_id="$FIRST_PAGE_ID" \
    revisioned_page_id="$REVISIONED_PAGE_ID" \
    revision_id_for_revisioned_page_id="$REVISION_ID_FOR_REVISIONED_PAGE_ID" \
    autosaved_page_id="$AUTOSAVED_PAGE_ID" \
    autosave_id_for_autosaved_page_id="$AUTOSAVE_ID_FOR_AUTOSAVED_PAGE_ID" \
    primary_menu_location="$PRIMARY_MENU_LOCATION" \
    footer_menu_location="$FOOTER_MENU_LOCATION" \
    nav_menu_item_id="$NAV_MENU_ITEM_ID" \
    autosave_id_for_nav_menu_item_id="$AUTOSAVE_ID_FOR_NAV_MENU_ITEM_ID" \
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

wp db export --skip-ssl --add-drop-table wp-content/dump.sql
