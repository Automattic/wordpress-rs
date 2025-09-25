<?php

/*
 * Plugin Name: Custom REST URL Prefix
 */

// Changes the REST URL prefix to `api` instead of `wp-json`. Our test suite validates that this is supported.
add_filter( 'rest_url_prefix', function () {
	return 'api';
} );
