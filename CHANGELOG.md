# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `WpDeriveParsedValue` proc macro that generates `{Enum}Value` wrapper types for enums with fallback variants. The generated type is a `uniffi::Object` that pairs a parsed `Option<Enum>` with the raw API string, ensuring forward-compatible comparisons when new enum variants are added.

### Changed

- **BREAKING:** `PostStatus` fields in response and params types (`SparseAnyPost.status`, `PostCreateParams.status`, `PostUpdateParams.status`, `MediaCreateParams.status`, `MediaUpdateParams.status`) are now `Arc<PostStatusValue>` instead of `PostStatus`. Use `PostStatusValue.newFromRaw(string)` or `PostStatusValue.newFromValue(variant)` to construct, and `parsed()`, `raw()`, `matches()`, `matchesRaw()`, `matchesAny()` to inspect and compare values.

## [0.4.0] - 2026-05-29

### Added

- `MediaService.create_media` for uploading media, with a Swift `WpService.uploadMedia` wrapper that streams upload progress
- `MediaService.update_media` for editing media items
- Sort/ordering predicates on `MediaListFilter`
- Live membership updates on `MediaMetadataCollectionWithEditContext`, so cached media lists reflect items moving in or out of the filter without a full refresh
- WordPress.com `/domains/{name}/is-available` endpoint for checking domain availability, pricing, and transfer status
- WordPress.com `/all-domains` endpoint for listing all domains across a user's sites

### Changed

- **BREAKING:** Add `Alert`, `Neutral`, and `Premium` variants to `DomainListItemStatusType`. Values that previously deserialized as `Other(String)` will now match their own variants, which may affect exhaustive `match`/`when` expressions.
- **BREAKING:** Add `SiteRedirect` variant to `DomainSubtypeId` for redirect domains. Previously deserialized as `Other("site_redirect")`.
- **BREAKING:** Replace `u8`, `u16` types with `u32` across struct fields, function parameters, return types, and enum reprs as a defensive measure against an [Android ART AOT compiler bug](https://github.com/jkmassel/uniffi-armv7-aot-checksum-bug) that mishandles small integer JNI return values on ARM32 ([#1339](https://github.com/Automattic/wordpress-rs/issues/1339))
- `MediaService.delete_media_permanently` now updates live media collections in place, so deletes appear without a refresh round-trip
- **Internal:** Buildkite step on trunk pushes that prunes `pr-build/<n>` branches whose PR is closed, sweeping orphans accumulated since `publish_pr_xcframework` started creating them.
- **Internal:** Group Dependabot Ruby minor/patch updates and cap open bundler PRs at 5.
- **Internal:** Request reviews from the apps-infra-tooling team on Dependabot bundler PRs.

## [0.3.0] - 2026-05-18

### Added

- [Block Directory Items](https://developer.wordpress.org/rest-api/reference/block-directory-items/) endpoint
- [Block Pattern Categories](https://developer.wordpress.org/rest-api/reference/block-pattern-categories/) endpoint
- [Block Patterns](https://developer.wordpress.org/rest-api/reference/block-patterns/) endpoint
- [Block Types](https://developer.wordpress.org/rest-api/reference/block-types/) endpoint
- [Editor Blocks](https://developer.wordpress.org/rest-api/reference/blocks/) endpoint
- [Block Revisions](https://developer.wordpress.org/rest-api/reference/block-revisions/) endpoint
- [Block Autosaves](https://developer.wordpress.org/rest-api/reference/block-revisions/) endpoint
- [Global Styles](https://developer.wordpress.org/rest-api/reference/wp_global_styles/) endpoint
- [Global Styles Revisions](https://developer.wordpress.org/rest-api/reference/wp_global_styles-revisions/) endpoint
- [Pattern Directory Items](https://developer.wordpress.org/rest-api/reference/pattern-directory-items/) endpoint
- [Rendered Blocks](https://developer.wordpress.org/rest-api/reference/rendered-blocks/) endpoint
- [Sidebars](https://developer.wordpress.org/rest-api/reference/sidebars/) endpoint
- `MediaService` on `WpService` (sync, fetch, state tracking, `delete_media_permanently`) and `MediaMetadataCollectionWithEditContext`, mirroring the existing `PostService` / `PostMetadataCollectionWithEditContext` pattern for a cached, paginated, observable media list
- `MediaListFilter`, the subset of `MediaListParams` that backs `MediaService.create_media_metadata_collection_with_edit_context` (excludes pagination, include/exclude, and date ranges)
- `wp_mobile_cache` storage for media: `media_edit_context` table (migration 0014), `DbTable::MediaEditContext`, `EntityType::MediaEditContext`, and a `MediaRepository<EditContext>` mirroring `PostRepository` minus term relationships
- `MetadataService::remove_entity_from_lists_with_key_prefix` so service-level deletes can scrub a deleted entity from every cached list for a site without waiting for a refresh

### Changed

- `MediaDetails` now derives `Eq + Hash` (raw-JSON-string comparison) and is exported via `#[uniffi::export(Eq, Hash)]`; `SparseMedia`'s `#[WpContextualDontDerivePartialEq]` opt-out is removed so `MediaWithEditContext` and the generated `FullEntityMediaWithEditContext` Swift wrapper synthesize `Equatable + Hashable`

### Removed

- Investigation artifacts for Nav Menu Item Autosaves

## [0.2.0] - 2026-05-05

### Added

- [Post Types](https://developer.wordpress.org/rest-api/reference/post-types/) endpoint
- [Site Settings](https://developer.wordpress.org/rest-api/reference/settings/) endpoint
- [Wp Site Health Tests](https://developer.wordpress.org/rest-api/reference/wp-site-health-tests/) endpoint
- [Template Parts](https://developer.wordpress.org/rest-api/reference/wp_template_parts/) endpoint
- [Template Revisions](https://developer.wordpress.org/rest-api/reference/wp_template-revisions/) endpoint
- [Template Autosaves](https://developer.wordpress.org/rest-api/reference/wp_template-revisions/) endpoint
- [Template Part Autosaves](https://developer.wordpress.org/rest-api/reference/wp_template_part-revisions/) endpoint
- [Template Part Revisions](https://developer.wordpress.org/rest-api/reference/wp_template_part-revisions/) endpoint
- WordPress.com Publicize endpoints (`/sites/<site>/publicize/connections` and `/sites/<site>/publicize/services`) for listing, creating, updating, and deleting Jetpack Social connections
- WordPress.com `/me/connections` (keyring) endpoint for listing third-party OAuth connections used by Jetpack Social
- WordPress.com `/products` endpoint
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
