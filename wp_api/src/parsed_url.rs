use std::fmt;
use std::fmt::Display;
use url::Url;
use wp_localization::{MessageBundle, WpMessages, WpSupportsLocalization};
use wp_localization_macro::WpDeriveLocalizable;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, uniffi::Object)]
pub struct ParsedUrl {
    pub inner: Url,
}

impl ParsedUrl {
    pub fn new(url: Url) -> Self {
        Self { inner: url }
    }
}

impl Display for ParsedUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
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
    fn message_bundle(&self) -> MessageBundle {
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
}
