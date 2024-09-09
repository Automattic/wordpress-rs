use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;
use std::sync::Arc;
use wp_serde_helper::deserialize_i64_or_string;

pub use login_client::WpLoginClient;
pub use url_discovery::{UrlDiscoveryState, UrlDiscoverySuccess};

use crate::ParsedUrl;
use crate::WpUuid;

const KEY_APPLICATION_PASSWORDS: &str = "application-passwords";
const KEY_OAUTH_2: &str = "oauth2";

mod login_client;
mod url_discovery;

#[derive(Debug, uniffi::Record)]
pub struct WpRestApiUrls {
    api_details: Arc<WpApiDetails>,
    api_root_url: String,
}

// After a successful login, the system will receive an OAuth callback with the login details
// embedded as query params. This function parses that URL and extracts the login details as an object.
#[uniffi::export]
pub fn extract_login_details_from_url(
    url: Arc<ParsedUrl>,
) -> Result<WpApiApplicationPasswordDetails, OAuthResponseUrlError> {
    let f = |key| {
        url.inner
            .query_pairs()
            .find_map(|(k, v)| (k == key).then_some(v.to_string()))
    };
    if let Some(is_success) = f("success") {
        if is_success == "false" {
            return Err(OAuthResponseUrlError::UnsuccessfulLogin);
        }
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

#[derive(Debug, Serialize, Deserialize, uniffi::Object)]
pub struct WpApiDetails {
    pub name: String,
    pub description: String,
    pub url: String,
    pub home: String,
    #[serde(deserialize_with = "deserialize_i64_or_string")]
    pub gmt_offset: i64,
    pub timezone_string: String,
    pub namespaces: Vec<String>,
    pub authentication: HashMap<String, AuthenticationProtocol>,
    pub site_icon_url: Option<String>,
}

#[uniffi::export]
impl WpApiDetails {
    pub fn find_application_passwords_authentication_url(&self) -> Option<String> {
        match self.authentication.get(KEY_APPLICATION_PASSWORDS) {
            Some(AuthenticationProtocol::ApplicationPassword(scheme)) => {
                Some(scheme.endpoints.authorization.clone())
            }
            _ => None,
        }
    }

    pub fn find_oauth_server_details(&self) -> Option<WpApiOAuth2ServerDetails> {
        match self.authentication.get(KEY_OAUTH_2) {
            Some(AuthenticationProtocol::OAuth2(scheme)) => Some(scheme.clone().into()),
            _ => None,
        }
    }

    pub fn registered_authentication_methods(&self) -> Vec<WpAuthenticationProtocol> {
        let mut methods: Vec<WpAuthenticationProtocol> = vec![];

        for (name, protocol) in &self.authentication {
            methods.push(protocol.clone().into())
        }

        methods
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AuthenticationProtocol {
    OAuth2(OAuth2Scheme),
    ApplicationPassword(ApplicationPasswordScheme),
    Other(UnknownAuthenticationData),
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum WpAuthenticationProtocol {
    OAuth2(WpApiOAuth2ServerDetails),
    ApplicationPassword(String),
    Other(UnknownAuthenticationData),
}

impl From<AuthenticationProtocol> for WpAuthenticationProtocol {
    fn from(protocol: AuthenticationProtocol) -> Self {
        match protocol {
            AuthenticationProtocol::OAuth2(scheme) => {
                WpAuthenticationProtocol::OAuth2(scheme.clone().into())
            }
            AuthenticationProtocol::ApplicationPassword(scheme) => {
                WpAuthenticationProtocol::ApplicationPassword(
                    scheme.endpoints.authorization.clone(),
                )
            }
            AuthenticationProtocol::Other(scheme) => {
                WpAuthenticationProtocol::Other(scheme.clone())
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, uniffi::Enum)]
#[serde(untagged)]
pub enum UnknownAuthenticationData {
    Bool(bool),
    Int(i64),
    String(String),
    Float(f64),
    Object(HashMap<String, UnknownAuthenticationData>),
    Dictionary(HashMap<String, String>),
    List(Vec<UnknownAuthenticationData>),
}

/// An internal JSON representation of the WP Core `application-passwords` authentication method.
///
#[derive(Debug, Serialize, Deserialize, Clone, uniffi::Record)]
pub struct ApplicationPasswordScheme {
    endpoints: ApplicationPasswordEndpoints,
}

/// An internal JSON representation of the WP Core `application-passwords` authentication method's endpoints.
#[derive(Debug, Serialize, Deserialize, Clone, uniffi::Record)]
pub struct ApplicationPasswordEndpoints {
    authorization: String,
}

/// An internal JSON representation of an `oauth2` authentication method as provided by https://wordpress.org/plugins/oauth2-provider/
/// Provides a fallback for servers that use an `endpoints` key to match the `application-passwords` method.
///
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum OAuth2Scheme {
    WithoutEndpointKey(OAuth2SchemeWithoutEndpoint),
    WithEndpointKey(OAuth2SchemeWithEndpoint),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuth2SchemeWithEndpoint {
    endpoints: OAuth2Endpoints,
}

/// An internal JSON representation of the `oauth2` authentication method's endpoints.
///
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuth2Endpoints {
    authorization: String,
    token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuth2SchemeWithoutEndpoint {
    authorize: String,
    token: String,
}

/// A derived representation of `OAuth2Scheme` for clients that normalizes the fields
///
#[derive(Debug, Serialize, Deserialize, Clone, uniffi::Record)]
pub struct WpApiOAuth2ServerDetails {
    pub authorization_url: String,
    pub token_url: String,
}

impl From<OAuth2Scheme> for WpApiOAuth2ServerDetails {
    fn from(scheme: OAuth2Scheme) -> Self {
        match scheme {
            OAuth2Scheme::WithoutEndpointKey(subscheme) => WpApiOAuth2ServerDetails {
                authorization_url: subscheme.authorize.clone(),
                token_url: subscheme.token.clone(),
            },
            OAuth2Scheme::WithEndpointKey(subscheme) => WpApiOAuth2ServerDetails {
                authorization_url: subscheme.endpoints.authorization.clone(),
                token_url: subscheme.endpoints.token.clone(),
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, uniffi::Record)]
pub struct WpApiApplicationPasswordDetails {
    pub site_url: String,
    pub user_login: String,
    pub password: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, thiserror::Error, uniffi::Error)]
pub enum OAuthResponseUrlError {
    #[error("The given URL is missing the `site_url` query parameter")]
    MissingSiteUrl,
    #[error("The given URL is missing the `username` query parameter")]
    MissingUsername,
    #[error("The given URL is missing the `password` query parameter")]
    MissingPassword,
    #[error("Unsuccessful Login")]
    UnsuccessfulLogin,
}

/// Return a URL to be used in application password authentication.
///
/// See the "Authorization Flow" section for details:
/// https://make.wordpress.org/core/2020/11/05/application-passwords-integration-guide/
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
            "https://example.com/wp-login.php?app_name=AppName&app_id={}&success_url=https%3A%2F%2Fexample.com%2Fsuccess&reject_url=https%3A%2F%2Fexample.com%2Freject",
            app_id_str
        );
        assert_eq!(auth_url, ParsedUrl::parse(expected_url.as_str()).unwrap());
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct AuthenticationTest {
        authentication: HashMap<String, AuthenticationProtocol>,
    }

    #[rstest]
    #[case(r#"{ "authentication": { } }"#)]
    // #[case(r#"{ "authentication": [ ] }"#)] // TODO
    fn test_empty_authentication_can_be_parsed(#[case] input_json: &str) {
        let test_object: AuthenticationTest = serde_json::from_str(input_json).unwrap();
        assert!(test_object.authentication.is_empty())
    }

    #[rstest]
    fn test_authentication_with_valid_application_passwords() {
        let input_json = r#"
        { "authentication": { "application-passwords": { "endpoints": { "authorization": "http:\/\/localhost\/wp-admin\/authorize-application.php" } } } }"#;
        let test_object: AuthenticationTest = serde_json::from_str(input_json).unwrap();
        assert!(matches!(
            test_object
                .authentication
                .get(KEY_APPLICATION_PASSWORDS)
                .unwrap(),
            AuthenticationProtocol::ApplicationPassword(_)
        ));
    }

    #[rstest]
    #[case(r#"{ "authentication": { "application-passwords": { } } }"#)]
    #[case(r#"{ "authentication": { "application-passwords": [ ] } }"#)]
    #[case(r#"{ "authentication": { "application-passwords": { "disabled": true } } }"#)]
    #[case(r#"{ "authentication": { "application-passwords": { "florps": 42 } } }"#)]
    #[case(r#"{ "authentication": { "application-passwords": { "florps": -42 } } }"#)]
    #[case(r#"{ "authentication": { "application-passwords": { "florps": 0.5234 } } }"#)]
    fn test_authentication_with_invalid_application_passwords_is_other(#[case] input_json: &str) {
        let test_object: AuthenticationTest = serde_json::from_str(input_json).unwrap();
        assert!(matches!(
            test_object
                .authentication
                .get(KEY_APPLICATION_PASSWORDS)
                .unwrap(),
            AuthenticationProtocol::Other(_)
        ))
    }

    #[rstest]
    #[case(r#"{ "authentication": { "oauth2": { "authorize": "http:\/\/localhost\/oauth\/authorize", "token": "http:\/\/localhost\/oauth\/token", "me": "http:\/\/localhost\/oauth\/me", "version": "2.0", "software": "WP OAuth Server" } } }"#)]
    #[case(r#"{ "authentication": { "oauth2": { "endpoints": { "authorization": "https:\/\/public-api.wordpress.com\/oauth2\/authorize", "token": "https:\/\/public-api.wordpress.com\/oauth2\/token" } } } }"#)]
    fn test_authentication_with_valid_oauth2(#[case] input_json: &str) {
        let test_object: AuthenticationTest = serde_json::from_str(input_json).unwrap();
        println!("{:?}", test_object);
        assert!(matches!(
            test_object.authentication.get(KEY_OAUTH_2).unwrap(),
            AuthenticationProtocol::OAuth2(_)
        ));
    }
}
