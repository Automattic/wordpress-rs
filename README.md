# wordpress-rs

> This project is under heavy development and shouldn't be considered production-ready yet. We're happy to hear any feedback you might have, but we're not yet ready to accept significant code contributions from others. We look forward to engaging with the community on this project in early 2025.

A cross-platform implementation of the [WordPress REST API](https://developer.wordpress.org/rest-api/) written in Rust, with bindings for Kotlin, Swift, and more platforms.

## Prerequisites

The [Rust toolchain](https://www.rust-lang.org/tools/install) is required to build this project.

Given the multi-platform nature of this project, the development environment will vary depending on the platform you are targeting. Below are dependencies for each platform, grouped by language.

### Kotlin

| Dependency                                                | Platform         |
| --------------------------------------------------------- | ---------------- |
| [Gradle](https://gradle.org/install/)                     | Core             |
| [Android SDK](https://developer.android.com/tools)        | Android          |
| [Docker](https://www.docker.com/) (for integration tests) | Core + Android   |

See [Android Studio Configuration](#android-studio-configuration) for required IDE setup.

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

#### Android Studio Configuration

This project generates large Kotlin files that exceed Android Studio's default indexing limits. Add the following VM option to enable proper code indexing:

```
-Didea.max.intellisense.filesize=999999
```

To add this:
1. Go to `Help` > `Edit Custom VM Options`
2. Add the line to the `studio.vmoptions` file
3. Invalidate caches and restart Android Studio

This setting helps resolve indexing issues with large generated files like `wp_api.kt`.

**Troubleshooting:** If Android Studio cannot find `cargo`, it may be using its internal JDK instead of the system one. To fix this, [switch Android Studio to use your system JDK](https://stackoverflow.com/questions/30631286/how-to-specify-the-jdk-version-in-android-studio).

