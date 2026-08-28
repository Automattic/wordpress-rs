use crate::parsed_url::{ParsedUrl, QueryPair};
use std::sync::Arc;
use url::Url;

/// A REST URL resolved against a specific site, retaining enough context to emit
/// both halves a preloading editor needs: the request URL to fetch, and the
/// canonical, origin-less route key that `@wordpress/api-fetch`'s preload
/// middleware matches on.
///
/// `ApiUrlResolver::resolve` returns this instead of a bare [`ParsedUrl`]. The
/// request URL and the cache key are two projections of the same resolved
/// endpoint: [`ResolvedUrl::url`] for the request, and
/// [`ResolvedUrl::canonical_route_key`] for the preload key.
#[derive(Debug, Clone, uniffi::Object)]
pub struct ResolvedUrl {
    /// Request URL in the site's advertised form (`…/wp-json/…` or
    /// `…?rest_route=…`). This is the URL to actually fetch.
    inner: Url,
    /// The API root this was resolved against (`…/wp-json`, `…/index.php?rest_route=/`,
    /// or a WordPress.com base). Retained as the resolution context so callers
    /// never have to pass the root back in to derive the key.
    api_root: Arc<ParsedUrl>,
    /// The canonical, origin-less REST route path (e.g. `/wp/v2/themes`),
    /// memoized at resolve time so [`canonical_route_key`](Self::canonical_route_key)
    /// is a pure concatenation and is guaranteed to match `route_path()`.
    route_path: String,
}

#[uniffi::export]
impl ResolvedUrl {
    /// Assembles a `ResolvedUrl` from its parts.
    ///
    /// Exported so foreign [`ApiUrlResolver`](crate::request::endpoint::ApiUrlResolver)
    /// implementations can build the value their `resolve` returns. `url` is the
    /// request URL, `api_root` is the root it was resolved against, and
    /// `route_path` is the canonical route path the same resolver's
    /// `route_path()` would produce for these inputs (leading slash, real
    /// slashes, no trailing slash).
    #[uniffi::constructor]
    pub fn new(url: Arc<ParsedUrl>, api_root: Arc<ParsedUrl>, route_path: String) -> Arc<Self> {
        Arc::new(Self {
            inner: url.inner.clone(),
            api_root,
            route_path,
        })
    }

    /// The request URL to actually fetch, as a string. Mirrors
    /// [`ParsedUrl::url`], so a `resolve(...).url()` call site keeps returning
    /// the same string it did when `resolve` returned a `ParsedUrl`.
    pub fn url(&self) -> String {
        self.inner.to_string()
    }

    /// The request URL to actually fetch, as a [`ParsedUrl`] — the escape hatch
    /// for anywhere a `ParsedUrl` is needed (further query edits, comparisons).
    pub fn parsed_url(&self) -> Arc<ParsedUrl> {
        Arc::new(ParsedUrl::new(self.inner.clone()))
    }

    /// A copy with `pairs` appended to the request URL's query, preserving the
    /// resolution context. The route path is unchanged by query parameters, so
    /// the canonical route key gains the same pairs but keeps its path.
    ///
    /// Delegates to [`ParsedUrl::by_appending_query_pairs`] so the encoding
    /// matches the request URL exactly.
    pub fn by_appending_query_pairs(&self, pairs: Vec<QueryPair>) -> Arc<ResolvedUrl> {
        let appended = ParsedUrl::new(self.inner.clone()).by_appending_query_pairs(pairs);
        Arc::new(Self {
            inner: appended.inner.clone(),
            api_root: self.api_root.clone(),
            route_path: self.route_path.clone(),
        })
    }

