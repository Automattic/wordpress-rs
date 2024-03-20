pub use url::Url;

// A type that's guaranteed to represent a valid URL
//
// It is a programmer error to instantiate this object with an invalid URL
#[derive(Debug, uniffi::Record)]
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
