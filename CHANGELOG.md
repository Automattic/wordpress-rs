## Trunk

### Breaking Changes

_None_

### New Features

- [Wp Site Health Tests](https://developer.wordpress.org/rest-api/reference/wp-site-health-tests/) endpoint

_None_

### Bug Fixes

_None_

### Internal Changes

- `WpDerivedRequest` now supports plain `get` requests

_None_

## 0.1

This first release includes the following for the Kotlin, Rust & Swift platforms:
- Authentication using Application Passwords
- [Application Passwords](https://developer.wordpress.org/rest-api/reference/application-passwords/) endpoint
- [Users](https://developer.wordpress.org/rest-api/reference/users/) endpoint
- [Plugins](https://developer.wordpress.org/rest-api/reference/plugins/) endpoint

It also includes all of the underlying infrastructure for this – including:
- `wp_contextual` – A proc macro that generates `Edit`, `Embed` & `View` contextual types from given Sparse type
- `wp_derive_request_builder` – A proc macro that generates endpoint, request builder and request executor types
