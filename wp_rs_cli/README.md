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
# WordPress.com (Bearer)
wp_rs_cli fetch-post \
  --wpcom-site example.wordpress.com \
  --post-id 123 \
  --bearer "$WP_BEARER_TOKEN" \
  --pretty

# WordPress.org/Jetpack (Application Password)
wp_rs_cli fetch-post \
  --api-root https://yoursite.com/wp-json \
  --post-id 123 \
  --username "$WP_USERNAME" \
  --password "$WP_APP_PASSWORD" \
  --pretty
```

## License

This project is licensed under the MPL license.
