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

## License

This project is licensed under the MPL license.
