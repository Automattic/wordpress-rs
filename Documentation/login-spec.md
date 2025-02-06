# Login

Login for WordPress.org sites is pretty complicated – there are many different scenarios that we need to handle. These are documented below.


# 1: Happy Path Login

Conditions:
1. Stock WordPress Installation with no plugins.
2. The site uses SSL.
3. There is no CDN or aggressive caching preventing the `Link` header from being sent with each front-end HTTP response.

Signals:
1. The `Link` header is present
2. The `authorization_url` is present in the JSON API root

Outcome:
1. Login is successful.

Example Site: https://vanilla.wpmt.co

# 2: Local Development Environment without overrides

Conditions:
1. Stock WordPress Installation with no plugins.
2. The site doesn't have a valid SSL certificate (because it's a local dev environment)
3. There is no `WP_ENVIRONMENT_TYPE` set, or it's set to something other than `production`

Signals:
1. The `Link` header is present.
2. The `authorization_url` is *not* present in the JSON API root.
3. The `has_https` check does not pass.
4. The URL is an IP address, ends in `.dev` or `.local`, or is `localhost`

Outcome:
1. Login fails with the error "This site is a local development environment. You'll need to enable application passwords to connect to it with the app."

http://localhost

# 3: User enters a wp-admin URL for an otherwise happy-path login

Conditions:
1. Stock WordPress Installation with no plugins.
2. The site uses SSL.
3. There is no CDN or aggressive caching preventing the `Link` header from being sent with each front-end HTTP response.
4. User incorrectly appends /wp-login.php to the URL.

