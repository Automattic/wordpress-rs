pub use url::Url;

// A type that's guaranteed to represent a valid URL
//
// It is a programmer error to instantiate this object with an invalid URL
#[derive(Debug, PartialEq, uniffi::Record)]
pub struct WPRestAPIURL {
    pub string_value: String,
}

impl WPRestAPIURL {
    pub fn as_str(&self) -> &str {
        self.string_value.as_str()
    }

    pub fn as_url(&self) -> url::Url {
        Url::parse(self.string_value.as_str()).unwrap()
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

impl From<&str> for WPRestAPIURL {
    fn from(str: &str) -> Self {
        WPRestAPIURL { string_value: str.to_string() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_be_converted_to_str() {
        assert_eq!("http://example.com", WPRestAPIURL::from("http://example.com").as_str());
    }

    #[test]
    fn can_be_converted_to_string() {
        assert_eq!(String::from(WPRestAPIURL::from("http://example.com")), "http://example.com".to_string() );
    }

    #[test]
    fn can_be_converted_to_url() {
        assert_eq!(url::Url::parse("http://example.com").unwrap(), WPRestAPIURL::from("http://example.com").as_url());
    }

    #[test]
    fn can_be_created_from_url() {
        assert_eq!(WPRestAPIURL::from("http://example.com/"), url::Url::parse("http://example.com").unwrap().into());
    }

    #[test]
    #[should_panic]
    fn panics_on_invalid_url() {
        WPRestAPIURL::from("invalid").as_url();
    }
}
