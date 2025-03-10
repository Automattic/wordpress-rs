use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;
use std::sync::Arc;
use wp_serde_helper::deserialize_i64_or_string;

use crate::login::url_discovery::is_local_dev_environment_url;
use crate::ParsedUrl;
use crate::WpUuid;

const KEY_APPLICATION_PASSWORDS: &str = "application-passwords";

pub mod login_client;
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

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Object)]
pub struct WpApiDetails {
    pub name: String,
    pub description: String,
    pub url: String,
    pub home: String,
    #[serde(deserialize_with = "deserialize_i64_or_string")]
    pub gmt_offset: i64,
    pub timezone_string: String,
    pub namespaces: Vec<String>,
    pub authentication: WpApiDetailsAuthenticationMap,
    pub site_icon_url: Option<String>,
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
    /// https://github.com/WordPress/wordpress-develop/blob/530493396b324f5bed518a494e2843e7fdb020f1/src/wp-includes/rest-api.php#L1099-L1119
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
        KnownApplicationPasswordBlockingPlugin::all()
            .iter()
            .any(|plugin| self.namespaces.contains(&plugin.namespace))
    }

    /// Returns a list of plugins that might be responsible for disabling application passwords.
    pub fn application_password_blocking_plugins(
        &self,
    ) -> Vec<KnownApplicationPasswordBlockingPlugin> {
        KnownApplicationPasswordBlockingPlugin::all()
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
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct KnownApplicationPasswordBlockingPlugin {
    /// The name of the plugin.
    pub name: String,
    /// The plugin's REST API namespace.
    pub namespace: String,
    /// A URL to the plugin's support page, where users can find help.
    pub support_url: String,
}

impl KnownApplicationPasswordBlockingPlugin {
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
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct WpRestApiAuthenticationScheme {
    pub endpoints: WpRestApiAuthenticationEndpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct WpRestApiAuthenticationEndpoint {
    pub authorization: String,
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

#[derive(Debug, Clone, Serialize)]
pub struct WpApiDetailsAuthenticationMap(HashMap<String, WpRestApiAuthenticationScheme>);

// If the response is `[]`, default to an empty `HashMap`
impl<'de> Deserialize<'de> for WpApiDetailsAuthenticationMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer
            .deserialize_any(wp_serde_helper::DeserializeEmptyVecOrT::<
                HashMap<String, WpRestApiAuthenticationScheme>,
            >::new(Box::new(HashMap::new)))
            .map(Self)
    }
}

impl WpApiDetailsAuthenticationMap {
    pub fn has_application_passwords_authentication_url(&self) -> bool {
        self.0.contains_key(KEY_APPLICATION_PASSWORDS)
    }

    pub fn find_application_passwords_authentication_url(&self) -> Option<String> {
        self.0
            .get(KEY_APPLICATION_PASSWORDS)
            .map(|auth_scheme| auth_scheme.endpoints.authorization.clone())
    }
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

    #[test]
    fn test_parse_wp_api_details_authentication_map() {
        let json = r#"{
          "authentication": {
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
        assert!(result
            .expect("Already verified result is Ok")
            .authentication
            .0
            .is_empty());
    }

    #[derive(Debug, Deserialize)]
    struct WpApiDetailsAuthenticationMapWrapper {
        authentication: WpApiDetailsAuthenticationMap,
    }
}
