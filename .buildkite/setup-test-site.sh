#!/bin/bash

set -e

# This script sets up a WordPress test site on the `wordpress` docker image. 
# You might wonder "why not do this work once, then just import the database for each run?"
# We do each step each time for each build because we're trying to get a "mint" condition site
# for each WordPress version – if there are issues with DB migrations, different default themes
# available, etc we don't want to have to deal with them.

## Install WordPress
wp core download --force

wp core install \
	--url=localhost \
	--title=my-test-site \
	--admin_user=test@example.com \
	--admin_email=test@example.com \
	--admin_password=strongpassword \
	--skip-email

## Ensure URLs work as expected
wp rewrite structure '/%year%/%monthnum%/%postname%/'

## Download the sample data (https://codex.wordpress.org/Theme_Unit_Test)
curl https://raw.githubusercontent.com/WPTT/theme-unit-test/master/themeunittestdata.wordpress.xml -C - -o /tmp/testdata.xml

## Then install the importer plugin
wp plugin install wordpress-importer --activate

## Then install the test data (https://developer.wordpress.org/cli/commands/import/)
wp import /tmp/testdata.xml --authors=create

## Then clean up the importer plugin
wp plugin delete wordpress-importer

## Create an Application password for the admin user, and store it where it can be used by the test suite
wp user application-password create test@example.com test --porcelain > /var/www/html/test_credentials
