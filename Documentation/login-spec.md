# WordPress Login Specification

This document specifies the requirements and behaviors for WordPress.org site login functionality. The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in RFC 2119.

# 1: Standard SSL-Enabled WordPress Installation

Requirements:
1. The site MUST be a stock WordPress Installation without modifications.
2. The site MUST use SSL.
3. The site MUST NOT have a CDN or caching that prevents the `Link` header from being sent.

Expected Signals:
1. The `Link` header MUST be present
2. The `authorization_url` MUST be present in the JSON API root

Behavior:
1. The system MUST complete login successfully.
2. The system MUST NOT display any warnings.

Reference Implementation: https://vanilla.wpmt.co

# 2: Local Development Environment

Requirements:
1. The site MUST be a stock WordPress Installation.
2. The site MAY have an invalid SSL certificate.
3. The `WP_ENVIRONMENT_TYPE` MUST NOT be set to "production"

Expected Signals:
1. The `Link` header MUST be present
2. The `authorization_url` MUST NOT be present in the JSON API root
3. The `has_https` check MUST fail
4. The URL MUST match one of:
   - An IP address
   - A domain ending in `.dev`
   - A domain ending in `.local`
   - The hostname `localhost`

Behavior:
1. The system MUST reject the login attempt
2. The system MUST display the error: "This site is a local development environment. You'll need to enable application passwords to connect to it with the app."

Reference Implementation: http://localhost

# 3: WordPress Admin URL Entry

Requirements:
1. The site MUST be a stock WordPress Installation without modifications.
2. The site MUST use SSL.
3. The site MUST NOT have a CDN or caching that prevents the `Link` header.
4. The URL entered MUST contain /wp-login.php or /wp-admin.

Expected Signals:
1. The initial URL request MUST fail to find the API root
2. The system MUST attempt a "fallback" request by dropping everything after the WordPress root
3. For the fallback request:
   - The `Link` header MUST be present
   - The `authorization_url` MUST be present in the JSON API root

Behavior:
1. The system MUST complete login successfully.
2. The system MUST NOT display any warnings.

Reference Implementation: https://vanilla.wpmt.co/wp-login.php

# 4: HTTP Protocol with HTTPS Support

Requirements:
1. The site MUST be a stock WordPress Installation.
2. The site MUST support SSL.
3. The site MUST respond to HTTP requests without redirects.
4. The URL entered MUST use the HTTP protocol.

Expected Signals:
1. The `Link` header MUST be present
2. The `authorization_url` MUST NOT be present in the JSON API root
3. The `has_https` check MUST fail
4. The system MUST attempt an HTTPS connection

Behavior:
1. The system MUST complete login successfully using HTTPS.
2. The system SHOULD display a warning about using HTTPS instead.

Reference Implementation: http://optional-https.wpmt.co

# 5: HTTP-Only Site Without Application Password Support

Requirements:
1. The site MUST be a stock WordPress Installation.
2. The site MUST NOT have a valid SSL certificate.
3. The site MUST NOT have any override to enable Application Passwords.

Expected Signals:
1. The `Link` header MUST be present
2. The `authorization_url` MUST NOT be present in the JSON API root
3. The `has_https` check MUST fail

Behavior:
1. The system MUST reject the login attempt
2. The system MUST display the error: "Unable to securely connect to `${DOMAIN}` – make sure it's using SSL"

Reference Implementation: http://optional-https.wpmt.co

# 6: HTTP-Only Site with Application Password Override

Requirements:
1. The site MUST be a stock WordPress Installation.
2. The site MUST NOT have a valid SSL certificate.
3. The site MUST explicitly enable Application Passwords over HTTP.

Expected Signals:
1. The `Link` header MUST be present
2. The `authorization_url` MUST be present in the JSON API root
3. The `has_https` check MUST fail

Behavior:
1. The system MUST complete login successfully
2. The system MUST display a warning about the lack of HTTPS security

Reference Implementation: http://no-https.wpmt.co

# 7: CDN-Cached Site

Requirements:
1. The site MUST be a stock WordPress Installation.
2. The site MUST use SSL.
3. The site MUST use a CDN that strips the `Link` header.

Expected Signals:
1. The `Link` header MUST NOT be present
2. The HTML body MUST contain a valid `<link rel="https://api.w.org/" href="[...]" />` tag

Behavior:
1. The system MUST complete login successfully using the API URL from the HTML tag
2. The system SHOULD NOT display any warnings

Reference Implementation: https://aggressive-caching.wpmt.co

# 8: Site with Disabled Application Passwords

Requirements:
1. The site MUST have a plugin that disables Application Passwords
2. The site MUST use SSL

Expected Signals:
1. The `Link` header MUST be present
2. The `authorization_url` MUST be present in the JSON API root
3. The WP-JSON root MUST contain identifiable plugin namespaces

Behavior:
1. The system MUST reject the login attempt
2. The system MUST display the error: "Unable to login to `${DOMAIN}` – the `${PLUGIN_NAME}` plugin might have disabled Application Passwords"

Reference Implementation: https://wordfence.wpmt.co

# 9: Non-WordPress Site

