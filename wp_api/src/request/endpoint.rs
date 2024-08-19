use url::Url;

use crate::SparseField;

pub(crate) mod application_passwords_endpoint;
pub(crate) mod plugins_endpoint;
pub(crate) mod post_types_endpoint;
pub(crate) mod posts_endpoint;
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
enum WpSite {
    WPCom { host: String },
    SelfHosted { url: Url },
}

#[derive(Debug, Clone)]
pub(crate) struct ApiBaseUrl {
    site: WpSite,
}

impl ApiBaseUrl {
    fn wordpress_com(host: &str) -> Self {
        Self {
            site: WpSite::WPCom {
                host: host.to_string(),
            },
        }
    }

    fn self_hosted(url: Url) -> Self {
        Self {
            site: WpSite::SelfHosted { url },
        }
    }

    fn url_at_namespace(&self, namespace: Option<&Namespace>) -> Url {
        match &self.site {
            WpSite::WPCom { host } => {
                let mut url =
                    Url::parse("https://public-api.wordpress.com").expect("Host is a valid url");

                if let Some(namespace) = namespace {
                    url.path_segments_mut()
                        .expect("url is a full URL")
                        .extend(namespace.path_segments());

                    url.path_segments_mut()
                        .expect("url is a full URL")
                        .push("sites")
                        .push(host);
                }

                url
            }
            WpSite::SelfHosted { url } => {
                let mut url = url
                    .clone()
                    .extend(WP_JSON_PATH_SEGMENTS.iter())
                    .expect("Host is a valid url");

                if let Some(namespace) = namespace {
                    url.path_segments_mut()
                        .expect("url is a parsed full URL")
                        .extend(namespace.path_segments());
                }

                url
            }
        }
    }
}

impl From<Url> for ApiBaseUrl {
    fn from(url: Url) -> Self {
        ApiBaseUrl::self_hosted(url)
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

    pub fn by_extending_and_splitting_by_forward_slash<I>(
        &self,
        namespace: Option<&Namespace>,
        segments: I,
    ) -> Url
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.url_at_namespace(namespace)
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

    fn root(&self) -> Url {
        self.url_at_namespace(None)
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
pub enum Namespace {
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

    fn path_segments(&self) -> impl Iterator<Item = &str> {
        self.as_str()
            .split("/")
            .map(|f| f.trim())
            .filter(|f| !f.is_empty())
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
        assert_eq!(expected_wp_json_url, api_base_url.root().as_str());
        assert_eq!(
            api_base_url
                .by_extending_and_splitting_by_forward_slash(None, ["bar", "baz"])
                .as_str(),
            format!("{}/bar/baz", expected_wp_json_url)
        );
        assert_eq!(
            api_base_url
                .by_extending_and_splitting_by_forward_slash(None, ["bar", "baz/quox"])
                .as_str(),
            format!("{}/bar/baz/quox", expected_wp_json_url)
        );
        assert_eq!(
            api_base_url
                .by_extending_and_splitting_by_forward_slash(None, ["/bar", "/baz/quox"])
                .as_str(),
            format!("{}/bar/baz/quox", expected_wp_json_url)
        );
    }

    #[rstest]
    fn wp_com_url() {
        let api_base_url: ApiBaseUrl = ApiBaseUrl::wordpress_com("example.com");
        assert_eq!(
            api_base_url.root().as_str(),
            "https://public-api.wordpress.com/"
        );
        assert_eq!(
            api_base_url
                .url_at_namespace(Some(&Namespace::WpV2))
                .as_str(),
            "https://public-api.wordpress.com/wp/v2/sites/example.com"
        );
        assert_eq!(
            api_base_url
                .by_extending_and_splitting_by_forward_slash(
                    Some(&Namespace::WpV2),
                    ["users", "me"]
                )
                .as_str(),
            "https://public-api.wordpress.com/wp/v2/sites/example.com/users/me"
        )
    }

    fn wp_json_endpoint(base_url: &str) -> String {
        format!("{}/{}", base_url, WP_JSON_PATH_SEGMENTS.join("/"))
    }

    fn wp_json_endpoint_by_appending(base_url: &str, suffix: &str) -> String {
        format!("{}{}", wp_json_endpoint(base_url), suffix)
    }

    #[fixture]
    pub fn fixture_api_base_url() -> Arc<ApiBaseUrl> {
        let url = Url::parse("https://example.com").unwrap();
        ApiBaseUrl::self_hosted(url).into()
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
                fixture_api_base_url().root().as_str(),
                namespace.as_str(),
                path
            )
        );
    }
}
