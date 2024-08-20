## Trunk

### Breaking Changes

- [Condense error variants into WpError](https://github.com/Automattic/wordpress-rs/pull/230)
- [Contextual filtering](https://github.com/Automattic/wordpress-rs/pull/176)

### New Features

- [Post Types](https://developer.wordpress.org/rest-api/reference/post-types/) endpoint
- [Site Settings](https://developer.wordpress.org/rest-api/reference/settings/) endpoint
- [Wp Site Health Tests](https://developer.wordpress.org/rest-api/reference/wp-site-health-tests/) endpoint

### Bug Fixes

- [Support both Integer and String for `WPApiDetails.gmt_offset`](https://github.com/Automattic/wordpress-rs/pull/209)

### Internal Changes

- `WpDerivedRequest` now supports plain `get` requests
- `WpDerivedRequest` now supports `additional_query_pairs`

## 0.1

This first release includes the following for the Kotlin, Rust & Swift platforms:
- Authentication using Application Passwords
- [Application Passwords](https://developer.wordpress.org/rest-api/reference/application-passwords/) endpoint
- [Users](https://developer.wordpress.org/rest-api/reference/users/) endpoint
- [Plugins](https://developer.wordpress.org/rest-api/reference/plugins/) endpoint

It also includes all of the underlying infrastructure for this – including:
- `wp_contextual` – A proc macro that generates `Edit`, `Embed` & `View` contextual types from given Sparse type
- `wp_derive_request_builder` – A proc macro that generates endpoint, request builder and request executor types
