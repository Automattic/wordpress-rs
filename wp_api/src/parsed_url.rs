use std::fmt;
use std::fmt::Display;
use std::sync::Arc;
use url::Url;
use wp_localization::{MessageBundle, WpMessages, WpSupportsLocalization};
use wp_localization_macro::WpDeriveLocalizable;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, uniffi::Object)]
#[uniffi::export(Eq, Hash)]
pub struct ParsedUrl {
    pub inner: Url,
}

impl ParsedUrl {
    pub fn new(url: Url) -> Self {
        Self { inner: url }
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    pub fn by_extending_and_splitting_by_forward_slash<I>(&self, segments: I) -> Url
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.inner
            .clone()
            .extend(segments.into_iter().flat_map(|s| {
                s.as_ref()
                    .split('/')
                    .filter_map(|x| match x.trim() {
                        "" => None,
                        y => Some(y.to_string()),
                    })
                    .collect::<Vec<String>>()
            }))
            .expect("ParsedUrl is already parsed, so this can't result in an error")
    }

    /// Extend an API-root URL with REST API path segments, preserving the form
    /// the site advertises. On plain-permalink sites WordPress advertises the
    /// query-parameter form (`…/index.php?rest_route=/`); for those we append
    /// into the `rest_route` value rather than the path. Otherwise we extend
    /// the path as before.
    pub fn by_extending_rest_api_path<I>(&self, segments: I) -> Url
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let appended_path = segments
            .into_iter()
            .flat_map(|s| {
                s.as_ref()
                    .split('/')
                    .filter_map(|x| match x.trim() {
                        "" => None,
                        y => Some(y.to_string()),
                    })
                    .collect::<Vec<String>>()
            })
            .collect::<Vec<_>>()
            .join("/");

        if self.inner.query_pairs().any(|(k, _)| k == "rest_route") {
            let pairs: Vec<(String, String)> = self
                .inner
                .query_pairs()
                .map(|(k, v)| {
                    if k == "rest_route" {
                        let base = v.trim_end_matches('/');
                        let new_value = if appended_path.is_empty() {
                            v.into_owned()
                        } else {
                            format!("{base}/{appended_path}")
                        };
                        (k.into_owned(), new_value)
                    } else {
                        (k.into_owned(), v.into_owned())
                    }
                })
                .collect();

            let mut url = self.inner.clone();
            url.query_pairs_mut().clear();
            for (k, v) in pairs {
                url.query_pairs_mut().append_pair(&k, &v);
            }
            return url;
        }

        self.by_extending_and_splitting_by_forward_slash([appended_path])
    }
}

impl Display for ParsedUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

trait UrlExtension {
    fn extend<I>(self, segments: I) -> Result<Url, ()>
    where
        I: IntoIterator,
        I::Item: AsRef<str>;
}

impl UrlExtension for Url {
    fn extend<I>(mut self, segments: I) -> Result<Url, ()>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        // Drop the trailing slash, so that `foo/` and `bar` turn into `foo/bar` instead of `foo//bar`.
        if let Some(mut segments) = self.path_segments()
            && segments.next_back() == Some("")
        {
            self.path_segments_mut()?.pop();
        }

        self.path_segments_mut()?.extend(segments);
        Ok(self)
    }
}

/// A single `name=value` query parameter, used to attach endpoint query
/// parameters to a resolved REST URL across the UniFFI boundary.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct QueryPair {
    pub name: String,
    pub value: String,
}

#[uniffi::export]
impl ParsedUrl {
    #[uniffi::constructor]
    pub fn parse(input: &str) -> Result<Self, ParseUrlError> {
        Url::parse(input)
            .map(Self::new)
            .map_err(ParseUrlError::from)
    }

    pub fn url(&self) -> String {
        self.inner.to_string()
    }

