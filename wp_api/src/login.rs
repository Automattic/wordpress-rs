use crate::{
    JsonValue, login::url_discovery::is_local_dev_environment_url, parsed_url::ParsedUrl,
    request::endpoint::ApiUrlResolver, uuid::WpUuid,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str, sync::Arc};
use wp_localization::{MessageBundle, WpMessages, WpSupportsLocalization};
use wp_localization_macro::WpDeriveLocalizable;
use wp_serde_helper::{
    deserialize_empty_array_or_hashmap, deserialize_false_or_string, deserialize_offset,
    deserialize_string_vec_or_string_as_option,
};

const KEY_APPLICATION_PASSWORDS: &str = "application-passwords";
const KEY_OAUTH2: &str = "oauth2";

pub mod login_client;
pub mod nonce;
pub mod oauth2_configuration;
pub mod url_discovery;

#[derive(Debug, uniffi::Record)]
pub struct WpRestApiUrls {
    api_details: Arc<WpApiDetails>,
    api_root_url: String,
}

// After a successful login, the system will receive an OAuth callback with the login details
// embedded as query params. This function parses that URL and extracts the login details as an object.
#[uniffi::export]
pub fn extract_login_details_from_url(
    url: String,
) -> Result<WpApiApplicationPasswordDetails, OAuthResponseUrlError> {
    let url = ParsedUrl::parse(&url).map_err(|_| OAuthResponseUrlError::InvalidUrl)?;
    extract_login_details_from_parsed_url(url)
}

pub fn extract_login_details_from_parsed_url(
    url: ParsedUrl,
) -> Result<WpApiApplicationPasswordDetails, OAuthResponseUrlError> {
    let f = |key| {
        url.inner
            .query_pairs()
            .find_map(|(k, v)| (k == key).then_some(v.to_string()))
    };
    if let Some(is_success) = f("success")
        && is_success == "false"
    {
        return Err(OAuthResponseUrlError::UnsuccessfulLogin);
    }
    let site_url = f("site_url").ok_or(OAuthResponseUrlError::MissingSiteUrl)?;
    let user_login = f("user_login").ok_or(OAuthResponseUrlError::MissingUsername)?;
    let password = f("password").ok_or(OAuthResponseUrlError::MissingPassword)?;
    Ok(WpApiApplicationPasswordDetails {
        site_url,
        user_login,
        password,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, uniffi::Object)]
pub struct WpApiDetails {
    pub name: String,
    pub description: String,
    pub url: String,
    pub home: String,
    #[serde(default, deserialize_with = "deserialize_offset")]
    pub gmt_offset: Option<f64>,
    pub timezone_string: Option<String>,
    pub namespaces: Vec<String>,
    pub authentication: WpApiDetailsAuthenticationMap,
    #[serde(default, deserialize_with = "deserialize_false_or_string")]
    pub site_icon_url: Option<String>,
    pub routes: HashMap<String, WpRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, uniffi::Record)]
pub struct WpRoute {
    pub namespace: String,
    pub methods: Vec<String>,
    pub endpoints: Vec<WpEndpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, uniffi::Record)]
pub struct WpEndpoint {
    pub methods: Vec<String>,
    pub args: Arc<WpEndpointArgs>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, uniffi::Object)]
#[serde(transparent)]
pub struct WpEndpointArgs(serde_json::Value);

impl WpEndpointArgs {
    pub fn get(&self, arg: &str) -> Option<WpEndpointArg> {
        let obj = self.0.as_object()?;
        let value = obj.get(arg)?;
        serde_json::from_value(value.clone()).ok()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, uniffi::Record)]
pub struct WpEndpointArg {
    pub required: bool,

    pub default: Option<JsonValue>,
    pub description: Option<String>,
    #[serde(deserialize_with = "deserialize_string_vec_or_string_as_option")]
    #[serde(default)]
    pub r#type: Option<Vec<String>>,
    pub r#enum: Option<Vec<JsonValue>>,
    // There are many other fields that are specific to the type of argument. These are not currently supported because
    // they're likely to be of limited value to library users. We're open to adding them if there's a demand for them.
}

impl TryFrom<&[u8]> for WpApiDetails {
    type Error = serde_json::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        // If the body starts with the UTF-8 BOM, remove it
        if value.starts_with(&[0xEF, 0xBB, 0xBF]) {
            serde_json::from_slice::<WpApiDetails>(&value[3..])
        } else {
            serde_json::from_slice::<WpApiDetails>(value)
        }
    }
}