Requirements:
1. The site MUST NOT be a WordPress site
2. The site MUST use SSL

Expected Signals:
1. The `Link` header MUST NOT be present
2. The HTML body MUST NOT contain a WordPress API link tag
3. The `is_wordpress` check MUST return `false`

Behavior:
1. The system MUST reject the login attempt
2. The system MUST display the error: "Unable to login to `${DOMAIN}`. Please double-check that this is a WordPress site"

Reference Implementation: https://google.com

# 10: WordPress in Subdirectory with Link Header

Requirements:
1. WordPress MUST be installed in a subdirectory
2. The site MUST use SSL
3. The root domain MUST emit a `Link` header pointing to the WordPress installation

Expected Signals:
1. The `Link` header MUST be present and point to the correct subdirectory
2. The system MUST follow the subdirectory path for authentication

Behavior:
1. The system MUST complete login successfully
2. The system MUST NOT display any warnings

Reference Implementation: https://subdirectory.wpmt.co/index.php?link_header=true

# 11: WordPress in Subdirectory with HTML Link Tag

Requirements:
1. WordPress MUST be installed in a subdirectory
2. The site MUST use SSL
3. The root domain MUST contain a WordPress API link tag

Expected Signals:
1. The `Link` header MUST NOT be present
2. The HTML body MUST contain a valid `<link rel="https://api.w.org/" href="[...]" />` tag
3. The system MUST follow the API URL from the HTML tag

Behavior:
1. The system MUST complete login successfully
2. The system MUST NOT display any warnings

Reference Implementation: https://subdirectory.wpmt.co

# 12: Direct Subdirectory Access

Requirements:
1. WordPress MUST be installed in a subdirectory
2. The site MUST use SSL
3. The user MUST enter the correct subdirectory URL

Expected Signals:
1. The `Link` header MUST be present
2. The `authorization_url` MUST be present in the JSON API root

Behavior:
1. The system MUST complete login successfully
2. The system MUST NOT display any warnings

Reference Implementation: https://subdirectory.wpmt.co/index.php?redirect=true

# 13: Basic Authentication

Requirements:
1. The site MUST require Basic Authentication
2. The site MUST use SSL

Expected Signals:
1. Initial request MUST receive a 401 status code
2. After Basic Auth:
   - The `Link` header MUST be present
   - The `authorization_url` MUST be present in the JSON API root

Behavior:
1. The system MUST prompt for Basic Auth credentials
2. The system MUST complete login successfully after valid credentials

Reference Implementation: https://basic-auth.wpmt.co
Username: test@example.com
Password: str0ngp4ssw0rd!

# 14: Custom REST API Prefix

Requirements:
1. The site MUST use a non-standard REST API prefix
2. The site MUST use SSL

Expected Signals:
1. The `Link` header MUST point to the custom REST API path
2. The HTML link tag MUST reflect the custom path

Behavior:
1. The system MUST follow the custom REST API path
2. The system MUST complete login successfully

Reference Implementation: https://custom-rest-prefix.wpmt.co

# 15: Rate Limited Access

Requirements:
1. The site MUST be a WordPress Installation.
2. The site MUST use SSL.
3. The site MUST implement rate limiting.
4. The rate limiting response MUST include a valid `Retry-After` header.

Expected Signals:
1. One or more requests during the login process MUST receive a 429 status code
2. The 429 response MUST contain a valid `Retry-After` header with either:
   - A positive integer number of seconds to wait
   - An HTTP date after which to retry

Behavior:
1. The system MUST pause the login process when receiving a 429
2. The system MUST respect the `Retry-After` header value
3. The system MUST automatically retry after the specified delay
4. The system MUST complete login successfully after rate limiting expires
5. The system MUST display the error: "The server is rate limiting requests in a way that will never succeed. Please check your site's rate limit configuration." if the rate limit is not lifted after 3 attempts.

Reference Implementation: https://aggressive-rate-limiting.wpmt.co

# 16: Non-Existent Website

Requirements:
1. The domain MUST NOT resolve to a valid IP address.

Expected Signals:
1. HTTP stack emits a failure notice
2. Network connection MUST fail with either:
   - DNS resolution error
   - Connection timeout
   - Connection refused

Behavior:
1. The system MUST reject the login attempt
2. The system MUST display the error: "Unable to connect to `${DOMAIN}`. Please check that the website address is correct"

Reference Implementation: https://valid-looking-url-but-not-actually.foo

# 17: Invalid SSL Certificate

Requirements:
1. The site MUST be a WordPress Installation
2. The site MUST use HTTPS
3. The site's SSL certificate MUST be invalid due to one of:
   - Self-signed certificate
   - Expired certificate
   - Certificate with mismatched domain
   - Untrusted certificate authority

Expected Signals:
1. SSL/TLS handshake MUST fail with a certificate validation error
2. The system MUST NOT bypass certificate validation unless the domain is explicitly allowlisted.

Behavior:
1. The system MUST reject the login attempt
2. The system MUST display the error: "Unable to establish a secure connection to `${DOMAIN}`. The site's SSL certificate is invalid or expired"

Reference Implementation: https://wordpress-1315525-4803651.cloudwaysapps.com
