use crate::{
    JsonValue, login::url_discovery::is_local_dev_environment_url, parsed_url::ParsedUrl,
    uuid::WpUuid,
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

pub mod login_client;
pub mod nonce;
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

    /// Returns `true` if the site has routes matching the given namespace.
    pub fn has_namespace(&self, namespace: String) -> bool {
        self.namespaces.contains(&namespace)
    }

    /// Returns `true` if the site has the given route.
    pub fn has_route(&self, route: String) -> bool {
        self.routes.contains_key(&route)
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, uniffi::Record)]
pub struct WpRestApiAuthenticationScheme {
    pub endpoints: Option<WpRestApiAuthenticationEndpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, uniffi::Record)]
pub struct WpRestApiAuthenticationEndpoint {
    pub authorization: String,
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
            .and_then(|auth_scheme| {
                auth_scheme
                    .endpoints
                    .as_ref()
                    .map(|e| e.authorization.clone())
            })
    }
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
