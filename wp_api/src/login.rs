use crate::serde_helper::deserialize_i64_or_string;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;
use std::sync::Arc;

pub use login_client::WpLoginClient;
pub use url_discovery::{UrlDiscoveryState, UrlDiscoverySuccess};

use crate::ParsedUrl;
use crate::WpUuid;

const KEY_APPLICATION_PASSWORDS: &str = "application-passwords";

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
    pub authentication: HashMap<String, WpRestApiAuthenticationScheme>,
    pub site_icon_url: Option<String>,
}

#[uniffi::export]
impl WpApiDetails {
    pub fn find_application_passwords_authentication_url(&self) -> Option<String> {
        self.authentication
            .get(KEY_APPLICATION_PASSWORDS)
            .map(|auth_scheme| auth_scheme.endpoints.authorization.clone())
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WpRestApiAuthenticationScheme {
    pub endpoints: WpRestApiAuthenticationEndpoint,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
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

/// Return an URL to be used in application password authentication.
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
}