    /// Returns a copy of this URL with `pairs` appended to its query string,
    /// preserving any query that is already present.
    ///
    /// This works uniformly on both REST API root forms produced by
    /// `ApiUrlResolver::resolve`:
    /// - **Path root** (`…/wp-json/wp/v2/themes`) gains `?context=edit&status=active`.
    /// - **Query root** (`…/index.php?rest_route=%2Fwp%2Fv2%2Fthemes`) keeps its
    ///   existing `rest_route` value and gains `&context=edit&status=active`.
    ///
    /// Pairs are appended in order and duplicate keys are kept (not
    /// deduplicated). Names and values are serialized with
    /// `application/x-www-form-urlencoded` encoding — the same encoding
    /// `by_extending_rest_api_path` already uses — so a value like
    /// `core,gutenberg` is written as `core%2Cgutenberg`. An empty `pairs`
    /// returns the URL unchanged (no trailing `?` is added).
    pub fn by_appending_query_pairs(&self, pairs: Vec<QueryPair>) -> Arc<ParsedUrl> {
        // Returning early keeps the URL byte-for-byte identical. `query_pairs_mut()`
        // pushes a `?` as soon as it is called, even if nothing is appended, so we
        // must avoid touching it when there is nothing to add.
        if pairs.is_empty() {
            return Arc::new(self.clone());
        }

        let mut url = self.inner.clone();
        {
            let mut query_pairs = url.query_pairs_mut();
            for pair in &pairs {
                query_pairs.append_pair(&pair.name, &pair.value);
            }
        }
        Arc::new(ParsedUrl::new(url))
    }

    /// A user-facing URL string that omits unnecessary details.
    pub fn pretty_url(&self) -> String {
        if self.inner.path() == "/" {
            self.inner
                .host_str()
                .unwrap_or(self.inner.as_str())
                .to_string()
        } else {
            self.inner.to_string()
        }
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    thiserror::Error,
    uniffi::Error,
    WpDeriveLocalizable,
)]
pub enum ParseUrlError {
    Generic { reason: String },
    EmptyHost,
    IdnaError,
    InvalidPort,
    InvalidIpv4Address,
    InvalidIpv6Address,
    InvalidDomainCharacter,
    RelativeUrlWithoutBase,
    RelativeUrlWithCannotBeABaseBase,
    SetHostOnCannotBeABaseUrl,
    Overflow,
}

impl WpSupportsLocalization for ParseUrlError {
    fn message_bundle(&self) -> MessageBundle<'_> {
        WpMessages::url_parsing_error()
    }
}

impl From<url::ParseError> for ParseUrlError {
    fn from(value: url::ParseError) -> Self {
        use url::ParseError;
        match value {
            ParseError::EmptyHost => Self::EmptyHost,
            ParseError::IdnaError => Self::IdnaError,
            ParseError::InvalidPort => Self::InvalidPort,
            ParseError::InvalidIpv4Address => Self::InvalidIpv4Address,
            ParseError::InvalidIpv6Address => Self::InvalidIpv6Address,
            ParseError::InvalidDomainCharacter => Self::InvalidDomainCharacter,
            ParseError::RelativeUrlWithoutBase => Self::RelativeUrlWithoutBase,
            ParseError::RelativeUrlWithCannotBeABaseBase => Self::RelativeUrlWithCannotBeABaseBase,
            ParseError::SetHostOnCannotBeABaseUrl => Self::SetHostOnCannotBeABaseUrl,
            ParseError::Overflow => Self::Overflow,
            _ => Self::Generic {
                reason: value.to_string(),
            },
        }
    }
}

impl TryFrom<&str> for ParsedUrl {
    type Error = ParseUrlError;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        Self::parse(input)
    }
}

impl From<Url> for ParsedUrl {
    fn from(input: Url) -> Self {
        Self::new(input)
    }
}

