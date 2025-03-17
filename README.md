# WordPress Database Inspector

A small CLI tool for interacting with a WordPress database.

## Usage

Run the command with the following syntax:

```
wp_rs_cli [OPTIONS] <SUBCOMMAND>
```

### Options

- `-d, --database <DATABASE>`: Path to the WordPress database file
- `-h, --help`: Print help
- `-V, --version`: Print version

### Subcommands

- `wp-posts`: Operations related to WordPress posts
  - `list`: List posts in the WordPress database
  - `get`: Get a specific post by ID
  - `help`: Print this message or the help of the given subcommand(s)

- `wp-options`: Operations related to WordPress options
  - `list`: List options in the WordPress database
  - `get`: Get a specific option by name
  - `help`: Print this message or the help of the given subcommand(s)

- `help`: Print this message or the help of the given subcommand(s)

## Examples

List all posts:
```
wp_rs_cli -d path/to/wordpress.db wp-posts list
```

Get a specific post:
```
wp_rs_cli -d path/to/wordpress.db wp-posts get 1
```

List all options:
```
wp_rs_cli -d path/to/wordpress.db wp-options list
```

Get a specific option:
```
wp_rs_cli -d path/to/wordpress.db wp-options get siteurl
```

## Building from Source

Clone the repository and build using Cargo:

```
git clone https://github.com/your-username/wp_rs_cli.git
cd wp_rs_cli
cargo build --release
```

The binary will be available at `target/release/wp_rs_cli`.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

# wordpress-rs

> This project is under heavy development and shouldn't be considered production-ready yet. We're happy to hear any feedback you might have, but we're not yet ready to accept significant code contributions from others. We look forward to engaging with the community on this project in early 2025.

A cross-platform implementation of the [WordPress REST API](https://developer.wordpress.org/rest-api/) written in Rust, with bindings for Kotlin, Swift, and more.

## Prerequisites

The [Rust toolchain](https://www.rust-lang.org/tools/install) is required to build this project.

Given the multi-platform nature of this project, the development environment will vary depending on the platform you are targeting. Below are dependencies for each platform, grouped by language.

### Kotlin

| Dependency                                                | Platform         |
| --------------------------------------------------------- | ---------------- |
| [Gradle](https://gradle.org/install/)                     | Core             |
| [Android SDK](https://developer.android.com/tools)        | Android          |
| [Docker](https://www.docker.com/) (for integration tests) | Core + Android   |

### Swift

| Dependency                                                                       | Platform        |
| -------------------------------------------------------------------------------- | --------------- |
| [Swift toolchain](https://www.swift.org/install/macos/)                          | Core            |
| [Xcode](https://developer.apple.com/xcode/)                                      | Apple Platforms |
| [Docker](https://www.docker.com/)  (for integration tests and server-side Swift) | Core + Server   |

- **Core:** Required for all platforms.
- **Android:** Required for Android development.
- **Apple Platforms:** Required for iOS, macOS, watchOS, and tvOS development.
- **Server:** Required for server-side development.

## Development

Many of the project scripts are managed in a Makefile found in the root of the project. To see a list of available commands, run:

```sh
make help
```

See [this documentation](Documentation/debugging-from-xcode.md) if you want to debug Rust code from Xcode.

## Testing

This project has several test suites. Integration tests require Docker, and you must run `make test-server` prior to the test invocation.

| Test Suite                       | Run on local machine.                 | Run in Docker                     |
| -------------------------------- | ---------------------------------     | ---------------                   |
| Rust Unit Tests                  | `cargo test --lib`                    | `make test-rust-lib`              |
| Rust Documentation Tests         | `cargo test --doc`                    | `make test-rust-doc`              |
| Rust Integration Tests           | `cargo test -p wp_api_integration_tests` | `make test-rust-integration`    |
| Kotlin Integration Tests         | `cd native/kotlin && ./gradlew :api:kotlin:integrationTest` | `make test-kotlin-integration`   |
| Swift Unit Tests                 | `swift test`                          | `make test-swift-linux-in-docker` |

