use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, uniffi::Object)]
pub struct ParsedUrl {
    pub inner: Url,
}

impl ParsedUrl {
    pub fn new(url: Url) -> Self {
        Self { inner: url }
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
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, thiserror::Error, uniffi::Error)]
pub enum ParseUrlError {
    #[error("Error while parsing url: {}", reason)]
    Generic { reason: String },
    #[error("empty host")]
    EmptyHost,
    #[error("invalid international domain name")]
    IdnaError,
    #[error("invalid port number")]
    InvalidPort,
    #[error("invalid IPv4 address")]
    InvalidIpv4Address,
    #[error("invalid IPv6 address")]
    InvalidIpv6Address,
    #[error("invalid domain character")]
    InvalidDomainCharacter,
    #[error("relative URL without a base")]
    RelativeUrlWithoutBase,
    #[error("relative URL with a cannot-be-a-base base")]
    RelativeUrlWithCannotBeABaseBase,
    #[error("a cannot-be-a-base URL doesn’t have a host to set")]
    SetHostOnCannotBeABaseUrl,
    #[error("URLs more than 4 GB are not supported")]
    Overflow,
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
        assert_eq!(parsed_url.url(), "http://example.com/",);
        assert_eq!(parsed_url, Url::parse("http://example.com").unwrap().into());
    }

    #[rstest]
    #[case("https://", ParseUrlError::EmptyHost)]
    #[case("https://example.com:foo", ParseUrlError::InvalidPort)]
    #[case("https://1.2.3.4.5", ParseUrlError::InvalidIpv4Address)]
    #[case("https://[1", ParseUrlError::InvalidIpv6Address)]
    #[case("https:// .com", ParseUrlError::InvalidDomainCharacter)]
    #[case("", ParseUrlError::RelativeUrlWithoutBase)]
    // https://www.unicode.org/reports/tr46/#Validity_Criteria
    #[case("https://xn--u-ccb.com", ParseUrlError::IdnaError)]
    fn parse_url_error(#[case] input: &str, #[case] expected_err: ParseUrlError) {
        assert_eq!(ParsedUrl::try_from(input).unwrap_err(), expected_err);
    }
}
