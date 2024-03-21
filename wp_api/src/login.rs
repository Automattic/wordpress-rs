use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;

// After a successful login, the system will receive an OAuth callback with the login details
// embedded as query params. This function parses that URL and extracts the login details as an object.
#[derive(Debug, PartialEq, uniffi::Object)]
pub struct OAuthResponseUrl {
    string_value: String,
}

#[uniffi::export]
impl OAuthResponseUrl {
    #[uniffi::constructor]
    pub fn new(string_value: String) -> Self {
        Self { string_value }
    }

    pub fn get_password_details(
        &self,
    ) -> Result<WPAPIApplicationPasswordDetails, OAuthResponseUrlError> {
        let mut builder = WPAPIApplicationPasswordDetails::builder();

        let url =
            url::Url::parse(&self.string_value).map_err(|err| OAuthResponseUrlError::InvalidUrl)?;

        for pair in url.query_pairs() {
            match pair.0.to_string().as_str() {
                "site_url" => builder = builder.site_url(pair.1.to_string()),
                "user_login" => builder = builder.user_login(pair.1.to_string()),
                "password" => builder = builder.password(pair.1.to_string()),
                "success" => {
                    if pair.1 == "false" {
                        return Err(OAuthResponseUrlError::UnsuccessfulLogin);
                    }
                }
                _ => (),
            };
        }

        builder.build() //.map_err(|err| UrlParsingError::InvalidUrl)
    }
}

impl From<&str> for OAuthResponseUrl {
    fn from(str: &str) -> Self {
        OAuthResponseUrl {
            string_value: str.to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPAPIDetails {
    pub name: String,
    pub description: String,
    pub url: String,
    pub home: String,
    pub gmt_offset: String,
    pub timezone_string: String,
    pub namespaces: Vec<String>,
    pub authentication: HashMap<String, WPRestAPIAuthenticationScheme>,
    pub site_icon_url: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPRestAPIAuthenticationScheme {
    pub endpoints: WPRestApiAuthenticationEndpoint,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WPRestApiAuthenticationEndpoint {
    pub authorization: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct WPAPIApplicationPasswordDetails {
    pub site_url: String,
    pub user_login: String,
    pub password: String,
}

impl WPAPIApplicationPasswordDetails {
    fn builder() -> WPAPIApplicationPasswordDetailsBuilder {
        WPAPIApplicationPasswordDetailsBuilder::default()
    }
}

#[derive(Default)]
struct WPAPIApplicationPasswordDetailsBuilder {
    site_url: Option<String>,
    user_login: Option<String>,
    password: Option<String>,
}

impl WPAPIApplicationPasswordDetailsBuilder {
    fn site_url(mut self, site_url: String) -> Self {
        self.site_url = Some(site_url);
        self
    }

    fn user_login(mut self, user_login: String) -> Self {
        self.user_login = Some(user_login);
        self
    }

    fn password(mut self, password: String) -> Self {
        self.password = Some(password);
        self
    }

    fn build(self) -> Result<WPAPIApplicationPasswordDetails, OAuthResponseUrlError> {
        let site_url = if let Some(site_url) = self.site_url {
            site_url
        } else {
            return Err(OAuthResponseUrlError::MissingSiteUrl);
        };

        let user_login = if let Some(user_login) = self.user_login {
            user_login
        } else {
            return Err(OAuthResponseUrlError::MissingUsername);
        };

        let password = if let Some(password) = self.password {
            password
        } else {
            return Err(OAuthResponseUrlError::MissingPassword);
        };

        Ok(WPAPIApplicationPasswordDetails {
            site_url,
            user_login,
            password,
        })
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum OAuthResponseUrlError {
    #[error("Invalid URL")]
    InvalidUrl,

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
mod oauth_response_url_tests {
    use super::*;

    #[test]
    fn can_be_initialized() {
        assert_eq!(OAuthResponseUrl::new("foo".to_string()), OAuthResponseUrl::from("foo"))
    }

    #[test]
    fn creates_password_details_for_valid_url() {
        let url = OAuthResponseUrl::from(
            "exampleauth://login?site_url=http://example.com&user_login=test&password=1234",
        );

        assert_eq!(
            url.get_password_details().unwrap(),
            default_password_details()
        );
    }

    #[test]
    fn ignores_extra_query_params_for_valid_url() {
        let url = OAuthResponseUrl::from(
            "exampleauth://login?site_url=http://example.com&user_login=test&password=1234&foo=bar",
        );

        assert_eq!(
            url.get_password_details().unwrap(),
            default_password_details()
        );
    }

    #[test]
    fn throws_error_for_missing_site_url() {
        let result = OAuthResponseUrl::from("exampleauth://login?user_login=test&password=1234")
            .get_password_details();
        assert!(matches!(result, Err(OAuthResponseUrlError::MissingSiteUrl)));
    }

    #[test]
    fn throws_error_for_missing_user_login() {
        let result =
            OAuthResponseUrl::from("exampleauth://login?site_url=http://example.com&password=1234")
                .get_password_details();
        assert!(matches!(
            result,
            Err(OAuthResponseUrlError::MissingUsername)
        ));
    }

    #[test]
    fn throws_error_for_missing_password() {
        let result = OAuthResponseUrl::from(
            "exampleauth://login?site_url=http://example.com&user_login=test",
        )
        .get_password_details();
        assert!(matches!(
            result,
            Err(OAuthResponseUrlError::MissingPassword)
        ));
    }

    #[test]
    fn throws_error_for_unsuccessful_login() {
        let result =
            OAuthResponseUrl::from("exampleauth://login?success=false").get_password_details();
        assert!(matches!(
            result,
            Err(OAuthResponseUrlError::UnsuccessfulLogin)
        ));
    }

    #[test]
    fn throws_appropriate_error_for_malformed_response() {
        let result =
            OAuthResponseUrl::from("exampleauth://login?success=true").get_password_details();
        assert!(matches!(result, Err(OAuthResponseUrlError::MissingSiteUrl)));
    }

    fn default_password_details() -> WPAPIApplicationPasswordDetails {
        WPAPIApplicationPasswordDetails::builder()
            .site_url("http://example.com".to_string())
            .user_login("test".to_string())
            .password("1234".to_string())
            .build()
            .unwrap()
    }
}
