# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- [Post Types](https://developer.wordpress.org/rest-api/reference/post-types/) endpoint
- [Site Settings](https://developer.wordpress.org/rest-api/reference/settings/) endpoint
- [Wp Site Health Tests](https://developer.wordpress.org/rest-api/reference/wp-site-health-tests/) endpoint
- [Template Parts](https://developer.wordpress.org/rest-api/reference/wp_template_parts/) endpoint
- [Template Revisions](https://developer.wordpress.org/rest-api/reference/wp_template-revisions/) endpoint
- [Template Autosaves](https://developer.wordpress.org/rest-api/reference/wp_template-revisions/) endpoint
- [Template Part Autosaves](https://developer.wordpress.org/rest-api/reference/wp_template_part-revisions/) endpoint
- WordPress.com Publicize endpoints (`/sites/<site>/publicize/connections` and `/sites/<site>/publicize/services`) for listing, creating, updating, and deleting Jetpack Social connections
- WordPress.com `/me/connections` (keyring) endpoint for listing third-party OAuth connections used by Jetpack Social
- `WpApiCache` APIs to remove cached data for a self-hosted site (by URL) or a WordPress.com site (by site ID), with matching Swift wrappers on `WordPressApiCache`
- `WpDerivedRequest` now supports plain `get` requests
- `WpDerivedRequest` now supports `additional_query_pairs`

### Changed

- **BREAKING:** [Condense error variants into `WpError`](https://github.com/Automattic/wordpress-rs/pull/230)
- **BREAKING:** [Contextual filtering](https://github.com/Automattic/wordpress-rs/pull/176)
- **BREAKING:** Renamed `AnyJson` to `WpAdditionalFields`; the type is now constructible from foreign bindings and exposes typed value accessors
- **BREAKING:** `PostMeta` is now a `uniffi::Object` wrapping the raw meta payload instead of a typed record with a single `footnotes` field; consumers must use the `footnotes()` / `with_footnotes()` accessors and arbitrary meta keys are reachable via `value_for_key` / `with_value`
- **BREAKING:** `PostCreateParams::meta`, `PostUpdateParams::meta`, and `SparseAnyPost::meta` are now `Option<Arc<PostMeta>>` instead of `Option<PostMeta>`
- **BREAKING:** [Replace `WpService.selfHosted` and `WpService.wordpressCom` with a single `WpService.new(siteInfo:)` constructor](https://github.com/Automattic/wordpress-rs/pull/1239). `SiteInfo.SelfHosted` now carries `ParsedUrl` values for `site_url` and `api_root` instead of `String`, and the `wordpress_com_site_api_root` helper has been removed — construct `SiteInfo.WordPressCom` directly.
- **BREAKING:** [Swift `WordPressAPI` initializers now accept a `SiteInfo`](https://github.com/Automattic/wordpress-rs/pull/1239) instead of `apiRootUrl`/`apiUrlResolver`, and the `siteUrl` parameter is now a `ParsedUrl` rather than a `String`.
- **BREAKING:** `SparseUser::extra_capabilities` is now `Option<UserCapabilitiesMap>` instead of `Option<HashMap<String, bool>>`, mirroring the wrapper used by `SparseUser::capabilities`. In Swift and Kotlin bindings, `UserWithEditContext.extraCapabilities` is now `UserCapabilitiesMap` rather than `[String: Bool]` / `Map<String, Boolean>`
- Reformat `CHANGELOG.md` to follow [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and enforce changelog updates on PRs via a Buildkite check that also surfaces the failure as a GitHub PR comment (using the shared `comment_on_pr` helper from `a8c-ci-toolkit`)
- Release documentation

### Fixed

- [Support both Integer and String for `WPApiDetails.gmt_offset`](https://github.com/Automattic/wordpress-rs/pull/209)
- Pin `wp-cli/restful` to v0.4.1 to fix Docker build (v0.4.2+ requires unreleased `wp-cli ^2.13`)
- **Internal:** `WpApiCache` lookups for a self-hosted site by URL now tolerate trailing-slash and other URL-normalization differences.
- [Deserialize `extra_capabilities` on legacy WordPress sites and on sites where plugins write non-bool capability values to `wp_capabilities` usermeta](https://github.com/Automattic/wordpress-rs/issues/1313)

## [0.1]

Initial release with support for Kotlin, Rust, and Swift platforms.

### Added

- Authentication using Application Passwords
- [Application Passwords](https://developer.wordpress.org/rest-api/reference/application-passwords/) endpoint
- [Users](https://developer.wordpress.org/rest-api/reference/users/) endpoint
- [Plugins](https://developer.wordpress.org/rest-api/reference/plugins/) endpoint
- `wp_contextual` – a proc macro that generates `Edit`, `Embed` & `View` contextual types from a given Sparse type
- `wp_derive_request_builder` – a proc macro that generates endpoint, request builder, and request executor types
