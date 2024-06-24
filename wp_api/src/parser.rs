use url::Url;

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum ParseSiteUrlError {
    #[error("Error while parsing site url: {}", reason)]
    ParsingError { reason: String },
}

impl From<url::ParseError> for ParseSiteUrlError {
    fn from(value: url::ParseError) -> Self {
        Self::ParsingError {
            reason: value.to_string(),
        }
    }
}

#[derive(Debug)]
pub struct ParsedSiteUrl {
    pub site_url: Url,
}

impl ParsedSiteUrl {
    pub(crate) fn parse_str(str: impl AsRef<str>) -> Result<ParsedSiteUrl, ParseSiteUrlError> {
        Url::parse(str.as_ref())
            .map(ParsedSiteUrl::from)
            .map_err(ParseSiteUrlError::from)
    }
}

impl From<Url> for ParsedSiteUrl {
    fn from(url: Url) -> Self {
        Self { site_url: url }
    }
}
