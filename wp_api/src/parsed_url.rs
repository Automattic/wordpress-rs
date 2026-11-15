use std::fmt;
use std::fmt::Display;
use std::fmt::Write;
use url::{Position, Url};
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

/// `url` as a string that only uses characters RFC 3986 allows.
///
/// `url` follows the WHATWG URL standard, which leaves some characters as they
/// are, such as `{`, `}`, `|`, `^`, `[` and `]` in a query, or `{` in a host.
/// Stricter parsers reject them: Foundation's `URL(string:)` returns `nil`
/// (for a host, on every OS version), and Java's `URI` throws.
///
/// This percent-encodes those characters anywhere in the URL, except the
/// brackets around an IPv6 address. It also encodes a `%` that doesn't start
/// an escape, and a `#` inside the fragment. The result names the same
/// resource; for a URL that is already valid RFC 3986 it is unchanged.
pub(crate) fn rfc3986_serialization(url: &Url) -> String {
    let serialized = url.as_str();
    let path_start = url[..Position::BeforePath].len();
    let bytes = serialized.as_bytes();

    let mut result = String::with_capacity(serialized.len());
    let mut in_fragment = false;
    for (index, &byte) in bytes.iter().enumerate() {
        let in_authority = index < path_start;
        let is_escape = byte == b'%'
            && bytes.get(index + 1).is_some_and(u8::is_ascii_hexdigit)
            && bytes.get(index + 2).is_some_and(u8::is_ascii_hexdigit);
        let is_fragment_start = !in_authority && byte == b'#' && !in_fragment;
        if is_fragment_start {
            in_fragment = true;
        }
        let is_allowed = is_rfc3986_path_query_or_fragment_char(byte)
            // `url` only leaves brackets in the authority around an IPv6
            // address; it encodes them in the user name and password.
            || (in_authority && matches!(byte, b'[' | b']'));
        if is_escape || is_fragment_start || is_allowed {
            result.push(byte as char);
        } else {
            write!(result, "%{byte:02X}").expect("writing to a String succeeds");
        }
    }
    result
}

/// Whether RFC 3986 allows `byte` as it is in a path, query or fragment:
/// `unreserved`, `sub-delims`, `:`, `@`, `/` and `?`.
fn is_rfc3986_path_query_or_fragment_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@/?".contains(&byte)
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

    #[rstest]
    #[case::already_valid(
        "https://user:pw@example.com:8080/a/b.png?x=1&y=a+b#frag",
        "https://user:pw@example.com:8080/a/b.png?x=1&y=a+b#frag"
    )]
    #[case::ipv6_host("http://[2001:db8::1]:8080/", "http://[2001:db8::1]:8080/")]
    #[case::query_characters(
        "https://example.com/icon.png?v={hash}|2^x[0]",
        "https://example.com/icon.png?v=%7Bhash%7D%7C2%5Ex%5B0%5D"
    )]
    #[case::path_characters("https://example.com/a|b^c[d]", "https://example.com/a%7Cb%5Ec%5Bd%5D")]
    #[case::fragment_characters("https://example.com/#a{b}#c", "https://example.com/#a%7Bb%7D%23c")]
    #[case::existing_escapes_kept(
        "https://example.com/a%20b?q=%7B",
        "https://example.com/a%20b?q=%7B"
    )]
    #[case::stray_percent("https://example.com/100%?q=%zz", "https://example.com/100%25?q=%25zz")]
    #[case::non_ascii(
        "https://example.com/café?q=é",
        "https://example.com/caf%C3%A9?q=%C3%A9"
    )]
    #[case::cannot_be_a_base("mailto:someone@example.com", "mailto:someone@example.com")]
    #[case::host_placeholder("https://{{cdn}}/favicon.png", "https://%7B%7Bcdn%7D%7D/favicon.png")]
    #[case::host_quote_and_backtick("https://a\"b`c/", "https://a%22b%60c/")]
    #[case::userinfo_stray_percent("https://us%er@example.com/", "https://us%25er@example.com/")]
    #[case::query_backtick_and_backslash(
        "https://example.com/?a`b\\c",
        "https://example.com/?a%60b%5Cc"
    )]
    #[case::opaque_path("foo:a{b}|c", "foo:a%7Bb%7D%7Cc")]
    fn rfc3986_serialization(#[case] input: &str, #[case] expected: &str) {
        let url = Url::parse(input).unwrap();
        assert_eq!(super::rfc3986_serialization(&url), expected);
        // An RFC 3986 parser we didn't write accepts it.
        assert!(
            iri_string::types::UriStr::new(expected).is_ok(),
            "{expected} isn't valid RFC 3986"
        );
        // Encoding is stable: parsing the result again doesn't change it.
        let reparsed = Url::parse(expected).unwrap();
        assert_eq!(super::rfc3986_serialization(&reparsed), expected);
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
}
