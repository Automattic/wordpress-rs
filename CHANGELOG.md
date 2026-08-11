# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- REST URL resolution can now attach endpoint query parameters via `ParsedUrl.by_appending_query_pairs` (Swift/Kotlin), so consumers building `?rest_route=` URLs no longer re-implement the `?`→`&` merge. It preserves any existing query, is order-stable, keeps duplicate keys, and form-urlencodes names and values — the same encoding `WpOrgSiteApiUrlResolver.resolve` already produces. Pairs are passed as the new `QueryPair` record ([#1543](https://github.com/Automattic/wordpress-rs/issues/1543)).
- WordPress.com `POST /me/transactions` endpoint for redeeming a shopping cart with the account's WordPress.com credits, completing a domain purchase
- WordPress.com `GET /sites/<site_id>/purchases` endpoint for listing a site's purchases (plans, domains, and other subscriptions)
- Publish the Kotlin bindings' per-endpoint Markdown API reference as an `ai-docs` Maven classifier zip on `rs.wordpress.api:kotlin`, generated from the UniFFI bindings for agent/tooling consumption
- WordPress.com `POST /sites/<site_id>/domains/primary` endpoint for setting a site's primary domain
- WordPress.com `GET /sites/<site_id>/stats/post/<post_id>` endpoint for a post's view history, like count, comment count, and metadata
- WordPress.com `GET /sites/<site_id>/plans` endpoint for listing the plans a site can buy, priced for that site, with the plan it's currently on flagged.
- `RequestExecutionErrorReason` gained `isSiteUnreachable` and `isDeviceOffline` for distinguishing a site that could not be reached (the host did not resolve, or a connection to it could not be established) from a device with no network connection. Previously consumers had to match the `NonExistentSiteError` / `DeviceIsOfflineError` variants themselves. Available on both platforms as properties on the reason, which is reachable from `WpRequestResult.RequestExecutionFailed` and `WpApiException.RequestExecutionFailed` on Kotlin. Swift additionally exposes both as convenience properties on `WpApiError` and `RequestExecutionError`.
- **BREAKING:** `RequestExecutionErrorReason` gained a `ConnectionError` variant — the host resolved, but no connection to the server could be established (the connection was refused, there was no route, or the host was unreachable). Exhaustive matches over `RequestExecutionErrorReason` (Swift `switch`, Kotlin `when`, Rust `match`) must now handle the new case. `isSiteUnreachable` covers it alongside DNS failures, so it's a single portable "we couldn't reach the site" signal that returns the same answer on every executor; match `NonExistentSiteError` / `ConnectionError` directly to tell a bad domain apart from a server that's down. Refused and unreachable connections previously classified as the generic `HttpError` on Kotlin and reqwest. A connect timeout is not included; it remains `HttpTimeoutError`.

### Changed

- **BREAKING:** `product_type` fields on `Product` and `WPComProduct` changed from `String` to `ProductType`. Callers that match on or construct these values will need to wrap/unwrap with `ProductType(...)`.
- **BREAKING:** The cache now enables SQLite foreign key enforcement on every connection it prepares, and fails with `SqliteDbError::ForeignKeysUnavailable` if the setting doesn't take effect. Removing a site relies on `ON DELETE CASCADE` to clear its cached rows, so on builds where enforcement defaulted to off those rows were silently left behind.
- **BREAKING:** `ShoppingCart.coupon` changed from `String` to `CouponCode`, and `ShoppingCartCostOverride.override_code` from `String` to `CostOverrideCode`, so the shopping cart and site plans describe these values with the same types. Callers will need to wrap/unwrap with `CouponCode(...)` / `CostOverrideCode(...)`.
- Documented `GET /all-domains/` subtypes and parameters. `DomainSubtypeId::DefaultAddress` covers staging and garden subdomains as well as the free WordPress.com address, and is the set v1.1's `no_wpcom=true` excluded; v1.2 has no equivalent parameter, so clients filter this subtype out instead.
- **Internal:** Corrected `GET /all-domains/` fixtures that claimed subtypes the endpoint never returns (`site_redirect`, `domain_mapping`).
- Kotlin: The request executor now classifies cancelling an in-flight request via `CancellableCall.cancel()` as `CancellationError` instead of `GenericError`, matching Swift's handling of `URLError.cancelled`. Whole-call `callTimeout` expiry is classified as `HttpTimeoutError` rather than being mistaken for a cancellation, and a `CancellationException` surfacing synchronously inside the executor (e.g. from an upload callback) is classified as `CancellationError` rather than flattened into a `GenericError` ([#1492](https://github.com/Automattic/wordpress-rs/issues/1492)).
- **Internal:** Run clippy's `--tests` lint pass with `--jobs 1` to cap the Rust lint step's peak memory at ~4.7 GiB (down from ~9 GiB) on CI.
- **Internal:** Build the Android JNI libraries with `cargo-ndk` instead of the `rust-android-gradle` Gradle plugin.
- **Internal:** Upgraded the Android/Kotlin build to Android Gradle Plugin `9.3.0` / Gradle `9.5.0` (Kotlin `2.3.21`, `compileSdk` 36), migrating `api/android` to the AGP 9 variant APIs and splitting the example app into a `com.android.kotlin.multiplatform.library` shared module and a standalone `com.android.application` module.
- **Internal:** Bumped `syn` from `2.0` to `3.0`, updating the proc-macro crates for its breaking changes.
- **Internal:** Use Xcode 26.6 on CI.
- **Internal:** Use Ruby 3.4.9 for automation tooling.
- **Internal:** Fix a flaky unit test.
- **Internal:** Route the Kotlin integration tests through the in-VPC Reposilite dependency mirror
- **Internal:** Merge `CHANGELOG.md` with git's built-in `union` driver (via `.gitattributes`), so the frequent conflicts on the `## [Unreleased]` section resolve by keeping both sides instead of emitting conflict markers.
- **Internal:** Documented how to pick an xcframework build for local work, and added `make help` descriptions for the `xcframework-only-*` targets. Verifying a UniFFI change needs only `make xcframework-only-macos`, not the full 11-target `make xcframework`.
- **Internal:** Fixed `make xcframework-only-<platform>` building the per-target libraries but never assembling them into the xcframework. The `@# Help:` comments added for those `make help` descriptions became each rule's recipe and silently shadowed the shared `xcframework-only-%` pattern rule that ran the assemble step; each rule now runs the assemble step directly.
- **Internal:** Update translations.
- **Internal:** Added a golden URL table (`plain_permalinks_url_tests`) asserting all 45 self-hosted endpoints resolve to the correct `?rest_route=` URL on a plain-permalinks site, by driving each endpoint's real URL builder through a `rest_route`-seeded `WpOrgSiteApiUrlResolver`. To guard the table's accuracy against an independent source, the 22 non-parameterized routes are also checked (`index_self_href_url_tests`) against WordPress's own published URL — the `_links.self.href` captured in a real-site REST index fixture — and the parameterized (ID-bearing) routes, which the index never publishes a URL for, are exercised end-to-end by a new plain-permalinks integration test that fetches a real object by numeric id over `?rest_route=`.

