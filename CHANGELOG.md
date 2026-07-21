# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Publish the Kotlin bindings' per-endpoint Markdown API reference as an `ai-docs` Maven classifier zip on `rs.wordpress.api:kotlin`, generated from the UniFFI bindings for agent/tooling consumption

### Changed

- **Internal:** Build the Android JNI libraries with `cargo-ndk` instead of the `rust-android-gradle` Gradle plugin.

## [0.6.0] - 2026-07-16

### Added

- WordPress.com `GET /sites/<site_id>/stats/visits` now supports the `start_date` and `stat_fields` query parameters. `StatsVisitsParams` gained a `start_date: Option<String>` field and a `stat_fields: Vec<StatsVisitsField>` field, where `StatsVisitsField` is a new enum (`Views`, `Visitors`, `Likes`, `Reblogs`, `Comments`, `Posts`). An empty `stat_fields` omits the parameter, letting the API return its default set of fields.

### Changed

- Updated uniffi-rs to 0.32.0.
- Kotlin: `WpHttpClient.DefaultHttpClient` now applies more forgiving default OkHttp timeouts (connect 15s, read 60s, write 60s) instead of relying on OkHttp's 10s per-operation defaults, preventing premature timeouts when fetching large or slow-to-render responses. The values are configurable via a new `HttpClientTimeouts` type, accepted by both `DefaultHttpClient` and the `WpRequestExecutor` convenience constructor so callers can override them without building an `OkHttpClient` by hand.
- Posts and media are now fetched by ID in batches of `5` instead of `100`, so sites that can't render a large batch within the request timeout can still sync (at the cost of more, smaller requests).
- Swift: `WordPressAPI.uploadMedia` and `WpService.uploadMedia` now accept a `Progress` whose total is zero, defaulting the total to the upload file's byte size.
- Self-hosted login and endpoint requests now work against sites with plain permalinks. Previously, the client path-extended the discovered `?rest_route=/` API root, producing URLs like `…/index.php/wp/v2/users/me?rest_route=/` that WordPress collapsed to the API index ([#1366](https://github.com/Automattic/wordpress-rs/issues/1366)).
- **Internal:** Hardened the Claude review GitHub Actions workflow with scoped permissions and gated triggers.
- **Internal:** Added a `buildkite-triage` Claude skill for pulling failing tests from a Buildkite build via the Buildkite MCP.
- **Internal:** Quarantined the external login-discovery integration tests (`test_login_remote`) behind `#[ignore]`; they now run in a dedicated soft-fail Buildkite step so intermittent `*.wpmt.co` timeouts no longer fail the build.
- **Internal:** Added a scheduled Buildkite health digest for the quarantined `test_login_remote` tests that reads the soft-fail step's real pass/fail from the Buildkite API, separates external `*.wpmt.co` timeouts from genuine failures, and posts a summary to Slack.
- **Internal:** Updated `aes-gcm` from `0.10` to `0.11`, migrating the password encryption transformer to the `aead` 0.6 API. The AES-256-GCM scheme and stored `salt:nonce:ciphertext` format are unchanged.
- **Internal:** Bumped the CI Rust toolchain from `1.90.0` to `1.97.0` and added a pinned `rust-toolchain.toml` so local and CI compilers can't drift, fixed the clippy lints the newer toolchain surfaced (`iter_kv_map`, `manual_filter`, `useless_borrows_in_formatting`), and added a nightly Buildkite check that nudges Slack when the pin falls behind stable ([#1436](https://github.com/Automattic/wordpress-rs/issues/1436)).
- **Internal:** Bumped the pinned CI Rust toolchain from `1.97.0` to `1.97.1` (latest stable) in lockstep across `rust-toolchain.toml`, the `Makefile`, and `wp_rs_web/Dockerfile`. No new clippy lints surfaced ([#1436](https://github.com/Automattic/wordpress-rs/issues/1436)).

## [0.5.0] - 2026-06-18

### Added

- WordPress.com `POST /me/shopping-cart` endpoint for creating shopping carts with domain and plan products
- WordPress.com `GET /sites/<site_id>/domains` endpoint for fetching all domains associated with a site
- WordPress.com REST API implementation checklist (`WPCOM_REST_API_CHECKLIST.md`)
- WordPress.com `GET /me/transactions/supported-countries` endpoint for listing countries supported in payment transactions
- WordPress.com `GET /me/domain-contact-information` endpoint for fetching WHOIS/domain contact details
- Kotlin: `WpRequestResult.toLogErrorString()` returns a concise, log-only description of a failed request (or `null` on success) for diagnostics
- Kotlin: `WpComApiClient`, `WpApiClient`, and `JetpackApiClient` accept an optional `errorLogger: RequestErrorLogger` constructor parameter. When provided, failed requests are reported to it via `toLogErrorString()`, letting callers route errors to their own logger or crash reporter. No logging happens when no logger is configured.

### Changed
- **BREAKING:** `AllDomainItem.expiry` changed from `Option<WpGmtDateTime>` to `Option<WpDateString>`. The `/all-domains` endpoint returns date-only values (`"YYYY-MM-DD"`), which failed to deserialize as a datetime; callers now receive the raw `"YYYY-MM-DD"` string instead of a timestamp.
- **BREAKING:** `product_slug` and `billing_product_slug` fields on `Product`, `PaidDomainSuggestion`, `DomainPricing`, `WPComPlan`, and `WPComProduct` changed from `String` to `ProductSlug`. Callers that match on or construct these values will need to wrap/unwrap with `ProductSlug(...)`.
- Publish releases automatically on version-bump in the changelog file
- **Internal:** Route Dependabot Ruby dependency reviews through `CODEOWNERS` instead of the retired `reviewers` key.
- **Internal:** Extend CODEOWNERS Ruby/Apps-Infra review routing to CI config and toolchain pins.
- Pinned third-party GitHub Actions to commit SHA to mitigate supply-chain vulnerabilities

### Fixed

- Fixed a typo in a public doc comment (`maintainance` → `maintenance`)
- Fixed a crash when using an invalid token in `WpComApiClient`

## [0.4.0] - 2026-05-29

### Added

- `MediaService.create_media` for uploading media, with a Swift `WpService.uploadMedia` wrapper that streams upload progress
- `MediaService.update_media` for editing media items
- Sort/ordering predicates on `MediaListFilter`
- Live membership updates on `MediaMetadataCollectionWithEditContext`, so cached media lists reflect items moving in or out of the filter without a full refresh
- WordPress.com `/domains/{name}/is-available` endpoint for checking domain availability, pricing, and transfer status
- WordPress.com `/all-domains` endpoint for listing all domains across a user's sites
- WordPress.com `/mobile-support/unified-conversations` endpoints for listing, fetching, and replying to unified support conversations, with attachment uploads and encrypted log IDs supported when replying

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
