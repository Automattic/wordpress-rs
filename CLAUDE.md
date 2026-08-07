# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust implementation of the WordPress REST API client library with cross-platform bindings for iOS, Android, and other platforms. The project uses a workspace structure with multiple crates providing modular functionality.

## Build Commands

```bash
# Start WordPress test instance
make test-server

# Run unit tests
cargo test --lib

# Run integration tests
cargo test -p wp_api_integration_tests

# Run integration tests for a specific file
cargo test -p wp_api_integration_tests --test '{file_name}'

# Run linting and format checks
cargo fmt --all -- --check
cargo clippy --tests --all-targets --all-features -- -D warnings
swift package plugin swiftlint --strict

# Generate API documentation
cargo doc --no-deps --all-features

# Generate Swift bindings and an xcframework `swift build` can link.
# Use this to verify UniFFI changes locally — it builds 2 targets (~18GB).
make xcframework-only-macos
swift build --target WordPressAPI

# Compile the generated Kotlin bindings and lint them. Builds for the host, so
# no Android NDK is required.
cd native/kotlin && ./gradlew :api:kotlin:compileKotlin :api:kotlin:detekt
```

**Do not run `make xcframework` to verify a local change.** It builds all 11
Apple targets, consumes ~100GB in `target/`, and takes tens of minutes. It is
what CI and releases need, not what a code change needs. Note also that
`xcframework-only-<platform>` *replaces* the xcframework rather than adding a
slice, so running the `ios` variant will break a subsequent `swift build`.

See `Documentation/local-development.md` for the full rationale and the fast
path for verifying a UniFFI change.

## Architecture

### Workspace Structure
- `wp_api/` - Core REST API implementation
- `wp_api_integration_tests/` - Integration tests requiring Dockerized WordPress instance
- `wp_contextual/` - Procedural macro for context-aware types
- `wp_serde/` - Custom serialization helpers
- `uniffi-bindgen/` - Cross-platform binding generator
- `kotlin/` - Kotlin/Android wrapper for generated bindings
- `native/apple/` - Swift/iOS wrapper for generated bindings

### Testing

Tests require a WordPress instance. Use Docker:
```bash
# Start test server (keep running)
make test-server

# Run the integration tests
cargo test -p wp_api_integration_tests
```

Test credentials are configured in:
- `test_credentials.json` (WordPress.org)
- `wp_com_test_credentials.json` (WordPress.com)

## Important Files

- `Makefile` - Build automation and platform-specific targets
- `wp_api/src/lib.rs` - Main library entry point
- `wp_api/src/request.rs` - Core request/response handling
- `wp_api/src/api_client.rs` - Request builder & executor wrapper API client types
- `wp_api/src/api_error.rs` - Error types and handling
- `wp_api_integration_tests/src/lib.rs` - Helpers for integration tests

## Development Tips

- Platform bindings are generated automatically - don't edit generated files directly
- For URL columns in the `wp_mobile_cache` database (e.g. `self_hosted_sites.url`), compare with the `urls_eq(a, b)` scalar function rather than `=`. It compares via `ParsedUrl` parsing, so lookups tolerate trailing-slash and other URL-normalization differences.

## PR Description Format

When writing PR descriptions, use the template in `.github/PULL_REQUEST_TEMPLATE.md` as the base format.

**Format Rules:**
- Always output PR descriptions as raw markdown inside a code block so users can copy them directly
- Use bullet points or numbered lists for changes