#[uniffi::export]
impl WpApiDetails {
    /// Does the site have application passwords enabled?
    pub fn has_application_passwords_authentication_url(&self) -> bool {
        self.authentication
            .has_application_passwords_authentication_url()
    }

    /// Returns the URL to be used in application password authentication.
    ///
    /// See the "Authorization Flow" section for details:
    /// <https://github.com/WordPress/wordpress-develop/blob/530493396b324f5bed518a494e2843e7fdb020f1/src/wp-includes/rest-api.php#L1099-L1119>
    pub fn find_application_passwords_authentication_url(&self) -> Option<String> {
        self.authentication
            .find_application_passwords_authentication_url()
    }

    /// Does the site use OAuth2?
    pub fn has_oauth2(&self) -> bool {
        self.authentication.has_oauth2()
    }

    pub fn find_oauth2_endpoints(&self) -> Option<OAuth2Endpoints> {
        self.authentication.find_oauth2_endpoints()
    }

    /// Does the site URL (as defined by the site itself, not by user input) use HTTPS?
    pub fn uses_https(&self) -> bool {
        self.url.starts_with("https://")
    }

    /// Does the site use a plugin that disables application passwords?
    pub fn has_application_password_blocking_plugin(&self) -> bool {
        KnownAuthenticationBlockingPlugin::application_passwords()
            .iter()
            .any(|plugin| self.namespaces.contains(&plugin.namespace))
    }

    /// Returns a list of plugins that might be responsible for disabling application passwords.
    pub fn application_password_blocking_plugins(&self) -> Vec<KnownAuthenticationBlockingPlugin> {
        KnownAuthenticationBlockingPlugin::application_passwords()
            .iter()
            .filter(|plugin| self.namespaces.contains(&plugin.namespace))
            .cloned()
            .collect()
    }

    /// Returns a list of plugins that might be responsible for disabling XML-RPC.
    pub fn xmlrpc_blocking_plugins(&self) -> Vec<KnownAuthenticationBlockingPlugin> {
        KnownAuthenticationBlockingPlugin::xmlrpc()
            .iter()
            .filter(|plugin| self.namespaces.contains(&plugin.namespace))
            .cloned()
            .collect()
    }

    /// Returns the site URL (as defined by the site itself, not by user input) as a string.
    pub fn site_url_string(&self) -> String {
        self.url.clone()
    }

    /// Returns `true` if the site URL looks like a local development environment URL.
    pub fn site_url_is_local_development_environment(&self) -> bool {
        ParsedUrl::parse(self.url.as_str())
            .is_ok_and(|parsed_url| is_local_dev_environment_url(&parsed_url))
    }

    /// Returns the site home URL as a string.
    pub fn home_url_string(&self) -> String {
        self.home.clone()
    }

    /// Returns the site GMT offset.
    pub fn gmt_offset(&self) -> Option<f64> {
        self.gmt_offset
    }

    /// Returns `true` if the site has routes matching the given namespace.
    pub fn has_namespace(&self, namespace: String) -> bool {
        self.namespaces.contains(&namespace)
    }

    /// Returns `true` if the site has the given route.
    pub fn has_route(&self, route: String) -> bool {
        self.routes.contains_key(&route)
    }

    /// Returns `true` if the site has a route matching the given namespace and path.
    ///
    /// Uses the resolver to construct the expected route key (which may
    /// include site-specific segments like `/sites/{id}` for WordPress.com).
    pub fn has_route_for_endpoint(
        &self,
        api_url_resolver: &dyn ApiUrlResolver,
        namespace: String,
        endpoint_path: String,
    ) -> bool {
        let route_key = api_url_resolver.route_path(namespace, endpoint_path);
        self.routes.contains_key(&route_key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, uniffi::Record)]
pub struct KnownAuthenticationBlockingPlugin {
    /// The name of the plugin.
    pub name: String,
    /// The plugin's REST API namespace.
    pub namespace: String,
    /// A URL to the plugin's support page, where users can find help.
    pub support_url: String,
}

impl KnownAuthenticationBlockingPlugin {
    fn all() -> Vec<Self> {
        vec![
            Self {
                name: "Wordfence".to_string(),
                namespace: "wordfence/v1".to_string(),
                // TODO: Ensure this is correct with the WordFence folks
                support_url: "https://www.wordfence.com/support/".to_string(),
            },
            Self {
                name: "Hostinger Tools".to_string(),
                namespace: "hostinger-tools-plugin/v1".to_string(),
                // TODO: Ensure this is correct with the Hostinger folks
                support_url: "https://wordpress.org/support/plugin/hostinger/".to_string(),
            },
            Self {
                name: "FluentAuth".to_string(),
                namespace: "fluent-auth".to_string(),
                // TODO: Ensure this is correct with the FluentAuth folks
                support_url: "https://wordpress.org/support/plugin/fluent-security/".to_string(),
            },
        ]
    }

