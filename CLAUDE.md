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

2. **Error handling for an endpoint**:
   - The server will return a `crate::WpErrorCode` instance for most error types.
   - While implementing the errors for a new endpoint, if it's missing from the `crate::WpErrorCode` variants, it needs to be added there. If you need to do this, please add the new variants to the very top of the type and add a comment on top with `// Needs Triage`.
   - Integration tests for error cases go into `wp_api_integration_tests/tests/test_{endpoint_name}_err.rs` file.
   - The implementation should follow a similar approach to `wp_api_integration_tests/tests/test_users_err.rs`.
   - There are several `api_client` helper functions available. The default `api_client` function is authenticated with an admin users. This should be the preferred client if creating the error case doesn't require an inauthenticated or a separate user. There is also `api_client_as_subscriber` function that is authenticated with a subscriber user. Most authentication error types can be triggered using this client type. Another possibility is `api_client_with_auth_provider(WpAuthenticationProvider::none().into())` which doesn't have any authentication headers, so it's useful in specific cases.
   - Implementing these tests can be difficult without having a full understanding of how to trigger them. So, if you are not sure how to implement it, generate a test function following existing patterns, but leave the implementation empty. Instead, add a comment about what you can find from the implementation related to how one might be able to trigger this error.
   - The existing tests don't have much documentation, because the test implementation can act as one. However, when you are implementing the test, please add a documentation. This is because we need some context about why you implemented a test in a specific way. If you include a documentation, we can check if what you are trying to do is correct, before reviewing the implementation.

## Important Files

- `Makefile` - Build automation and platform-specific targets
- `wp_api/src/lib.rs` - Main library entry point
- `wp_api/src/request.rs` - Core request/response handling
- `wp_api/src/wp_error.rs` - Error types and handling

## Development Tips

- Platform bindings are generated automatically - don't edit generated files directly
