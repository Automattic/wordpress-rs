# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- [Post Types](https://developer.wordpress.org/rest-api/reference/post-types/) endpoint
- [Site Settings](https://developer.wordpress.org/rest-api/reference/settings/) endpoint
- [Wp Site Health Tests](https://developer.wordpress.org/rest-api/reference/wp-site-health-tests/) endpoint
- `WpDerivedRequest` now supports plain `get` requests
- `WpDerivedRequest` now supports `additional_query_pairs`

### Changed

- **BREAKING:** [Condense error variants into `WpError`](https://github.com/Automattic/wordpress-rs/pull/230)
- **BREAKING:** [Contextual filtering](https://github.com/Automattic/wordpress-rs/pull/176)
- Reformat `CHANGELOG.md` to follow [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and enforce changelog updates on PRs via a Buildkite check

### Fixed

- [Support both Integer and String for `WPApiDetails.gmt_offset`](https://github.com/Automattic/wordpress-rs/pull/209)

## [0.1]

Initial release with support for Kotlin, Rust, and Swift platforms.

### Added

- Authentication using Application Passwords
- [Application Passwords](https://developer.wordpress.org/rest-api/reference/application-passwords/) endpoint
- [Users](https://developer.wordpress.org/rest-api/reference/users/) endpoint
- [Plugins](https://developer.wordpress.org/rest-api/reference/plugins/) endpoint
- `wp_contextual` – a proc macro that generates `Edit`, `Embed` & `View` contextual types from a given Sparse type
- `wp_derive_request_builder` – a proc macro that generates endpoint, request builder, and request executor types
