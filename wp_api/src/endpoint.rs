use url::Url;

pub use plugins_endpoint::*;
pub use users_endpoint::*;

mod plugins_endpoint;
mod users_endpoint;

const WP_JSON_PATH_SEGMENTS: [&str; 3] = ["wp-json", "wp", "v2"];

#[derive(Debug, Clone)]
pub struct ApiBaseUrl {
    url: Url,
}

impl ApiBaseUrl {
    pub fn new(site_base_url: &str) -> Result<Self, url::ParseError> {
        Url::parse(site_base_url).map(|parsed_url| {
            let url = parsed_url
                .extend(WP_JSON_PATH_SEGMENTS)
                .expect("ApiBaseUrl is already parsed, so this can't result in an error");
            Self { url }
        })
    }

    fn by_appending(&self, segment: &str) -> Url {
        self.url
            .clone()
            .append(segment)
            .expect("ApiBaseUrl is already parsed, so this can't result in an error")
    }

    fn by_extending<I>(&self, segments: I) -> Url
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.url
            .clone()
            .extend(segments)
            .expect("ApiBaseUrl is already parsed, so this can't result in an error")
    }

    fn as_str(&self) -> &str {
        self.url.as_str()
    }
}

pub struct ApiEndpoint {
    pub base_url: ApiBaseUrl,
    pub users: UsersEndpoint,
    pub plugins: PluginsEndpoint,
}

impl ApiEndpoint {
    pub fn new(api_base_url: ApiBaseUrl) -> Self {
        Self {
            base_url: api_base_url.clone(),
            users: UsersEndpoint::new(api_base_url.clone()),
            plugins: PluginsEndpoint::new(api_base_url.clone()),
        }
    }

    pub fn new_from_str(site_base_url: &str) -> Result<Self, url::ParseError> {
        ApiBaseUrl::new(site_base_url).map(Self::new)
    }
}

trait UrlExtension {
    fn append(self, segment: &str) -> Result<Url, ()>;
    fn extend<I>(self, segments: I) -> Result<Url, ()>
    where
        I: IntoIterator,
        I::Item: AsRef<str>;
}

impl UrlExtension for Url {
    fn append(mut self, segment: &str) -> Result<Url, ()> {
        self.path_segments_mut()?.push(segment);
        Ok(self)
    }

    fn extend<I>(mut self, segments: I) -> Result<Url, ()>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.path_segments_mut()?.extend(segments);
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[test]
    fn append_url() {
        let url = Url::parse("https://example.com").unwrap();
        assert_eq!(
            url.append("bar").unwrap().as_str(),
            "https://example.com/bar"
        );
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
    fn api_base_url(
        #[values(
            "http://example.com",
            "https://example.com",
            "https://www.example.com",
            "https://f.example.com",
            "https://example.com/f"
        )]
        test_base_url: &str,
    ) {
        let api_base_url = ApiBaseUrl::new(test_base_url).unwrap();
        let expected_wp_json_url = wp_json_endpoint(test_base_url);
        assert_eq!(expected_wp_json_url, api_base_url.as_str());
        assert_eq!(
            api_base_url.by_appending("bar").as_str(),
            format!("{}/bar", expected_wp_json_url)
        );
        assert_eq!(
            api_base_url.by_extending(["bar", "baz"]).as_str(),
            format!("{}/bar/baz", expected_wp_json_url)
        );
    }

    fn wp_json_endpoint(base_url: &str) -> String {
        format!("{}/{}", base_url, WP_JSON_PATH_SEGMENTS.join("/"))
    }

    fn wp_json_endpoint_by_appending(base_url: &str, suffix: &str) -> String {
        format!("{}{}", wp_json_endpoint(base_url), suffix)
    }

    #[fixture]
    pub fn fixture_api_base_url() -> ApiBaseUrl {
        ApiBaseUrl::new("https://example.com").unwrap()
    }

    pub fn validate_endpoint(endpoint_url: Url, path: &str) {
        assert_eq!(
            endpoint_url.as_str(),
            format!("{}{}", fixture_api_base_url().as_str(), path)
        );
    }
}
