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

# Generate API documentation
cargo doc --no-deps --all-features
```

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
- `wp_api_integration_tests/tests/test_credentials.json` (WordPress.org)
- `wp_api_integration_tests/tests/wp_com_test_credentials.json` (WordPress.com)

### Common Development Tasks

1. **Adding new types that uses WordPress REST API endpoint**:
   - The API returns different fields depending on the `context` parameter which can be `edit`, `embed` or `view` and defaults to `view`. To support this, we use the `wp_contextual::WpContextual` derive macro and add `#[WpContext(edit, embed, view)]` attribute to each field, using the available contexts.
   - Each contextual type's name has to start with `Sparse` prefix and all of its fields has to be `Option<T>`. `wp_contextual::WpContextual` derive macro will generate new types from it with the `WithEditContext`, `WithEmbedContext` & `WithViewContext` suffices. These new types will not use `Option<T>` for its fields unless a field in the original `Sparse` type is marked with `#[WpContextualOption]` attribute.
   - Most types should have the following derive macros `#[derive(Debug, serde::Serialize, serde::Deserialize, uniffi::Record)]`.
   - To implement new types, use `wp_api/src/posts.rs` as a reference and follow the same style
   - For a new endpoint, a set of JSONs should be provided to you for each context type, so you can compare them and figure out which field is returned for which contexts.

## Important Files

- `Makefile` - Build automation and platform-specific targets
- `wp_api/src/lib.rs` - Main library entry point
- `wp_api/src/request.rs` - Core request/response handling
- `wp_api/src/wp_error.rs` - Error types and handling

## Development Tips

- Platform bindings are generated automatically - don't edit generated files directly