    /// The origin-less canonical `path[?query]` that `@wordpress/api-fetch`'s
    /// preload middleware matches on.
    ///
    /// `api-fetch` normalizes both an outgoing request and each preload key
    /// through the same origin-less function: it keys on path + query, unwraps
    /// the `?rest_route=` form back to the canonical `/wp/v2/…` path, and early
    /// returns when there is no query. So the key is the canonical route path
    /// plus the endpoint query params (everything except `rest_route`), and a
    /// query-less endpoint is just the bare path — no trailing `?`.
    ///
    /// INVARIANT: for the same inputs, this is byte-identical on pretty
    /// (`…/wp-json/…`) and plain (`…?rest_route=…`) permalink sites. The path
    /// comes from the memoized `route_path`; the query is re-serialized from the
    /// request URL with `rest_route` dropped, which is the only per-form
    /// difference.
    pub fn canonical_route_key(&self) -> String {
        // Rebuild the query from the request URL's own pairs, dropping the
        // `rest_route` param that only plain-permalink roots carry. Re-encoding
        // through `query_pairs_mut` reuses the exact form-encoding the request
        // URL was built with, so the key's query is byte-for-byte what the
        // request carries (minus `rest_route`).
        let retained: Vec<(String, String)> = self
            .inner
            .query_pairs()
            .filter_map(|(name, value)| {
                (name != "rest_route").then(|| (name.into_owned(), value.into_owned()))
            })
            .collect();

        if retained.is_empty() {
            return self.route_path.clone();
        }

        let mut scratch = self.inner.clone();
        scratch.set_query(None);
        {
            let mut query_pairs = scratch.query_pairs_mut();
            for (name, value) in &retained {
                query_pairs.append_pair(name, value);
            }
        }

        match scratch.query() {
            Some(query) => format!("{}?{}", self.route_path, query),
            None => self.route_path.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    /// Builds a `ResolvedUrl` the way a resolver would, for exercising the
    /// projections in isolation from `ApiUrlResolver::resolve`.
    fn resolved(request_url: &str, api_root: &str, route_path: &str) -> Arc<ResolvedUrl> {
        ResolvedUrl::new(
            ParsedUrl::parse(request_url)
                .expect("valid request url")
                .into(),
            ParsedUrl::parse(api_root).expect("valid api root").into(),
            route_path.to_string(),
        )
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

    #[test]
    fn url_and_parsed_url_return_the_request_url() {
        let resolved = resolved(
            "https://example.com/wp-json/wp/v2/themes?context=edit",
            "https://example.com/wp-json",
            "/wp/v2/themes",
        );
        // `url()` mirrors `ParsedUrl::url()` — the URL string.
        assert_eq!(
            resolved.url(),
            "https://example.com/wp-json/wp/v2/themes?context=edit"
        );
        // `parsed_url()` is the escape hatch to the `ParsedUrl` object; its own
        // `url()` returns the same string.
        assert_eq!(resolved.parsed_url().url(), resolved.url());
    }

    #[test]
    fn appending_query_pairs_preserves_route_path_and_extends_the_request_url() {
        let resolved = resolved(
            "https://example.com/wp-json/wp/v2/themes",
            "https://example.com/wp-json",
            "/wp/v2/themes",
        );
        let appended = resolved
            .by_appending_query_pairs(query_pairs(&[("context", "edit"), ("status", "active")]));
        assert_eq!(
            appended.url(),
            "https://example.com/wp-json/wp/v2/themes?context=edit&status=active"
        );
        assert_eq!(
            appended.canonical_route_key(),
            "/wp/v2/themes?context=edit&status=active"
        );
    }

    /// A query-less resolved URL keys on the bare path — no trailing `?`.
    #[rstest]
    #[case::pretty("https://example.com/wp-json/wp/v2/themes")]
    #[case::plain("https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes")]
    fn canonical_route_key_without_query_is_the_bare_path(#[case] request_url: &str) {
        let resolved = resolved(request_url, "https://example.com/wp-json", "/wp/v2/themes");
        assert_eq!(resolved.canonical_route_key(), "/wp/v2/themes");
    }

    /// The crux: pretty and plain request URLs for the same endpoint produce a
    /// byte-identical key. `rest_route` is dropped; the remaining pairs re-encode
    /// identically.
    #[rstest]
    #[case::two_pairs(
        "https://example.com/wp-json/wp/v2/themes?context=edit&status=active",
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes&context=edit&status=active",
        "/wp/v2/themes?context=edit&status=active"
    )]
    #[case::encoded_value(
        "https://example.com/wp-json/wp/v2/themes?exclude=core%2Cgutenberg",
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes&exclude=core%2Cgutenberg",
        "/wp/v2/themes?exclude=core%2Cgutenberg"
    )]
    fn canonical_route_key_is_identical_across_permalink_forms(
        #[case] pretty_url: &str,
        #[case] plain_url: &str,
        #[case] expected_key: &str,
    ) {
        let pretty = resolved(pretty_url, "https://example.com/wp-json", "/wp/v2/themes");
        let plain = resolved(
            plain_url,
            "https://example.com/index.php?rest_route=/",
            "/wp/v2/themes",
        );
        assert_eq!(pretty.canonical_route_key(), expected_key);
        assert_eq!(plain.canonical_route_key(), expected_key);
    }
}
