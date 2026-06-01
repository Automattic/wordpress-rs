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