    fn application_passwords() -> Vec<Self> {
        Self::all()
    }

    fn xmlrpc() -> Vec<Self> {
        Self::all()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum WpRestApiAuthenticationScheme {
    ApplicationPassword(WpRestApiApplicationPasswordAuthenticationScheme),
    OAuth2(WpRestApiOAuth2AuthenticationScheme),
    /// Catch-all for unknown authentication schemes (e.g., oauth1)
    Unknown(serde_json::Value),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, uniffi::Record)]
pub struct WpRestApiApplicationPasswordAuthenticationScheme {
    pub endpoints: WpRestApiAuthorizationEndpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, uniffi::Record)]
pub struct WpRestApiAuthorizationEndpoint {
    pub authorization: String,
}

/// OAuth2 authentication scheme with authorization and token endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, uniffi::Record)]
pub struct WpRestApiOAuth2AuthenticationScheme {
    pub authorize: String,
    pub token: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, uniffi::Record)]
pub struct WpApiApplicationPasswordDetails {
    pub site_url: String,
    pub user_login: String,
    pub password: String,
}

#[derive(
    Debug, PartialEq, Eq, PartialOrd, Ord, thiserror::Error, uniffi::Error, WpDeriveLocalizable,
)]
pub enum OAuthResponseUrlError {
    InvalidUrl,
    MissingSiteUrl,
    MissingUsername,
    MissingPassword,
    UnsuccessfulLogin,
}

