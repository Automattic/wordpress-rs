wp_api_error_generic_error = Something went wrong.

site_error_message = {$error_message}

url_parsing_error = URL is invalid.

response_parsing_error = Response couldn't be parsed: {$reason}.

media_file_not_found = Media file not found at {$path}.

invalid_http_status_code = Invalid HTTP status code: {$status_code}.

request_execution_failed = Failed to send HTTP.

just = {$message}

invalid_ssl_error_certificate_not_valid_for_name = Invalid SSL certificate
invalid_ssl_error_generic_ssl_error = Unable to establish a secure connection to the server

non_existent_site_error = A server with the specified hostname could not be found.

http_authentication_required_error = The server at {$url} requires authentication. Please provide your username and password.
http_forbidden_error = The server at {$url} denied access to the requested resource. Please check your site's configuration.
http_timeout_error = The connection timed out
http_cancellation_error = The request was cancelled.

http_authentication_rejected_error = The server at {$url} rejected your credentials. Please provide a valid username and password.

http_server_error = Unable to connect to server: {$reason}. Please contact your server provider.

misconfigured_http_authentication_error = The server is sending invalid HTTP authentication information. Please check your site's HTTP authentication configuration.

misconfigured_rate_limit_error = The server is rate limiting requests in a way that will never succeed. Please check your site's rate limit configuration.

oauth_response_url_error_url_invalid = The site sent an invalid authentication response URL.
oauth_response_url_error_unsuccessful_login = Unsuccessful Login.

boolean_true_is_returned_when_string_is_expected = Expecting a `String` value for this field, but received the boolean `true` instead.

invalid_header_name_error = Invalid header name: {$header_name}.

invalid_header_value_error = Invalid header value: {$header_value}.

http_auth_method_missing_nonce = Missing nonce in HTTP authentication method.
http_auth_method_missing_qop = Missing QOP (Quality of Protection) in HTTP authentication method.
http_auth_method_missing_algorithm = Missing algorithm in HTTP authentication method.
http_auth_method_missing_opaque = Missing opaque value in HTTP authentication method.
http_auth_method_unknown = Unknown HTTP authentication method.

uniffi_serialization_error_serde = Serialization error: {$reason}.

uuid_parse_error_invalid_uuid = Invalid UUID string.
uuid_parse_error_not_version_4 = Not a version 4 UUID.

wordpress_org_api_client_error_request_encoding = Failed to encode request. Reason: {$reason}.

probably_not_wordpress_site = The site does not appear to be a WordPress site.
rest_api_disabled = The site's REST API is disabled. Please update your site settings to enable REST API.
xmlrpc_disabled_by_host = The site's XML-RPC is disabled. Please contact your hosting provider to solve this problem.
xmlrpc_disabled_by_plugin = The site's XML-RPC is disabled – the {$plugin} plugin might have disabled XML-RPC. Please visit {$support_url} to learn more.
xmlrpc_disabled_by_multiple_plugins = The site's XML-RPC is disabled – there are multiple installed plugins that might have disabled XML-RPC. Please disable them and try again.
xmlrpc_endpoint_not_found = The site's XML-RPC endpoint could not be found. Please check your site settings and try again.

application_password_blocked_by_plugin = Unable to login to {$url} – the {$plugin} plugin might have disabled Application Passwords. Please visit {$support_url} to learn more.
application_password_blocked_by_multiple_plugins = Unable to login to {$url} – there are multiple installed plugins that might have disabled Application Passwords. Please disable them and try again.
site_is_local_development_environment = This site is a local development environment. You'll need to enable application passwords to connect to it with the app.
application_passwords_disabled_for_http_site = Application Passwords is not enabled for this site – this is likely because we can't establish a secure connection to it. Please add an SSL certificate to this site and try again.
application_passwords_not_supported = The site does not support Application Passwords.

parse_api_root = Failed to parse the the site's WordPress REST API root response.
parse_api_root_failure_reason_server_fatal_error = Your server encountered an unrecoverable error and couldn't process the request. Please check your server error logs for details.
parse_api_root_failure_reason_wordfence_blocking_access = Wordfence is blocking access to the site's API. Please check your Wordfence configuration.

already_logged_in = You are already logged in as {$username}.

database_generic_message = There was a problem loading your data: {$reason}.
