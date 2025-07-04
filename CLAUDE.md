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

This section explains how to add new WordPress REST API endpoints and types to this codebase. The implementation follows a specific pattern to handle WordPress's context-aware responses and maintain type safety.

#### 1. Adding new types for WordPress REST API endpoints

WordPress REST API returns different fields depending on the `context` parameter (`view`, `edit`, or `embed`). We handle this using a procedural macro that generates context-specific types.

**Core concepts:**
- **Sparse types**: Base types with all fields as `Option<T>`, prefixed with `Sparse`
- **Context-specific types**: Generated types with appropriate fields for each context
- **Type safety**: New type wrappers for IDs and strongly-typed parameter enums

**Implementation steps:**

1. **Create the Sparse type** in `wp_api/src/{endpoint_name}.rs`:
   ```rust
   #[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
   pub struct SparseUser {
       #[WpContext(edit, embed, view)]
       pub id: Option<UserId>,
       #[WpContext(edit)]
       pub username: Option<String>,
       // ... other fields
   }
   ```
   - Start type name with `Sparse` prefix
   - All fields must be `Option<T>`
   - Add `#[WpContext(...)]` attributes based on API documentation
   - Fields marked with `#[WpContextualOption]` remain optional in generated types
   - Omit `_links` and `_meta` fields (add a comment for `_meta`)

2. **Create ID wrapper types** for type safety:
   ```rust
   impl_as_query_value_for_new_type!(UserId);
   uniffi::custom_newtype!(UserId, i64);
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   pub struct UserId(pub i64);
   ```

3. **Define parameter types** for list/create/update operations:
   ```rust
   #[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
   pub struct UserListParams {
       #[uniffi(default = None)]
       pub page: Option<u32>,
       // ... other fields
   }
   ```

4. **Implement query parameter handling**:
   - Create a `{Type}ListParamsField` enum:
     ```rust
     #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, IntoStaticStr)]
     enum UserListParamsField {
         #[strum(serialize = "page")]
         Page,
         // ... other fields
     }
     ```
   - Implement `AppendUrlQueryPairs` and `FromUrlQueryPairs` traits
   - Import helpers: `crate::url_query::{AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap}`

**Special parameter types:**

Some parameters require custom handling:
- **Enum parameters with partial serialization**: Use `OptionFromStr` trait (see `WpApiParamUsersWho`)
- **Complex parameters**: Implement custom `FromStr`/`Display` (see `WpApiParamUsersHasPublishedPosts`)
- **Parameters with special serialization**: Use serde attributes (see `UserAvatarSize`)

#### 2. Adding WordPress REST API endpoint implementations

Endpoints are implemented using a derive macro that generates the request builder functions.

**Implementation steps:**

1. **Create endpoint file** `wp_api/src/request/endpoint/{endpoint_name}_endpoint.rs`:
   ```rust
   use crate::{/* imports */};
   use wp_derive_request_builder::WpDerivedRequest;

   #[derive(WpDerivedRequest)]
   enum UsersRequest {
       #[contextual_paged(url = "/users", params = &UserListParams, output = Vec<crate::SparseUser>, filter_by = crate::SparseUserField)]
       List,
       #[post(url = "/users", params = &UserCreateParams, output = UserWithEditContext)]
       Create,
       // ... other variants
   }
   ```

2. **Choose appropriate attributes**:
   - `#[contextual_paged]` - For lists with pagination and context support
   - `#[contextual_get]` - For `GET` operations with context support
   - `#[get]` - For `GET` operations without context support
   - `#[post]` - For `POST` operations
   - `#[delete]` - For `DELETE` operations
   - `filter_by` parameter enables `_fields` query parameter support

3. **Use appropriate `output` types**
   - For lists with contextual types: `Vec<crate::{endpoint_name}::{sparse_endpoint_type}>`, i.e. `Vec<crate::posts::SparsePost>`
   - For single items with contextual types: `crate::{endpoint_name}::{sparse_endpoint_type}`, i.e. `crate::posts::SparsePost`
   - For non contextual types: `Vec<crate::{endpoint_name}::{return_type}>` & `crate::{endpoint_name}::{return_type}`

4. **Use appropriate `filter_by` types**
   - For lists with contextual types: `crate::{endpoint_name}::{sparse_field_type}`, i.e. `crate::posts::SparsePostField`
   - Procedural macro will turn `SparsePostField` into `SparsePostFieldWithEditContext`, `SparsePostFieldWithEmbedContext` & `SparsePostFieldWithViewContext`

5. **Handle special cases**:
   - **Delete vs Trash**: `Delete` requires `force=true`, `Trash` requires `force=false`
   - **URL parameters**: `<user_id>` becomes `UserId` parameter in generated functions

6. **Implement DerivedRequest trait**:
   ```rust
   impl DerivedRequest for UsersRequest {
       fn namespace() -> impl AsNamespace {
           WpNamespace::WpV2  // For /wp/v2 endpoints
       }
   }
   ```
   - Override `additional_query_pairs()` only for special cases (e.g., Delete/Trash)

7. **Add comprehensive unit tests**:
   - Test every endpoint variant
   - Test with default parameters
   - Test with all parameters populated
   - Use `validate_wp_v2_endpoint()` helper

8. **Add the new request builder & executor to `WpApiRequestBuilder` & `WpApiClient` in `wp_api/src/api_client.rs`**

#### 3. Error handling and integration tests

WordPress REST API returns specific error codes that need to be handled and tested.

**Implementation steps:**

1. **Add missing error codes** to `crate::WpErrorCode`:
   - Add new variants at the top with `// Needs Triage` comment for each one
   - Match the error codes from API responses

2. **Create error tests** in `wp_api_integration_tests/tests/test_{endpoint_name}_err.rs`:
   - Use appropriate client helpers:
     - `api_client()` - Admin authenticated (default)
     - `api_client_as_subscriber()` - Limited permissions
     - `api_client_with_auth_provider(WpAuthenticationProvider::none().into())` - Unauthenticated

3. **Document test rationale**:
   - Add doc comments explaining why tests are implemented a specific way
   - If unsure how to trigger an error, leave implementation empty with explanation

**Example references:**
- Types: `wp_api/src/posts.rs`, `wp_api/src/categories.rs`
- Endpoints: `wp_api/src/request/endpoint/posts_endpoint.rs`
- Error tests: `wp_api_integration_tests/tests/test_posts_err.rs`

## Important Files

- `Makefile` - Build automation and platform-specific targets
- `wp_api/src/lib.rs` - Main library entry point
- `wp_api/src/request.rs` - Core request/response handling
- `wp_api/src/api_client.rs` - Request builder & executor wrapper API client types
- `wp_api/src/api_error.rs` - Error types and handling
- `wp_api_integration_tests/src/lib.rs` - Helpers for integration tests

## Development Tips

- Platform bindings are generated automatically - don't edit generated files directly