impl From<ParsedUrl> for String {
    fn from(input: ParsedUrl) -> Self {
        input.url()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use url::Url;

    #[rstest]
    #[case("http://example.com")]
    fn parse_url_success(#[case] input: &str) {
        let parsed_url = ParsedUrl::parse(input).unwrap();
        assert_eq!(parsed_url.url(), "http://example.com/");
        assert_eq!(parsed_url, Url::parse("http://example.com").unwrap().into());
    }

    #[rstest]
    #[case("https://", ParseUrlError::EmptyHost)]
    #[case("https://example.com:foo", ParseUrlError::InvalidPort)]
    #[case("https://1.2.3.4.5", ParseUrlError::InvalidIpv4Address)]
    #[case("https://[1", ParseUrlError::InvalidIpv6Address)]
    #[case("foo://example>", ParseUrlError::InvalidDomainCharacter)]
    #[case("", ParseUrlError::RelativeUrlWithoutBase)]
    // https://www.unicode.org/reports/tr46/#Validity_Criteria
    #[case("https://xn--u-ccb.com", ParseUrlError::IdnaError)]
    fn parse_url_error(#[case] input: &str, #[case] expected_err: ParseUrlError) {
        assert_eq!(ParsedUrl::try_from(input).unwrap_err(), expected_err);
    }

    #[rstest]
    #[case("https://example.com", "example.com")]
    #[case("http://example.com", "example.com")]
    #[case("http://example.com/path", "http://example.com/path")]
    #[case("http://subdomain.example.com", "subdomain.example.com")]
    fn pretty_url(#[case] input: &str, #[case] expected_display: &str) {
        let url = ParsedUrl::parse(input).unwrap();
        assert_eq!(url.pretty_url(), expected_display);
    }

    #[test]
    fn extend_url() {
        let url = Url::parse("https://example.com").unwrap();
        assert_eq!(
            url.extend(["bar", "baz"]).unwrap().as_str(),
            "https://example.com/bar/baz"
        );
    }

    #[rstest]
    #[case::pretty_permalinks(
        "https://example.com/wp-json",
        vec!["/wp/v2", "users", "me"],
        "https://example.com/wp-json/wp/v2/users/me"
    )]
    #[case::pretty_permalinks_trailing_slash(
        "https://example.com/wp-json/",
        vec!["/wp/v2", "users", "me"],
        "https://example.com/wp-json/wp/v2/users/me"
    )]
    #[case::rest_route_query_form(
        "https://example.com/index.php?rest_route=/",
        vec!["/wp/v2", "users", "me"],
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fusers%2Fme"
    )]
    #[case::rest_route_query_form_no_trailing_slash(
        "https://example.com/index.php?rest_route=",
        vec!["/wp/v2", "users", "me"],
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fusers%2Fme"
    )]
    #[case::rest_route_with_other_query_params(
        "https://example.com/index.php?rest_route=/&debug=1",
        vec!["/wp/v2", "users", "me"],
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fusers%2Fme&debug=1"
    )]
    #[case::rest_route_empty_segments_preserves_value(
        "https://example.com/index.php?rest_route=/",
        Vec::<&str>::new(),
        "https://example.com/index.php?rest_route=%2F"
    )]
    fn by_extending_rest_api_path(
        #[case] api_root: &str,
        #[case] segments: Vec<&str>,
        #[case] expected: &str,
    ) {
        let parsed = ParsedUrl::parse(api_root).unwrap();
        assert_eq!(
            parsed.by_extending_rest_api_path(segments).as_str(),
            expected
        );
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

    #[rstest]
    // --- Golden cases from the issue's behavior spec ---
    #[case::path_root_two_pairs(
        "https://example.com/wp-json/wp/v2/themes",
        vec![("context", "edit"), ("status", "active")],
        "https://example.com/wp-json/wp/v2/themes?context=edit&status=active"
    )]
    #[case::query_root_two_pairs(
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes",
        vec![("context", "edit"), ("status", "active")],
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes&context=edit&status=active"
    )]
    #[case::query_root_preserves_existing_debug(
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes&debug=1",
        vec![("context", "edit"), ("status", "active")],
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes&debug=1&context=edit&status=active"
    )]
    // --- Path-root variations ---
    #[case::path_root_single_pair(
        "https://example.com/wp-json/wp/v2/themes",
        vec![("context", "edit")],
        "https://example.com/wp-json/wp/v2/themes?context=edit"
    )]
    #[case::path_root_trailing_slash(
        "https://example.com/wp-json/wp/v2/themes/",
        vec![("context", "edit")],
        "https://example.com/wp-json/wp/v2/themes/?context=edit"
    )]
    #[case::path_root_with_existing_query(
        "https://example.com/wp-json/wp/v2/themes?foo=bar",
        vec![("context", "edit")],
        "https://example.com/wp-json/wp/v2/themes?foo=bar&context=edit"
    )]
    #[case::api_root_only(
        "https://example.com/wp-json",
        vec![("context", "edit")],
        "https://example.com/wp-json?context=edit"
    )]
    // --- Query-root variations ---
    #[case::query_root_bare_slash_value(
        "https://example.com/index.php?rest_route=%2F",
        vec![("context", "edit")],
        "https://example.com/index.php?rest_route=%2F&context=edit"
    )]
    #[case::query_root_single_pair(
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes",
        vec![("status", "active")],
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes&status=active"
    )]
    // --- Encoding: reserved / special characters in values ---
    #[case::reserved_comma_encoded(
        "https://example.com/wp-json/wp/v2/themes",
        vec![("exclude", "core,gutenberg")],
        "https://example.com/wp-json/wp/v2/themes?exclude=core%2Cgutenberg"
    )]
    #[case::space_encoded_as_plus(
        "https://example.com/wp-json/wp/v2/search",
        vec![("search", "hello world")],
        "https://example.com/wp-json/wp/v2/search?search=hello+world"
    )]
    #[case::ampersand_in_value_encoded(
        "https://example.com/wp-json",
        vec![("q", "a&b")],
        "https://example.com/wp-json?q=a%26b"
    )]
    #[case::equals_in_value_encoded(
        "https://example.com/wp-json",
        vec![("q", "a=b")],
        "https://example.com/wp-json?q=a%3Db"
    )]
    #[case::plus_in_value_encoded(
        "https://example.com/wp-json",
        vec![("q", "a+b")],
        "https://example.com/wp-json?q=a%2Bb"
    )]
    #[case::slash_in_value_encoded(
        "https://example.com/wp-json",
        vec![("path", "/wp/v2")],
        "https://example.com/wp-json?path=%2Fwp%2Fv2"
    )]
    #[case::unicode_value_percent_encoded(
        "https://example.com/wp-json",
        vec![("q", "café")],
        "https://example.com/wp-json?q=caf%C3%A9"
    )]
    // --- Encoding: special characters in names ---
    #[case::space_in_name_encoded(
        "https://example.com/wp-json",
        vec![("a b", "c")],
        "https://example.com/wp-json?a+b=c"
    )]
    // --- Empty value / empty name edge cases ---
    #[case::empty_value(
        "https://example.com/wp-json",
        vec![("flag", "")],
        "https://example.com/wp-json?flag="
    )]
    #[case::empty_name(
        "https://example.com/wp-json",
        vec![("", "v")],
        "https://example.com/wp-json?=v"
    )]
    // --- Order preserved; duplicate keys kept ---
    #[case::order_preserved(
        "https://example.com/wp-json",
        vec![("z", "1"), ("a", "2"), ("m", "3")],
        "https://example.com/wp-json?z=1&a=2&m=3"
    )]
    #[case::duplicate_keys_kept(
        "https://example.com/wp-json",
        vec![("status", "active"), ("status", "inactive")],
        "https://example.com/wp-json?status=active&status=inactive"
    )]
    fn by_appending_query_pairs(
        #[case] input: &str,
        #[case] pairs: Vec<(&str, &str)>,
        #[case] expected: &str,
    ) {
        let parsed = ParsedUrl::parse(input).unwrap();
        let result = parsed.by_appending_query_pairs(query_pairs(&pairs));
        assert_eq!(result.url(), expected);
    }

    /// Appending an empty `pairs` must return the URL byte-for-byte unchanged —
    /// notably, it must not add a trailing `?` to a query-less URL.
    #[rstest]
    #[case::path_root_no_query("https://example.com/wp-json/wp/v2/themes")]
    #[case::path_root_trailing_slash("https://example.com/wp-json/wp/v2/themes/")]
    #[case::path_root_with_query("https://example.com/wp-json/wp/v2/themes?foo=bar")]
    #[case::query_root("https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes")]
    #[case::query_root_with_extra("https://example.com/index.php?rest_route=%2F&debug=1")]
    #[case::bare_host("https://example.com")]
    #[case::with_fragment("https://example.com/wp-json?foo=bar#section")]
    fn by_appending_empty_query_pairs_is_unchanged(#[case] input: &str) {
        let parsed = ParsedUrl::parse(input).unwrap();
        let result = parsed.by_appending_query_pairs(Vec::new());
        assert_eq!(result.url(), parsed.url());
        assert_eq!(*result, parsed);
    }

    /// A URL fragment must survive the append (the `url` crate detaches and
    /// restores it around the query edit).
    #[rstest]
    #[case::fragment_no_query(
        "https://example.com/wp-json/wp/v2/themes#section",
        vec![("context", "edit")],
        "https://example.com/wp-json/wp/v2/themes?context=edit#section"
    )]
    #[case::fragment_with_query(
        "https://example.com/wp-json/wp/v2/themes?foo=bar#section",
        vec![("context", "edit")],
        "https://example.com/wp-json/wp/v2/themes?foo=bar&context=edit#section"
    )]
    fn by_appending_query_pairs_preserves_fragment(
        #[case] input: &str,
        #[case] pairs: Vec<(&str, &str)>,
        #[case] expected: &str,
    ) {
        let parsed = ParsedUrl::parse(input).unwrap();
        let result = parsed.by_appending_query_pairs(query_pairs(&pairs));
        assert_eq!(result.url(), expected);
    }

    /// The receiver is borrowed, not consumed: appending returns a new URL and
    /// leaves the original untouched.
    #[test]
    fn by_appending_query_pairs_does_not_mutate_receiver() {
        let parsed = ParsedUrl::parse("https://example.com/wp-json/wp/v2/themes").unwrap();
        let before = parsed.url();
        let _ = parsed.by_appending_query_pairs(query_pairs(&[("context", "edit")]));
        assert_eq!(parsed.url(), before);
    }

    /// Successive appends accumulate, matching a single append of all pairs.
    #[test]
    fn by_appending_query_pairs_is_chainable() {
        let parsed = ParsedUrl::parse("https://example.com/wp-json/wp/v2/themes").unwrap();
        let chained = parsed
            .by_appending_query_pairs(query_pairs(&[("context", "edit")]))
            .by_appending_query_pairs(query_pairs(&[("status", "active")]));
        let combined = parsed
            .by_appending_query_pairs(query_pairs(&[("context", "edit"), ("status", "active")]));
        assert_eq!(
            chained.url(),
            "https://example.com/wp-json/wp/v2/themes?context=edit&status=active"
        );
        assert_eq!(chained.url(), combined.url());
    }

    /// End-to-end consumer flow: resolve the REST path (both root forms) and
    /// then attach endpoint query parameters, exactly as GutenbergKit will.
    #[rstest]
    #[case::path_root(
        "https://example.com/wp-json",
        "https://example.com/wp-json/wp/v2/themes?context=edit&status=active"
    )]
    #[case::query_root(
        "https://example.com/index.php?rest_route=/",
        "https://example.com/index.php?rest_route=%2Fwp%2Fv2%2Fthemes&context=edit&status=active"
    )]
    fn by_extending_then_appending_composes(#[case] api_root: &str, #[case] expected: &str) {
        let parsed = ParsedUrl::parse(api_root).unwrap();
        let resolved: ParsedUrl = parsed
            .by_extending_rest_api_path(["/wp/v2", "themes"])
            .into();
        let result = resolved
            .by_appending_query_pairs(query_pairs(&[("context", "edit"), ("status", "active")]));
        assert_eq!(result.url(), expected);
    }
}
