use url::Url;

use crate::SparseField;

pub(crate) mod application_passwords_endpoint;
pub(crate) mod plugins_endpoint;
pub(crate) mod post_types_endpoint;
pub(crate) mod site_settings_endpoint;
pub(crate) mod users_endpoint;
pub(crate) mod wp_site_health_tests_endpoint;

const WP_JSON_PATH_SEGMENTS: [&str; 1] = ["wp-json"];

uniffi::custom_newtype!(WpEndpointUrl, String);
#[derive(Debug, Clone)]
pub struct WpEndpointUrl(pub String);

impl From<Url> for WpEndpointUrl {
    fn from(url: Url) -> Self {
        Self(url.to_string())
    }
}

#[derive(Debug)]
pub(crate) struct ApiEndpointUrl {
    url: Url,
}

impl ApiEndpointUrl {
    pub fn new(url: Url) -> Self {
        Self { url }
    }

    fn url(&self) -> &Url {
        &self.url
    }

    pub fn as_str(&self) -> &str {
        self.url.as_str()
    }
}

impl From<Url> for ApiEndpointUrl {
    fn from(url: Url) -> Self {
        Self::new(url)
    }
}

impl From<ApiEndpointUrl> for WpEndpointUrl {
    fn from(url: ApiEndpointUrl) -> Self {
        Self(url.as_str().to_string())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ApiBaseUrl {
    url: Url,
}

impl From<Url> for ApiBaseUrl {
    fn from(url: Url) -> Self {
        let url = url
            .extend(WP_JSON_PATH_SEGMENTS)
            .expect("Given url is already parsed, so this can't result in an error");
        Self { url }
    }
}

impl TryFrom<&str> for ApiBaseUrl {
    type Error = url::ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Url::parse(value).map(ApiBaseUrl::from)
    }
}

impl ApiBaseUrl {
    pub fn new(site_base_url: &str) -> Result<Self, url::ParseError> {
        site_base_url.try_into()
    }

    fn by_appending(&self, segment: &str) -> Url {
        self.url
            .clone()
            .append(segment)
            .expect("ApiBaseUrl is already parsed, so this can't result in an error")
    }

    pub fn by_extending_and_splitting_by_forward_slash<I>(&self, segments: I) -> Url
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.url
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
            .expect("ApiBaseUrl is already parsed, so this can't result in an error")
    }

    fn as_str(&self) -> &str {
        self.url.as_str()
    }
}

trait UrlExtension {
    fn append(self, segment: &str) -> Result<Url, ()>;
    fn extend<I>(self, segments: I) -> Result<Url, ()>
    where
        I: IntoIterator,
        I::Item: AsRef<str>;
    fn append_filter_fields(self, fields: &[impl SparseField]) -> Url;
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

    fn append_filter_fields(mut self, fields: &[impl SparseField]) -> Url {
        self.query_pairs_mut().append_pair(
            "_fields",
            fields
                .iter()
                .map(|f| f.as_str())
                .collect::<Vec<&str>>()
                .join(",")
                .as_str(),
        );
        self
    }
}

trait DerivedRequest {
    fn namespace() -> Namespace;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Namespace {
    WpSiteHealthV1,
    WpV2,
}

impl Namespace {
    fn as_str(&self) -> &str {
        match self {
            Self::WpSiteHealthV1 => "/wp-site-health/v1",
            Self::WpV2 => "/wp/v2",
        }
    }
}

mod macros {
    macro_rules! default_sparse_field_implementation_from_field_name {
        ($ident:ident) => {
            paste::paste! {
                impl SparseField for $ident {
                    fn as_str(&self) -> &str {
                        self.as_field_name()
                    }
                }
            }
        };
    }

    pub(crate) use default_sparse_field_implementation_from_field_name;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

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
        let api_base_url: ApiBaseUrl = test_base_url.try_into().unwrap();
        let expected_wp_json_url = wp_json_endpoint(test_base_url);
        assert_eq!(expected_wp_json_url, api_base_url.as_str());
        assert_eq!(
            api_base_url.by_appending("bar").as_str(),
            format!("{}/bar", expected_wp_json_url)
        );
        assert_eq!(
            api_base_url
                .by_extending_and_splitting_by_forward_slash(["bar", "baz"])
                .as_str(),
            format!("{}/bar/baz", expected_wp_json_url)
        );
        assert_eq!(
            api_base_url
                .by_extending_and_splitting_by_forward_slash(["bar", "baz/quox"])
                .as_str(),
            format!("{}/bar/baz/quox", expected_wp_json_url)
        );
        assert_eq!(
            api_base_url
                .by_extending_and_splitting_by_forward_slash(["/bar", "/baz/quox"])
                .as_str(),
            format!("{}/bar/baz/quox", expected_wp_json_url)
        );
    }

    fn wp_json_endpoint(base_url: &str) -> String {
        format!("{}/{}", base_url, WP_JSON_PATH_SEGMENTS.join("/"))
    }

    fn wp_json_endpoint_by_appending(base_url: &str, suffix: &str) -> String {
        format!("{}{}", wp_json_endpoint(base_url), suffix)
    }

    #[fixture]
    pub fn fixture_api_base_url() -> Arc<ApiBaseUrl> {
        ApiBaseUrl::try_from("https://example.com").unwrap().into()
    }

    pub fn validate_wp_v2_endpoint(endpoint_url: ApiEndpointUrl, path: &str) {
        validate_endpoint(Namespace::WpV2, endpoint_url, path);
    }

    pub fn validate_wp_site_health_endpoint(endpoint_url: ApiEndpointUrl, path: &str) {
        validate_endpoint(Namespace::WpSiteHealthV1, endpoint_url, path);
    }

    fn validate_endpoint(namespace: Namespace, endpoint_url: ApiEndpointUrl, path: &str) {
        assert_eq!(
            endpoint_url.as_str(),
            format!(
                "{}{}{}",
                fixture_api_base_url().as_str(),
                namespace.as_str(),
                path
            )
        );
    }
}
