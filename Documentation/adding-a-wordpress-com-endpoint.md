# Adding a WordPress.com endpoint

The guide in `.claude/skills/wp-api-endpoints/` covers the WordPress REST API, which is the same on every site and has a test site in Docker. WordPress.com's own endpoints (`rest/v1.x` and `wpcom/v2`) differ: they come in versions, a value can change type from one post to the next, and they can only be tested against the live API.

This is the recipe the Reader endpoints were built with. The worked examples are `wp_api/src/wp_com/reader_saved.rs` (requests that write) and `wp_api/src/wp_com/reader_posts.rs` (streams of posts), each with its endpoint file in `wp_api/src/wp_com/endpoint/`.

## 1. Find out what the endpoint does

- **Use the newest version that does the job.** Ask each version (`rest/v1.3`, `v1.2`, `v1.1`) and compare the answers. Say in the doc comment why an older one was chosen, when it was.
- **Find its parameters** from public sources:
  - `https://public-api.wordpress.com/rest/v1.2/help` documents the `rest/v1.x` endpoints.
  - `https://public-api.wordpress.com/wpcom/v2/` lists the `wpcom/v2` routes with their arguments. It leaves some routes out.
  - [Calypso](https://github.com/Automattic/wp-calypso) calls most of them: see `packages/api-core/src/`.
  - The WordPress apps for iOS and Android show which parameters the apps send.
- **Record every parameter the endpoint takes** in the params type, each with a one-line comment that says what it does, from what you saw it do. One that had no effect you could see still goes in, with a comment that says so.
- **The live API is the final word.** Fetch many responses before modelling one. Try the parameter both ways when testing whether it does anything, and compare the posts, not the size: an identical answer can be a cached one.
- **Check whether it answers with no token.** The Reader works signed out, so each request's doc comment says whether it needs an account.

## 2. Model the response

`CONVENTIONS.md` has the rules for types and dates. On top of those:

- **Model from many recorded responses, not one.** A field can be a number in one post and a string in the next, or `false`, `""`, `0` or missing where an object is expected. Each must still parse: prefer `Option` to failing a page. `wp_serde_helper` has the deserializers (`deserialize_u64_or_string_or_none` and its relatives).
- **One item that can't be read doesn't fail its page.** A list of posts or comments is read through `LenientVec`, and the response lists the others, as `ReaderPostsResponse::unreadable_posts` does.
- **Reuse a type the API reuses.** A post is a `ReaderPost` wherever the API sends that shape. Don't add a second type for a shape that is nearly the same without a reason you can state.
- **IDs get their own types**: `WpComSiteId`, `ReaderFeedId`, `ReaderFeedItemId`, `WpComUserId`. Add one when a new kind of ID appears.
- **Optional fields get `#[uniffi(default = None)]`**, so the apps can build the type without naming every field.
- **Keep what you fetch out of the repo.** Responses hold private posts and the account's own data. Only anonymized fixtures are committed, under `wp_api/tests/wpcom/`: made-up names, addresses and content, with the types the API sent. A number sent as a string stays a string.

## 3. Write the request

- The requests of a group are an enum in `wp_api/src/wp_com/endpoint/<group>_endpoint.rs` that derives `WpDerivedRequest`, with a `#[get]` or `#[post]` on each variant, and a `DerivedRequest` impl that gives each variant its namespace (`WpComNamespace::RestV1_2`, `V2`, …).
- Query parameters are a struct that derives `WpDeriveParamsField`. It needs `#[supports_pagination(false)]` unless it pages by page number, and `#[field_name("orderBy")]` on a field whose name on the wire isn't its own.
- A new group is registered in three files, each next to `reader_saved`: `wp_api/src/wp_com/mod.rs`, `wp_api/src/wp_com/endpoint.rs`, and `wp_api/src/wp_com/client.rs` (the import, the builder field, the executor field, both constructors, and `api_client_generate_endpoint_impl!`).

Three things that have gone wrong:

- **A derived `#[post]` sends its parameters in the body.** A `rest/v1.x` endpoint reads a parameter it declares for the query from the address only, so such a parameter can't be modelled as a body field.
- **Variants that all start with the same word fail clippy**, and the derive doesn't accept an `#[allow]` on the enum. Name them so they don't (`SitePostsSeen`, `FeedItemsSeen`).
- **A variant named `List`, `Set` or `Map` breaks the Kotlin bindings** when another variant of the same enum has a `Vec` field: inside the generated class, the name means the variant. Only the Kotlin compile shows it.

## 4. Test it

- **Unit tests, next to the code.** Each fixture parses, with a few fields asserted. Each request's address is checked with no parameters and with every parameter. Each odd shape found live has a test: an empty list, a missing field, a string for a number.
- **A live trial in `wp_com_e2e`**, in the style of `wp_com_e2e/src/reader_posts_tests.rs`, against public data that will still be there next year. The WordPress.com Blog is site `3584907`. Run one with:

  ```bash
  WP_COM_API_KEY=… cargo run -q -p wp_com_e2e --bin wp_com_e2e -- <one filter>
  ```

- **A trial that writes puts back what it changed**, whatever happened in between. One that writes something public, such as a comment, runs only against a site the account owns, named in `WP_COM_E2E_OWN_SITE_ID`, and is skipped without it.
- **A stream of posts is added to `reader_sweep`**, which reads a large amount of real data through the library and reports every response and post it couldn't read. Its module comment says how to run it.
- `cargo test -p wp_derive_request_builder` if the change touched how requests are derived: those tests aren't run by `cargo test -p wp_api`.

## 5. Check the bindings

A type that Rust exports isn't usable from Swift until it's re-exported, and Kotlin compiles code that Swift never sees.

- **Swift.** Every new record, enum and object needs a `public typealias` in `native/swift/Sources/wordpress-api/Exports.swift`, and a new group needs an accessor in `WPComApiClient.swift`. Then:

  ```bash
  make xcframework-only-macos
  BUILDKITE=1 swift test
  xcrun swift format lint --strict --recursive native/swift/Sources/wordpress-api
  ```

- **Kotlin.**

  ```bash
  cd native/kotlin && ./gradlew :api:kotlin:compileKotlin :api:kotlin:detektMain
  ```

## 6. Before review

- `cargo fmt --all -- --check`
- `cargo clippy --tests --all-targets --all-features -- -D warnings`
- `cargo test -p wp_api --lib`
- An entry in `CHANGELOG.md` under `## [Unreleased]`: CI fails a pull request without one.
- The endpoint's line ticked in `WPCOM_REST_API_CHECKLIST.md`.
- The pull request says, for each endpoint: the version used and why, whether it answers signed out, how it pages (by date, page handle, offset or page number, and the largest page it returns), and what about the live API was a surprise.
