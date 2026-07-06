use std::fmt;
use std::fmt::Display;
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
}