Signals:
1. The user-entered URL fails to find the API root (because the `Link` header isn't present)
2. The "magic" login attempt drops everything after the WordPress root. For this attempt, the `Link` header is present 
3. The `authorization_url` is present in the JSON API root

https://vanilla.wpmt.co/wp-login.php?redirect_to=https%3A%2F%2Fvanilla.wpmt.co%2Fwp-admin%2Fplugins.php&reauth=1

Outcome:
1. Login is successful.

# 4: User explicitly specifies HTTP for a site that supports HTTPs

Conditions:
1. Stock WordPress Installation with no plugins.
2. The site uses SSL, but responds for HTTP requests without directs.
3. There is no CDN or aggressive caching preventing the `Link` header from being sent with each front-end HTTP response.

Signals:
1. The `Link` header is present.
2. The `authorization_url` is *not* present in the JSON API root.
3. The `has_https` check does not pass.
4. The "magic" login attempt tries with HTTPs, which is successful.

Outcome:
1. Login is successful

http://optional-https.wpmt.co


# 5: User explicitly specifies HTTP for a site that doesn't support HTTPS without special handling

Conditions:
1. Stock WordPress Installation with no plugins.
2. The site doesn't have a valid SSL certificate.
3. The site doesn't have any override to enable Application Passwords.

Signals:
1. The `Link` header is present.
2. The `authorization_url` is *not* present in the JSON API root.
3. The `has_https` check does not pass.

Outcome:
1. Login fails with the error "Unable to securely connect to `${DOMAIN}` – make sure it's using SSL"

// TODO: Build a test site for this

# 5: User explicitly specifies HTTP for a site that doesn't support HTTPS, but the site explicitly enables Application Passwords over HTTP

Conditions:
1. Stock WordPress Installation with no plugins.
2. The site doesn't have a valid SSL certificate.
3. The site explicitly enable Application Passwords.

Signals:
1. The `Link` header is present.
2. The `authorization_url` is present in the JSON API root.
3. The `has_https` check does not pass.

Outcome:
1. The response has a warning about the lack of HTTPS, but returns a successful outcome. The app should show the warning.

// TODO: Build a test site for this

# 6: The site uses a CDN that doesn't emit the `Link` header, but the site is otherwise valid

Conditions:
1. Stock WordPress Installation with no plugins.
2. The site has a valid SSL certificate.
3. The site uses a CDN like CloudFlare to cache page responses. This CDN doesn't pass along the `Link` header.

Signals:
1. The `Link` header is not present.
2. The HTTP body contains the `<link rel="https://api.w.org/" href="[...]" />` tag.
3. The login process continues like normal.

Outcome:
1. Login fails with the error "Unable to securely connect to `${DOMAIN}` – make sure it's using SSL"

https://aggressive-caching.wpmt.co

# 7: The site uses a plugin that disables Application Passwords

Conditions:
1. WordPress Installation contains a plugin like WordFence that disables Application Passwords.
2. The site has a valid SSL certificate.

Signals:
1. The `Link` header is present.
2. The `authorization_url` is present in the JSON API root
3. The WP-JSON root has a `wordfence/v1` namespace

Outcome:
1. Login fails with the error "Unable to login to `${DOMAIN}` – the `${PLUGIN_NAME}` plugin might have disabled Application Passwords. Please check your plugin settings and try again"

https://wordfence.wpmt.co

# 8: The given domain isn't a WordPress site

Conditions:
1. WordPress Installation exists at /wordpress.
2. The site has a valid SSL certificate.

Signals:
1. The `Link` header is not present.
2. The HTTP body does not contain the `<link rel="https://api.w.org/" href="[...]" />` tag.
3. The `is_wordpress` check returns `false`

Outcome:
1. Login fails with the error "Unable to login to `${DOMAIN}`. Please double-check that this is a WordPress site"

// TODO: Build a test site for this

# 9: The (correctly configured) WordPress site is in a subdirectory, and the user enters the root domain

Conditions:
1. WordPress Installation exists at /wordpress.
2. The site has a valid SSL certificate.
3. The site doesn't emit a `Link` header pointing at the WordPress path.
4. The HTTP body does not contain the `<link rel="https://api.w.org/" href="[...]" />` tag.

Signals:
1. The `Link` header is not present.
2. The HTTP body does not contain the `<link rel="https://api.w.org/" href="[...]" />` tag.
3. The `is_wordpress` check returns `false`

Outcome:
1. Login fails with the error "Unable to login to `${DOMAIN}`. Please double-check that this is a WordPress site"

// TODO: Build a test site for this

# 10:  The (correctly configured) WordPress site is in a subdirectory, and the user enters the root domain, but the site emits the `Link` header

Conditions:
1. WordPress Installation exists at /wordpress.
2. The site has a valid SSL certificate.
3. The site emits a `Link` header pointing at the WordPress path.
4. The HTTP body does not contain the `<link rel="https://api.w.org/" href="[...]" />` tag.

Signals:
1. The `Link` header is present.
2. The login continues as normal

Outcome:
1. Login is successful.

// TODO: Build a test site for this

# 11:  The (correctly configured) WordPress site is in a subdirectory, and the user enters the root domain, the `Link` header isn't present, but a `<link rel="https://api.w.org />` tag is.

Conditions:
1. WordPress Installation exists at /wordpress.
2. The site has a valid SSL certificate.
3. The site does not emit a `Link` header pointing at the WordPress path.
4. The HTTP body contains the `<link rel="https://api.w.org/" href="[...]" />` tag.

Signals:
1. The `Link` header is not present.
2. The HTTP body contains the `<link rel="https://api.w.org/" href="[...]" />` tag.
3. The login continues as normal.

Outcome:
1. Login is successful.

// TODO: Build a test site for this

# 12:  The (correctly configured) WordPress site is in a subdirectory, and the user enters the subdirectory path

Conditions:
1. WordPress Installation exists at /wordpress.
2. The site has a valid SSL certificate.
3. The site emits a `Link` header pointing at the WordPress path, but only in the WordPress installation.
4. The user has entered the WordPress installation path.

Signals:
1. The `Link` header is present
2. The `authorization_url` is present in the JSON API root

Outcome:
1. Login is successful.

// TODO: Build a test site for this

# 13: Site with Basic Authentication

Conditions:
1. WordPress Installation with Basic Authentication enabled (via .htaccess or other mechanism)
2. The site has a valid SSL certificate
3. User needs to pass Basic Auth before accessing WordPress

Signals:
1. Initial request receives a 401 status code
2. After providing Basic Auth credentials:
   - The `Link` header is present
   - The `authorization_url` is present in the JSON API root

Outcome:
1. App should prompt for Basic Auth credentials
2. After providing correct credentials, login is successful

// TODO: Build a test site for this

# 14: Site with Custom REST API Prefix

Conditions:
1. WordPress Installation where the REST API prefix has been changed from wp-json
2. The site has a valid SSL certificate

Signals:
1. The `Link` header points to a non-standard REST API path
2. The `<link>` tag in HTML also reflects this custom path

Outcome:
1. Login should succeed by following the custom REST API path

// TODO: Build a test site for this