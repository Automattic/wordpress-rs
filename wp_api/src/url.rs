pub use url::Url;

/// A type that's guaranteed to represent a valid URL
///
/// It is a programmer error to instantiate this object with an invalid URL
#[derive(Debug, PartialEq, uniffi::Record)]
pub struct WPRestAPIURL {
    /// A string representation of the URL
    pub string_value: String,
}

impl WPRestAPIURL {
    fn as_str(&self) -> &str {
        self.string_value.as_str()
    }
}

impl From<Url> for WPRestAPIURL {
    fn from(url: url::Url) -> Self {
        WPRestAPIURL {
            string_value: url.into(),
        }
    }
}

impl From<WPRestAPIURL> for String {
    fn from(url: WPRestAPIURL) -> Self {
        url.string_value
    }
}

impl TryFrom<WPRestAPIURL> for url::Url {
    type Error = UrlParsingError;

    fn try_from(url: WPRestAPIURL) -> Result<Self, UrlParsingError> {
        Url::parse(url.as_str()).map_err(|err| UrlParsingError::InvalidUrl)
    }
}

impl From<&str> for WPRestAPIURL {
    fn from(str: &str) -> Self {
        WPRestAPIURL {
            string_value: str.to_string(),
        }
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum UrlParsingError {
    #[error("The URL you've provided is invalid")]
    InvalidUrl,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_be_converted_to_str() {
        assert_eq!(
            "http://example.com",
            WPRestAPIURL::from("http://example.com").as_str()
        );
    }

    #[test]
    fn can_be_converted_to_string() {
        assert_eq!(
            String::from(WPRestAPIURL::from("http://example.com")),
            "http://example.com".to_string()
        );
    }

    #[test]
    fn can_be_converted_to_url() {
        assert_eq!(
            url::Url::parse("http://example.com").unwrap(),
            WPRestAPIURL::from("http://example.com").try_into().unwrap()
        );
    }

    #[test]
    fn can_be_created_from_url() {
        assert_eq!(
            WPRestAPIURL::from("http://example.com/"),
            url::Url::parse("http://example.com").unwrap().into()
        );
    }

    #[test]
    fn panics_on_invalid_url() {
        assert!(url::Url::try_from(WPRestAPIURL::from("invalid")).is_err())
    }
}
