use url::Url;

#[derive(Debug, Clone, uniffi::Object)]
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
            .map_err(|e| match e {
                url::ParseError::RelativeUrlWithoutBase => ParseUrlError::RelativeUrlWithoutBase,
                _ => ParseUrlError::Generic {
                    reason: e.to_string(),
                },
            })
            .map(Self::new)
    }

    pub fn url(&self) -> String {
        self.inner.to_string()
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ParseUrlError {
    #[error("Error while parsing url: {}", reason)]
    Generic { reason: String },
    #[error("Relative URL without a base")]
    RelativeUrlWithoutBase,
}
