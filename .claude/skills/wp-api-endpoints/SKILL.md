---
name: wp-api-endpoints
description: Guide for adding new WordPress REST API endpoints and types to the wordpress-rs codebase. Use when (1) adding new sparse types or context-aware API response types, (2) implementing new REST API endpoint request builders, (3) adding error handling and integration tests for endpoints, or (4) creating ID wrapper types or parameter types for API operations.
---

# Adding WordPress REST API Endpoints

This codebase uses procedural macros to handle WordPress's context-aware responses (`view`, `edit`, `embed`) and maintain type safety.

## 1. Adding New Types

Create types in `wp_api/src/{endpoint_name}.rs`.

### Sparse Types

```rust
#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseUser {
    #[WpContext(edit, embed, view)]
    pub id: Option<UserId>,
    #[WpContext(edit)]
    pub username: Option<String>,
}
```

- Prefix type name with `Sparse`, all fields `Option<T>`
- `#[WpContext(...)]` per API docs; `#[WpContextualOption]` keeps fields optional in generated types
- Omit `_links` and `_meta` fields (add a comment for `_meta`)

### ID Wrapper Types

```rust
impl_as_query_value_for_new_type!(UserId);
uniffi::custom_newtype!(UserId, i64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserId(pub i64);
```

### Parameter Types

```rust
#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct UserListParams {
    #[uniffi(default = None)]
    pub page: Option<u32>,
    #[uniffi(default = [])]
    pub exclude: Vec<UserId>,
}
```

- `WpDeriveParamsField` generates field enum and query parameter handling (import from `wp_derive`)
- `#[supports_pagination(true/false)]` — `#[field_name("custom_name")]` if API name differs
- **Array params**: use `Vec<T>` with `#[uniffi(default = [])]`, NOT `Option<Vec<T>>`

**Special parameter types:**
- Enum with partial serialization: `OptionFromStr` trait (see `WpApiParamUsersWho`)
- Complex params: custom `FromStr`/`Display` (see `WpApiParamUsersHasPublishedPosts`)
- Special serialization: serde attributes (see `UserAvatarSize`)

## 2. Adding Endpoint Implementations

Create `wp_api/src/request/endpoint/{endpoint_name}_endpoint.rs`:

```rust
use wp_derive_request_builder::WpDerivedRequest;

#[derive(WpDerivedRequest)]
enum UsersRequest {
    #[contextual_paged(url = "/users", params = &UserListParams, output = Vec<crate::SparseUser>, filter_by = crate::SparseUserField)]
    List,
    #[post(url = "/users", params = &UserCreateParams, output = UserWithEditContext)]
    Create,
}
```

**Attributes:**
- `#[contextual_paged]` — lists with pagination + context
- `#[contextual_get]` — GET with context
- `#[get]` / `#[post]` / `#[delete]` — without context

**Output types:**
- Contextual lists: `Vec<crate::{mod}::{SparseType}>`
- Contextual single: `crate::{mod}::{SparseType}`
- Non-contextual: the concrete type directly

**`filter_by`:** use `crate::{mod}::{SparseField}` — macro generates `SparseFieldWith{Edit,Embed,View}Context`

**Special cases:**
- Delete vs Trash: `Delete` needs `force=true`, `Trash` needs `force=false`
- URL params: `<user_id>` becomes `UserId` in generated functions

**DerivedRequest trait:**

```rust
impl DerivedRequest for UsersRequest {
    fn namespace() -> impl AsNamespace {
        WpNamespace::WpV2
    }
}
```

Override `additional_query_pairs()` only for special cases (e.g., Delete/Trash).

**Unit tests:** test every variant with default and fully-populated params using `validate_wp_v2_endpoint()`.

**Wire up:** add request builder & executor to `WpApiRequestBuilder` & `WpApiClient` in `wp_api/src/api_client.rs`.

## 3. Error Handling and Integration Tests

1. Add missing error codes to `crate::WpErrorCode` with `// Needs Triage` comment
2. Create `wp_api_integration_tests/tests/test_{endpoint_name}_err.rs` using:
   - `api_client()` — admin
   - `api_client_as_subscriber()` — limited permissions
   - `api_client_with_auth_provider(WpAuthenticationProvider::none().into())` — unauthenticated
3. Add doc comments explaining test rationale; leave empty with explanation if unsure how to trigger an error

## Reference Files

- Types: `wp_api/src/posts.rs`, `wp_api/src/categories.rs`
- Endpoints: `wp_api/src/request/endpoint/posts_endpoint.rs`
- Error tests: `wp_api_integration_tests/tests/test_posts_err.rs`
