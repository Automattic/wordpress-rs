# WordPress-rs CLI

A command-line interface for interacting with WordPress sites.

## Features

- Runs autodiscovery on a site and prints the result

## Installation

### From source

```bash
git clone https://github.com/automattic/wordpress-rs.git
cd wordpress-rs
cargo build --release
```

The binary will be available at `target/release/wp_rs_cli`.

## Usage

```bash
# Basic usage
wp_rs_cli discover-login-url https://example.com 

# Get help
wp_rs_cli --help
```


## Commands

- `discover-login-url`: Tries connecting to the given URL, and prints the library's relevant error message if unable to.
- `fetch-post`: Fetch a post and its comments, supporting WordPress.com (Bearer token) and WordPress.org/Jetpack (Application Password) sites.
- `wpcom-login`: Sign in to WordPress.com in a browser and save the account's bearer token to a file.

### fetch-post examples

```bash
# WordPress.com (Bearer) by post URL (auto derive site)
wp_rs_cli fetch-post \
  --url https://example.wordpress.com/2024/07/01/my-post \
  --bearer "$WP_BEARER_TOKEN" \
  --pretty

# WordPress.com (Bearer) by explicit site and post id
wp_rs_cli fetch-post \
  --wpcom-site example.wordpress.com \
  --post-id 123 \
  --bearer "$WP_BEARER_TOKEN" \
  --pretty

# WordPress.org/Jetpack (Application Password) by post URL (auto-discovers API root)
wp_rs_cli fetch-post \
  --url https://yoursite.com/blog/2024/07/01/my-post \
  --username "$WP_USERNAME" \
  --password "$WP_APP_PASSWORD" \
  --pretty

# WordPress.org/Jetpack (Application Password) by explicit API root and post id
wp_rs_cli fetch-post \
  --api-root https://yoursite.com/wp-json \
  --post-id 123 \
  --username "$WP_USERNAME" \
  --password "$WP_APP_PASSWORD" \
  --pretty

# Same, but for a plain-permalinks site (WordPress advertises the
# `?rest_route=/` form when Settings → Permalinks is set to Plain)
wp_rs_cli fetch-post \
  --api-root 'https://yoursite.com/index.php?rest_route=/' \
  --post-id 123 \
  --username "$WP_USERNAME" \
  --password "$WP_APP_PASSWORD" \
  --pretty
```

## License

This project is licensed under the MPL license.

### wpcom-login

Signs in with the OAuth2 authorization code flow and writes the token to a file only you can read. The token is never printed.

You need a WordPress.com application from https://developer.wordpress.com/apps/. The command reads its client ID and secret from a JSON file such as this repository's `wp_com_test_credentials.json`.

```bash
# The application lists http://localhost:8765/callback as a redirect URL: the sign-in is caught automatically.
wp_rs_cli wpcom-login --credentials-file wp_com_test_credentials.json --out ~/.wpcom-token

# Any other redirect URL: paste the address the browser ends up at when asked.
wp_rs_cli wpcom-login --credentials-file wp_com_test_credentials.json --redirect-uri myapp://authorized --out ~/.wpcom-token

# Print the sign-in address instead of opening a browser, to sign in as another account in a private window.
wp_rs_cli wpcom-login --credentials-file wp_com_test_credentials.json --out ~/.wpcom-token-2 --no-browser
```

The client ID can also come from `--client-id` or `WP_COM_CLIENT_ID`, and the secret from `--client-secret-file` or `WP_COM_CLIENT_SECRET`. The secret is never taken as an argument, so it stays out of the shell history.