impl WpSupportsLocalization for OAuthResponseUrlError {
    fn message_bundle(&self) -> MessageBundle<'_> {
        match self {
            OAuthResponseUrlError::MissingSiteUrl
            | OAuthResponseUrlError::MissingUsername
            | OAuthResponseUrlError::MissingPassword => {
                WpMessages::oauth_response_url_error_url_invalid()
            }
            OAuthResponseUrlError::UnsuccessfulLogin => {
                WpMessages::oauth_response_url_error_unsuccessful_login()
            }
            OAuthResponseUrlError::InvalidUrl => WpMessages::oauth_response_url_error_url_invalid(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WpApiDetailsAuthenticationMap(
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    HashMap<String, WpRestApiAuthenticationScheme>,
);

impl WpApiDetailsAuthenticationMap {
    pub fn has_application_passwords_authentication_url(&self) -> bool {
        self.0.contains_key(KEY_APPLICATION_PASSWORDS)
    }

    pub fn find_application_passwords_authentication_url(&self) -> Option<String> {
        self.0
            .get(KEY_APPLICATION_PASSWORDS)
            .and_then(|auth_scheme| match auth_scheme {
                WpRestApiAuthenticationScheme::ApplicationPassword(auth_scheme) => {
                    Some(auth_scheme.endpoints.authorization.clone())
                }
                _ => None,
            })
    }

    pub fn has_oauth2(&self) -> bool {
        self.0.contains_key(KEY_OAUTH2)
    }

    pub fn find_oauth2_endpoints(&self) -> Option<OAuth2Endpoints> {
        self.0
            .get(KEY_OAUTH2)
            .and_then(|auth_scheme| match auth_scheme {
                WpRestApiAuthenticationScheme::OAuth2(auth_scheme) => Some(OAuth2Endpoints {
                    authorization_url: auth_scheme.authorize.clone(),
                    token_url: auth_scheme.token.clone(),
                }),
                _ => None,
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, uniffi::Record)]
pub struct OAuth2Endpoints {
    pub authorization_url: String,
    pub token_url: String,
}

/// Return a URL to be used in application password authentication.
///
/// See the "Authorization Flow" section for details:
/// <https://make.wordpress.org/core/2020/11/05/application-passwords-integration-guide/>
#[uniffi::export]
pub fn create_application_password_authentication_url(
    login_url: Arc<ParsedUrl>,
    app_name: String,
    app_id: Option<Arc<WpUuid>>,
    success_url: Option<String>,
    reject_url: Option<String>,
) -> ParsedUrl {
    let mut auth_url = login_url.inner.clone();
    auth_url
        .query_pairs_mut()
        .append_pair("app_name", app_name.as_str());
    if let Some(app_id) = app_id {
        auth_url
            .query_pairs_mut()
            .append_pair("app_id", app_id.uuid_string().as_str());
    }
    if let Some(success_url) = success_url {
        auth_url
            .query_pairs_mut()
            .append_pair("success_url", success_url.as_str());
    }
    if let Some(reject_url) = reject_url {
        auth_url
            .query_pairs_mut()
            .append_pair("reject_url", reject_url.as_str());
    }
    ParsedUrl::new(auth_url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::endpoint::WpOrgSiteApiUrlResolver, wp_com::endpoint::WpComDotOrgApiUrlResolver,
    };
    use rstest::rstest;
    use std::io::Read;

    #[rstest]
    #[case(
        "exampleauth://login?site_url=http://example.com&user_login=test&password=1234",
        Ok(())
    )]
    #[case(
        "exampleauth://login?site_url=http://example.com&user_login=test&password=1234&foo=bar",
        Ok(())
    )]
    #[case(
        "exampleauth://login?user_login=test&password=1234",
        Err(OAuthResponseUrlError::MissingSiteUrl)
    )]
    #[case(
        "exampleauth://login?site_url=http://example.com&password=1234",
        Err(OAuthResponseUrlError::MissingUsername)
    )]
    #[case(
        "exampleauth://login?site_url=http://example.com&user_login=test",
        Err(OAuthResponseUrlError::MissingPassword)
    )]
    #[case(
        "exampleauth://login?success=false",
        Err(OAuthResponseUrlError::UnsuccessfulLogin)
    )]
    #[case(
        "exampleauth://login?success=true",
        Err(OAuthResponseUrlError::MissingSiteUrl)
    )]
    fn test_extract_login_details_from_url(
        #[case] input: &str,
        #[case] expected_result: Result<(), OAuthResponseUrlError>,
    ) {
        assert_eq!(
            extract_login_details_from_url(ParsedUrl::try_from(input).unwrap().into()),
            expected_result.map(|_| WpApiApplicationPasswordDetails {
                site_url: "http://example.com".to_string(),
                user_login: "test".to_string(),
                password: "1234".to_string(),
            })
        );
    }

    #[rstest]
    fn test_auth_url() {
        let app_id = WpUuid::new();
        let app_id_str = app_id.uuid_string();
        let login_url = ParsedUrl::parse("https://example.com/wp-login.php").unwrap();
        let auth_url = create_application_password_authentication_url(
            login_url.into(),
            "AppName".to_string(),
            Some(app_id.into()),
            Some("https://example.com/success".to_string()),
            Some("https://example.com/reject".to_string()),
        );

        let expected_url = format!(
            "https://example.com/wp-login.php?app_name=AppName&app_id={app_id_str}&success_url=https%3A%2F%2Fexample.com%2Fsuccess&reject_url=https%3A%2F%2Fexample.com%2Freject"
        );
        assert_eq!(auth_url, ParsedUrl::parse(expected_url.as_str()).unwrap());
    }

    #[test]
    fn test_parse_wp_api_details_authentication_map_only_application_passwords() {
        let json = r#"{
          "authentication": {
            "application-passwords": {
              "endpoints": {
                "authorization": "http://localhost/wp-admin/authorize-application.php"
              }
            }
          }
        }"#;
        test_parse_wp_api_details_authentication_map_helper(json);
    }

    #[test]
    fn test_parse_wp_api_details_authentication_map_application_passwords_and_oauth() {
        let json = r#"{
          "authentication": {
            "oauth1": {
              "request": "http://localhost/oauth1/request",
              "authorize": "http://localhost/oauth1/authorize",
              "access": "http://localhost/oauth1/access",
              "version": "0.1"
            },
            "application-passwords": {
              "endpoints": {
                "authorization": "http://localhost/wp-admin/authorize-application.php"
              }
            }
          }
        }"#;
        test_parse_wp_api_details_authentication_map_helper(json);
    }

    #[test]
    fn test_parse_wp_api_details_authentication_map_application_passwords_and_oauth2() {
        let json = r#"{
        "authentication": {
                "oauth2": {
                    "authorize": "http://localhost/oauth/authorize",
                    "token": "http://localhost/oauth/token",
                    "me": "http://localhost/oauth/me",
                    "version": "2.0",
                    "software": "WP OAuth Server"
                },
                "application-passwords": {
                    "endpoints": {
                        "authorization": "http://localhost/wp-admin/authorize-application.php"
                    }
                }
            }
        }"#;
        let result = serde_json::from_str::<WpApiDetailsAuthenticationMapWrapper>(json);
        assert!(
            result.is_ok(),
            "Failed to parse json as `WpApiDetailsAuthenticationMap`"
        );
        let auth_map = result
            .expect("Already verified result is Ok")
            .authentication;

        // Verify application passwords URL
        assert_eq!(
            auth_map.find_application_passwords_authentication_url(),
            Some("http://localhost/wp-admin/authorize-application.php".to_string())
        );

        // Verify OAuth2 endpoints
        assert!(auth_map.has_oauth2());
        let oauth2_endpoints = auth_map.find_oauth2_endpoints();
        assert!(oauth2_endpoints.is_some());
        let endpoints = oauth2_endpoints.unwrap();
        assert_eq!(
            endpoints.authorization_url,
            "http://localhost/oauth/authorize"
        );
        assert_eq!(endpoints.token_url, "http://localhost/oauth/token");
    }

    #[test]
    fn test_find_oauth2_endpoints_returns_none_when_missing() {
        let json = r#"{
            "authentication": {
                "application-passwords": {
                    "endpoints": {
                        "authorization": "http://localhost/wp-admin/authorize-application.php"
                    }
                }
            }
        }"#;
        let result = serde_json::from_str::<WpApiDetailsAuthenticationMapWrapper>(json)
            .expect("Failed to parse json");
        assert!(!result.authentication.has_oauth2());
        assert!(result.authentication.find_oauth2_endpoints().is_none());
    }

    #[test]
    fn test_find_oauth2_endpoints_only() {
        let json = r#"{
            "authentication": {
                "oauth2": {
                    "authorize": "https://example.com/oauth/authorize",
                    "token": "https://example.com/oauth/token"
                }
            }
        }"#;
        let result = serde_json::from_str::<WpApiDetailsAuthenticationMapWrapper>(json)
            .expect("Failed to parse json");
        assert!(result.authentication.has_oauth2());
        let endpoints = result.authentication.find_oauth2_endpoints().unwrap();
        assert_eq!(
            endpoints.authorization_url,
            "https://example.com/oauth/authorize"
        );
        assert_eq!(endpoints.token_url, "https://example.com/oauth/token");
    }

    fn test_parse_wp_api_details_authentication_map_helper(json: &str) {
        let result = serde_json::from_str::<WpApiDetailsAuthenticationMapWrapper>(json);
        assert!(
            result.is_ok(),
            "Failed to parse json as `WpApiDetailsAuthenticationMap`"
        );
        assert_eq!(
            result
                .expect("Already verified result is Ok")
                .authentication
                .find_application_passwords_authentication_url(),
            Some("http://localhost/wp-admin/authorize-application.php".to_string())
        );
    }

    #[test]
    fn test_parse_empty_vec_as_wp_api_details_authentication_map() {
        let json = r#"{"authentication": []}"#;
        let result = serde_json::from_str::<WpApiDetailsAuthenticationMapWrapper>(json);
        assert!(
            result.is_ok(),
            "Failed to parse '[]' as `WpApiDetailsAuthenticationMap`"
        );
        assert!(
            result
                .expect("Already verified result is Ok")
                .authentication
                .0
                .is_empty()
        );
    }

    #[rstest]
    #[case("api-details/test-case-01.json")]
    #[case("api-details/test-case-02.json")]
    #[case("api-details/test-case-03.json")]
    #[case("api-details/test-case-04.json")]
    #[case("api-details/test-case-05.json")]
    #[case("api-details/test-case-06.json")]
    #[case("api-details/test-case-07.json")]
    fn test_api_details_json(#[case] input: &str) {
        let json = test_json(input).expect("Failed to read test resource");

        let result = WpApiDetails::try_from(json.as_slice());

        assert!(
            result.is_ok(),
            "Failed to parse json as `WpApiDetails`: {result:#?}"
        );
    }

    #[test]
    fn test_has_namespace() {
        let json: Vec<u8> =
            test_json("api-details/test-case-03.json").expect("Failed to read test resource");
        let result = WpApiDetails::try_from(json.as_slice());
        assert!(
            result.is_ok(),
            "Failed to parse json as `WpApiDetails`: {result:#?}"
        );

        let unwrapped_result = result.unwrap();

        assert!(unwrapped_result.has_namespace("jetpack/v4".to_string()));
        assert!(!unwrapped_result.has_namespace("jetpack/v2".to_string()));
    }

    #[rstest]
    #[case("context", Some(JsonValue::String("edit".to_string())))]
    #[case("jetpack_blocks_disabled", Some(JsonValue::Bool(false)))]
    #[case("jetpack_portfolio_posts_per_page", Some(JsonValue::Int(10)))]
    #[case("show", Some(JsonValue::Array(vec![JsonValue::String("post".to_string())])))]
    #[case("sharing_services", Some(JsonValue::Object(HashMap::from([("visible".to_string(), JsonValue::Array(vec![JsonValue::String("facebook".to_string()), JsonValue::String("x".to_string())])), ("hidden".to_string(), JsonValue::Array(vec![]))]))))]
    fn test_route_args(#[case] argument_name: &str, #[case] expected_result: Option<JsonValue>) {
        let json: Vec<u8> =
            test_json("api-details/test-case-03.json").expect("Failed to read test resource");
        let result = WpApiDetails::try_from(json.as_slice());

        assert!(
            result.is_ok(),
            "Failed to parse json as `WpApiDetails`: {result:#?}"
        );

        let unwrapped_result = result.unwrap();

        assert!(unwrapped_result.has_route("/jetpack/v4/settings".to_string()));
        let route = unwrapped_result.routes.get("/jetpack/v4/settings").unwrap();

        let argument = route
            .endpoints
            .first()
            .unwrap()
            .args
            .get(argument_name)
            .unwrap();
        assert_eq!(argument.default, expected_result);
    }

    #[test]
    fn test_has_route() {
        let json: Vec<u8> =
            test_json("api-details/test-case-03.json").expect("Failed to read test resource");
        let result = WpApiDetails::try_from(json.as_slice());
        assert!(
            result.is_ok(),
            "Failed to parse json as `WpApiDetails`: {result:#?}"
        );

        let unwrapped_result = result.unwrap();

        assert!(unwrapped_result.has_route("/jetpack/v4/backup-helper-script".to_string()));
        assert!(!unwrapped_result.has_route("/jetpack/v4/fake-endpoint".to_string()));
    }

    fn wp_org_resolver() -> WpOrgSiteApiUrlResolver {
        WpOrgSiteApiUrlResolver::new(
            ParsedUrl::parse("https://example.com/wp-json")
                .expect("Valid URL")
                .into(),
        )
    }

    fn wp_com_resolver(site_id: &str) -> WpComDotOrgApiUrlResolver {
        WpComDotOrgApiUrlResolver::new(site_id.to_string(), crate::wp_com::WpComBaseUrl::Production)
    }

    fn api_details_with_routes(routes: Vec<&str>) -> WpApiDetails {
        let routes = routes
            .into_iter()
            .map(|r| {
                (
                    r.to_string(),
                    WpRoute {
                        namespace: String::new(),
                        methods: vec!["GET".to_string()],
                        endpoints: vec![],
                    },
                )
            })
            .collect();
        WpApiDetails {
            name: String::new(),
            description: String::new(),
            url: "https://example.com".to_string(),
            home: "https://example.com".to_string(),
            gmt_offset: None,
            timezone_string: None,
            namespaces: vec![],
            authentication: WpApiDetailsAuthenticationMap(HashMap::new()),
            site_icon_url: None,
            routes,
        }
    }

    // WP.org: has_route_for_endpoint matches existing routes
    #[rstest]
    #[case("/wp/v2", "posts", "/wp/v2/posts")]
    #[case("/wp/v2", "posts/123", "/wp/v2/posts/123")]
    #[case("/wp-block-editor/v1", "settings", "/wp-block-editor/v1/settings")]
    #[case(
        "/wp-site-health/v1",
        "tests/background",
        "/wp-site-health/v1/tests/background"
    )]
    fn test_has_route_for_endpoint_wp_org_found(
        #[case] namespace: &str,
        #[case] endpoint_path: &str,
        #[case] route_key: &str,
    ) {
        let details = api_details_with_routes(vec![route_key]);
        let resolver = wp_org_resolver();
        assert!(details.has_route_for_endpoint(
            &resolver,
            namespace.to_string(),
            endpoint_path.to_string()
        ));
    }

    // WP.org: has_route_for_endpoint returns false for missing routes
    #[rstest]
    #[case("/wp/v2", "fake-endpoint")]
    #[case("/wp-block-editor/v1", "nonexistent")]
    #[case("/wp/v2", "posts/999/revisions")]
    fn test_has_route_for_endpoint_wp_org_not_found(
        #[case] namespace: &str,
        #[case] endpoint_path: &str,
    ) {
        let details = api_details_with_routes(vec!["/wp/v2/posts", "/wp-block-editor/v1/settings"]);
        let resolver = wp_org_resolver();
        assert!(!details.has_route_for_endpoint(
            &resolver,
            namespace.to_string(),
            endpoint_path.to_string()
        ));
    }

    // WP.com: has_route_for_endpoint matches routes with sites/{site_id} inserted
    #[rstest]
    #[case("/wp/v2", "posts", "/wp/v2/sites/mobile.blog/posts")]
    #[case("/wp/v2", "posts/123", "/wp/v2/sites/mobile.blog/posts/123")]
    #[case(
        "/wp-block-editor/v1",
        "settings",
        "/wp-block-editor/v1/sites/mobile.blog/settings"
    )]
    #[case(
        "/wp-site-health/v1",
        "tests/background",
        "/wp-site-health/v1/sites/mobile.blog/tests/background"
    )]
    fn test_has_route_for_endpoint_wp_com_found(
        #[case] namespace: &str,
        #[case] endpoint_path: &str,
        #[case] route_key: &str,
    ) {
        let details = api_details_with_routes(vec![route_key]);
        let resolver = wp_com_resolver("mobile.blog");
        assert!(details.has_route_for_endpoint(
            &resolver,
            namespace.to_string(),
            endpoint_path.to_string()
        ));
    }

    // WP.com: has_route_for_endpoint returns false for missing routes
    #[rstest]
    #[case("/wp/v2", "fake-endpoint")]
    #[case("/wp-block-editor/v1", "nonexistent")]
    fn test_has_route_for_endpoint_wp_com_not_found(
        #[case] namespace: &str,
        #[case] endpoint_path: &str,
    ) {
        let details = api_details_with_routes(vec![
            "/wp/v2/sites/mobile.blog/posts",
            "/wp-block-editor/v1/sites/mobile.blog/settings",
        ]);
        let resolver = wp_com_resolver("mobile.blog");
        assert!(!details.has_route_for_endpoint(
            &resolver,
            namespace.to_string(),
            endpoint_path.to_string()
        ));
    }

    // Same namespace+path resolves differently depending on the resolver
    #[test]
    fn test_has_route_for_endpoint_same_input_different_resolvers() {
        let wp_org_details = api_details_with_routes(vec!["/wp/v2/posts"]);
        let wp_com_details = api_details_with_routes(vec!["/wp/v2/sites/mobile.blog/posts"]);

        let org_resolver = wp_org_resolver();
        let com_resolver = wp_com_resolver("mobile.blog");

        // WP.org resolver matches WP.org routes
        assert!(wp_org_details.has_route_for_endpoint(
            &org_resolver,
            "/wp/v2".to_string(),
            "posts".to_string(),
        ));
        // WP.org resolver does NOT match WP.com routes
        assert!(!wp_com_details.has_route_for_endpoint(
            &org_resolver,
            "/wp/v2".to_string(),
            "posts".to_string(),
        ));

        // WP.com resolver matches WP.com routes
        assert!(wp_com_details.has_route_for_endpoint(
            &com_resolver,
            "/wp/v2".to_string(),
            "posts".to_string(),
        ));
        // WP.com resolver does NOT match WP.org routes
        assert!(!wp_org_details.has_route_for_endpoint(
            &com_resolver,
            "/wp/v2".to_string(),
            "posts".to_string(),
        ));
    }

    // WP.com: different site IDs produce different route keys
    #[test]
    fn test_has_route_for_endpoint_wp_com_different_site_ids() {
        let details = api_details_with_routes(vec!["/wp/v2/sites/mobile.blog/posts"]);
        let correct_resolver = wp_com_resolver("mobile.blog");
        let wrong_resolver = wp_com_resolver("other.blog");

        assert!(details.has_route_for_endpoint(
            &correct_resolver,
            "/wp/v2".to_string(),
            "posts".to_string(),
        ));
        assert!(!details.has_route_for_endpoint(
            &wrong_resolver,
            "/wp/v2".to_string(),
            "posts".to_string(),
        ));
    }

    // Verify has_route_for_endpoint works against real WP.org test fixture data
    #[test]
    fn test_has_route_for_endpoint_with_wp_org_fixture() {
        let json: Vec<u8> =
            test_json("api-details/test-case-03.json").expect("Failed to read test resource");
        let details = WpApiDetails::try_from(json.as_slice()).unwrap();
        let resolver = wp_org_resolver();

        assert!(details.has_route_for_endpoint(
            &resolver,
            "/wp-block-editor/v1".to_string(),
            "settings".to_string(),
        ));
        assert!(details.has_route_for_endpoint(
            &resolver,
            "/wp/v2".to_string(),
            "posts".to_string(),
        ));
        assert!(!details.has_route_for_endpoint(
            &resolver,
            "/wp/v2".to_string(),
            "fake-endpoint".to_string(),
        ));
    }

    // Regression test: a base path that contains the namespace as a substring
    // must not cause has_route_for_endpoint to mis-locate the route key.
    // This would have failed under a `find(&namespace)` approach on the URL path.
    #[test]
    fn test_has_route_for_endpoint_namespace_substring_in_base_path() {
        let resolver = WpOrgSiteApiUrlResolver::new(
            ParsedUrl::parse("https://example.com/wp/v2/api/wp-json")
                .expect("Valid URL")
                .into(),
        );
        let details = api_details_with_routes(vec!["/wp/v2/posts"]);

        assert!(details.has_route_for_endpoint(
            &resolver,
            "/wp/v2".to_string(),
            "posts".to_string(),
        ));
    }

    // Consistency test: the route key produced by `route_path` must match the
    // tail of the URL path produced by `resolve`. If someone changes the URL
    // structure in `resolve` without updating `route_path` (or vice versa),
    // this test will fail.
    #[rstest]
    #[case::wp_org_wp_v2("/wp/v2", "posts")]
    #[case::wp_org_wp_v2_id("/wp/v2", "posts/123")]
    #[case::wp_org_block_editor("/wp-block-editor/v1", "settings")]
    #[case::wp_org_site_health("/wp-site-health/v1", "tests/background")]
    fn test_route_path_matches_resolve_wp_org(
        #[case] namespace: &str,
        #[case] endpoint_path: &str,
    ) {
        let resolver = wp_org_resolver();
        let resolved = resolver
            .resolve(namespace.to_string(), vec![endpoint_path.to_string()])
            .parsed_url();
        let route_key = resolver.route_path(namespace.to_string(), endpoint_path.to_string());

        assert!(
            resolved.inner.path().ends_with(&route_key),
            "route_path `{}` is not the tail of resolved URL path `{}` — `route_path` and `resolve` are out of sync",
            route_key,
            resolved.inner.path()
        );
    }

    #[rstest]
    #[case::wp_com_wp_v2("/wp/v2", "posts")]
    #[case::wp_com_wp_v2_id("/wp/v2", "posts/123")]
    #[case::wp_com_block_editor("/wp-block-editor/v1", "settings")]
    #[case::wp_com_site_health("/wp-site-health/v1", "tests/background")]
    fn test_route_path_matches_resolve_wp_com(
        #[case] namespace: &str,
        #[case] endpoint_path: &str,
    ) {
        let resolver = wp_com_resolver("mobile.blog");
        let resolved = resolver
            .resolve(namespace.to_string(), vec![endpoint_path.to_string()])
            .parsed_url();
        let route_key = resolver.route_path(namespace.to_string(), endpoint_path.to_string());

        assert!(
            resolved.inner.path().ends_with(&route_key),
            "route_path `{}` is not the tail of resolved URL path `{}` — `route_path` and `resolve` are out of sync",
            route_key,
            resolved.inner.path()
        );
    }

    fn test_json(input: &str) -> Result<Vec<u8>, std::io::Error> {
        let mut file_path = std::path::PathBuf::from(env!("CARGO_WORKSPACE_DIR"));
        file_path.push("test-data");
        file_path.push(input);

        let mut f = std::fs::File::open(file_path)?;
        let mut buffer = Vec::new();

        // read the whole file
        f.read_to_end(&mut buffer)?;

        Ok(buffer)
    }

    #[derive(Debug, Deserialize)]
    struct WpApiDetailsAuthenticationMapWrapper {
        authentication: WpApiDetailsAuthenticationMap,
    }
}
