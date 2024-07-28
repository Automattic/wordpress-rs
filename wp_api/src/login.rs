use crate::serde_helper::deserialize_i64_or_string;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;
use std::sync::Arc;

pub use login_client::WpLoginClient;
pub use url_discovery::{UrlDiscoveryState, UrlDiscoverySuccess};

use crate::ParsedUrl;

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
}
