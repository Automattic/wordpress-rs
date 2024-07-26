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

## Testing

This project has several test suites. Integration tests require Docker, and you must run `make test-server && make dump-mysql && make backup-wp-content-plugins` prior to the test invocation.

| Test Suite                       | Run on local machine.                 | Run in Docker                     |
| -------------------------------- | ---------------------------------     | ---------------                   |
| Rust Unit Tests                  | `cargo test --lib`                    | `make test-rust-lib`              |
| Rust Documentation Tests         | `cargo test --doc`                    | `make test-rust-doc`              |
| Rust Integration Tests           | `cargo test --test '*'`               | Coming soon                       |
| Kotlin Integration Tests         | `cd native/kotlin && ./gradlew :api:kotlin:integrationTest`  | Coming soon                       |
| Swift Unit Tests                 | `swift test`                          | `make test-swift-linux-in-docker` |

