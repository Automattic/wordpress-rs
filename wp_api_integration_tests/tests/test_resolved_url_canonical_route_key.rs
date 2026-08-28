//! End-to-end invariant for [`ResolvedUrl::canonical_route_key`]: for a given
//! endpoint, the origin-less route key `@wordpress/api-fetch`'s preload
//! middleware matches on is **byte-identical** whether the site advertises
//! pretty permalinks (`…/wp-json/…`) or plain permalinks (`…/index.php?rest_route=…`).
//!
//! This is the property GutenbergKit relies on: wprs precomputes the preload
//! key natively via `resolve(...).canonical_route_key()`, and it must match the
//! request api-fetch issues regardless of the site's permalink setting. The two
//! `WpOrgSiteApiUrlResolver`s below are seeded with the exact api-root forms a
//! self-hosted site advertises for each permalink setting, so this exercises the
//! same resolver path the client uses after discovery — no live server needed,
//! because the key is permalink-config-independent by construction and that is
//! precisely what is under test.

use wp_api::{
    parsed_url::{ParsedUrl, QueryPair},
    request::endpoint::{ApiUrlResolver, WpOrgSiteApiUrlResolver},
};
use wp_api_integration_tests::prelude::rstest;

/// The two api-root forms the same self-hosted site advertises: pretty
/// permalinks path-extend `…/wp-json`, plain permalinks extend the `rest_route`
/// query of `…/index.php?rest_route=/`.
const PRETTY_API_ROOT: &str = "https://example.com/wp-json";
const PLAIN_API_ROOT: &str = "https://example.com/index.php?rest_route=/";

fn resolver(api_root: &str) -> WpOrgSiteApiUrlResolver {
    WpOrgSiteApiUrlResolver::new(ParsedUrl::parse(api_root).expect("valid api root").into())
}

fn query_pairs(pairs: &[(&str, &str)]) -> Vec<QueryPair> {
    pairs
        .iter()
        .map(|(name, value)| QueryPair {
            name: name.to_string(),
            value: value.to_string(),
        })
        .collect()
}

/// Real endpoint route shapes: a collection, a nested singleton, an id-bearing
/// resource, a route with embedded slashes, and non-`wp/v2` namespaces.
#[rstest]
#[case::themes_collection("/wp/v2", vec!["themes"], "/wp/v2/themes")]
#[case::users_me("/wp/v2", vec!["users", "me"], "/wp/v2/users/me")]
#[case::posts_by_id("/wp/v2", vec!["posts", "123"], "/wp/v2/posts/123")]
#[case::types_post("/wp/v2", vec!["types", "post"], "/wp/v2/types/post")]
#[case::block_renderer_embedded_slash(
    "/wp-block-editor/v1",
    vec!["block-renderer", "core/paragraph"],
    "/wp-block-editor/v1/block-renderer/core/paragraph"
)]
#[case::site_health_nested(
    "/wp-site-health/v1",
    vec!["tests", "background"],
    "/wp-site-health/v1/tests/background"
)]
fn canonical_route_key_matches_across_permalink_forms(
    #[case] namespace: &str,
    #[case] segments: Vec<&str>,
    #[case] expected_key: &str,
) {
    let pretty = resolver(PRETTY_API_ROOT);
    let plain = resolver(PLAIN_API_ROOT);
    let segments: Vec<String> = segments.into_iter().map(str::to_string).collect();

    // Without a query, the key is the bare canonical path — no trailing `?`.
    let pretty_bare = pretty.resolve(namespace.to_string(), segments.clone());
    let plain_bare = plain.resolve(namespace.to_string(), segments.clone());
    assert_eq!(pretty_bare.canonical_route_key(), expected_key);
    assert_eq!(
        plain_bare.canonical_route_key(),
        pretty_bare.canonical_route_key(),
        "plain and pretty permalink keys diverged for {expected_key}"
    );

    // With endpoint query params, both forms gain the same query in the key even
    // though only the plain request URL also carries `rest_route`.
    let pairs = query_pairs(&[("context", "edit"), ("status", "active")]);
    let pretty_keyed = pretty
        .resolve(namespace.to_string(), segments.clone())
        .by_appending_query_pairs(pairs.clone());
    let plain_keyed = plain
        .resolve(namespace.to_string(), segments)
        .by_appending_query_pairs(pairs);
    let expected_keyed = format!("{expected_key}?context=edit&status=active");
    assert_eq!(pretty_keyed.canonical_route_key(), expected_keyed);
    assert_eq!(
        plain_keyed.canonical_route_key(),
        pretty_keyed.canonical_route_key(),
        "plain and pretty permalink keys diverged for {expected_keyed}"
    );
}