### Fixed

- Swift: `WpRequestExecutor` classifies a URLSession timeout (`URLError.timedOut`) as `HttpTimeoutError` instead of the catch-all `GenericError`. The timeout had no branch in the executor's error dispatch, so `HttpTimeoutError` was unreachable on Apple platforms and a caller wanting "retry on timeout" had to match `GenericError`, which also covers unrelated failures. This brings Swift to parity with reqwest (`is_timeout()`) and Kotlin (`SocketTimeoutException`). ([#1491](https://github.com/Automattic/wordpress-rs/issues/1491))
- Swift: `WpRequestExecutor.sleep(millis:)` converted milliseconds to nanoseconds with the wrong factor (`* 1_000` instead of `* 1_000_000`), so it slept 1000× too short — a `Retry-After: 30` waited 30 ms instead of 30 s. `RetryAfterMiddleware` then re-sent immediately, the server kept returning 429, and after `max_retries` the caller observed `MisconfiguredRateLimitError` where honoring the backoff would usually have succeeded. The executor now waits the full interval, and no longer risks a `fatalError` if the sleep's task is cancelled.
- Swift: Classify invalid-SSL failures from the failed handshake's `SecTrust` (`URLError.failureURLPeerTrust`), via `SecTrustCopyCertificateChain`, instead of reading the undocumented `NSErrorPeerCertificateChainKey` `userInfo` string that has no public constant. Behavior is unchanged on every platform: iOS/macOS/tvOS still surface the presented certificate as `certificateNotValidForName`, and watchOS — which exposes no peer trust — still degrades to `genericSslError`. ([#1510](https://github.com/Automattic/wordpress-rs/issues/1510))
- `isSiteUnreachable` now returns the same answer for a refused connection — the host resolves, but nothing is listening (server down, wrong port) — on every executor. Previously it was `NonExistentSiteError` on Swift (so `isSiteUnreachable` was `true`) but the generic `HttpError` on Kotlin and reqwest (so it was `false`); a refused connection is now a `ConnectionError` everywhere, which `isSiteUnreachable` covers. `NonExistentSiteError` is reserved for a DNS-resolution failure.

### Security

- **Internal:** Upgraded `reqwest` from `0.12` to `0.13` and removed the direct `hickory-resolver` dependency, moving `hickory-proto` to `0.26.1` to clear [RUSTSEC-2026-0119](https://rustsec.org/advisories/RUSTSEC-2026-0119.html) ([`GHSA-q2qq-hmj6-3wpp`](https://github.com/hickory-dns/hickory-dns/security/advisories/GHSA-q2qq-hmj6-3wpp)), a medium-severity O(n²) CPU-exhaustion DoS in DNS message encoding. Only the Rust `reqwest` request executor — used by the CLI, web tool, and integration tests — pulls in `hickory`; the shipping iOS/Android bindings don't compile `reqwest`, so they were never affected. `reqwest` 0.13 also drops the `native-tls`/`openssl` stack in favor of rustls-only, so the executor no longer links OpenSSL.
- **Internal:** Bumped the transitive `rand` dependency to `0.9.3` and `0.8.6` to clear [RUSTSEC-2026-0097](https://rustsec.org/advisories/RUSTSEC-2026-0097.html) (`GHSA-cq8v-f236-94qc`), a low-severity unsoundness in `rand` 0.9.2 / 0.8.5. Lockfile-only; the affected code path (a custom `log` logger calling `rand::rng()` during reseed) is not exercised here.
- **Internal:** Bumped `wp_rs_web`'s transitive `nanoid` dependency from `3.3.16` to `3.3.18` to clear [CVE-2026-67213](https://nvd.nist.gov/vuln/detail/CVE-2026-67213) ([GHSA-2v37-7h3g-55p8](https://github.com/advisories/GHSA-2v37-7h3g-55p8)), a denial-of-service via an infinite loop in `nanoid`'s `customAlphabet`/`customRandom` when called with a size of `0`. Lockfile-only; `nanoid` is pulled in only by Tailwind's build-time `postcss`, which never reaches the affected functions.

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
