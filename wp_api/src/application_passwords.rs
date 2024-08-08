use std::{fmt::Display, sync::Arc};

use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;

use crate::ParsedUrl;

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseApplicationPassword {
    #[WpContext(edit, embed, view)]
    pub uuid: Option<ApplicationPasswordUuid>,
    #[WpContext(edit, embed, view)]
    pub app_id: Option<ApplicationPasswordAppId>,
    #[WpContext(edit, embed, view)]
    pub name: Option<String>,
    #[WpContext(edit, view)]
    pub created: Option<String>,
    #[WpContextualOption]
    #[WpContext(edit, view)]
    pub last_used: Option<String>,
    #[WpContextualOption]
    #[WpContext(edit, view)]
    pub last_ip: Option<IpAddress>,
    #[WpContextualOption]
    #[WpContext(edit)]
    pub password: Option<String>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, uniffi::Record)]
#[serde(transparent)]
pub struct ApplicationPasswordUuid {
    pub uuid: String,
}

impl Display for ApplicationPasswordUuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.uuid)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, uniffi::Record)]
#[serde(transparent)]
pub struct ApplicationPasswordAppId {
    pub app_id: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, uniffi::Record)]
#[serde(transparent)]
pub struct IpAddress {
    #[serde(alias = "last_ip")]
    pub value: String,
}

#[derive(Debug, Serialize, uniffi::Record)]
pub struct ApplicationPasswordCreateParams {
    /// A UUID provided by the application to uniquely identify it.
    /// It is recommended to use an UUID v5 with the URL or DNS namespace.
    pub app_id: Option<String>,
    /// The name of the application password.
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct ApplicationPasswordDeleteResponse {
    pub deleted: bool,
    pub previous: ApplicationPasswordWithEditContext,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct ApplicationPasswordDeleteAllResponse {
    pub deleted: bool,
    pub count: i32,
}

#[derive(Debug, Serialize, uniffi::Record)]
pub struct ApplicationPasswordUpdateParams {
    /// A UUID provided by the application to uniquely identify it.
    /// It is recommended to use an UUID v5 with the URL or DNS namespace.
    pub app_id: Option<String>,
    /// The name of the application password.
    pub name: String,
}

#[derive(Debug, uniffi::Object)]
pub struct ApplicationPasswordAuthenticationRequest {
    pub app_name: String,
    pub app_id: Option<String>,
    pub success_url: Option<String>,
    pub reject_url: Option<String>,
}

#[uniffi::export]
impl ApplicationPasswordAuthenticationRequest {
    #[uniffi::constructor]
    pub fn new(
        app_name: String,
        app_id: Option<String>,
        success_url: Option<String>,
        reject_url: Option<String>,
    ) -> Self {
        Self {
            app_name,
            app_id,
            success_url,
            reject_url,
        }
    }

    pub fn auth_url(&self, login_url: Arc<ParsedUrl>) -> ParsedUrl {
        let mut auth_url = login_url.inner.clone();
        auth_url
            .query_pairs_mut()
            .append_pair("app_name", &self.app_name);
        if let Some(app_id) = &self.app_id {
            auth_url.query_pairs_mut().append_pair("app_id", app_id);
        }
        if let Some(success_url) = &self.success_url {
            auth_url
                .query_pairs_mut()
                .append_pair("success_url", success_url);
        }
        if let Some(reject_url) = &self.reject_url {
            auth_url
                .query_pairs_mut()
                .append_pair("reject_url", reject_url);
        }
        ParsedUrl::new(auth_url)
    }
}

#[derive(Debug, uniffi::Object)]
pub struct ApplicationPasswordAuthenticationSuccess {
    pub site_url: ParsedUrl,
    pub username: String,
    pub password: String,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum ApplicationPasswordAuthenticationError {
    #[error("Application Password authentication request was rejected")]
    RequestRejected,

    #[error("Missing query in the callback URL: {}", query)]
    MissingQuery { query: String },

    #[error("Invalid site url in the callback url")]
    InvalidSiteUrl,
}

#[uniffi::export]
impl ApplicationPasswordAuthenticationSuccess {
    #[uniffi::constructor]
    pub fn new(
        callback_url: Arc<ParsedUrl>,
    ) -> Result<Self, ApplicationPasswordAuthenticationError> {
        let mut rejected = false;
        let mut site_url: Option<String> = None;
        let mut username: Option<String> = None;
        let mut password: Option<String> = None;

        for (key, value) in callback_url.inner.query_pairs() {
            match key.as_ref() {
                "success" => {
                    rejected = value == "false";
                }
                "site_url" => {
                    site_url = Some(value.to_string());
                }
                "user_login" => {
                    username = Some(value.to_string());
                }
                "password" => {
                    password = Some(value.to_string());
                }
                _ => {}
            }
        }

        if rejected {
            return Err(ApplicationPasswordAuthenticationError::RequestRejected);
        }

        let username = username.ok_or(ApplicationPasswordAuthenticationError::MissingQuery {
            query: "user_login".to_string(),
        })?;
        let password = password.ok_or(ApplicationPasswordAuthenticationError::MissingQuery {
            query: "password".to_string(),
        })?;
        let site_url = site_url.ok_or(ApplicationPasswordAuthenticationError::MissingQuery {
            query: "site_url".to_string(),
        })?;
        let site_url = ParsedUrl::parse(site_url.as_str())
            .map_err(|_| ApplicationPasswordAuthenticationError::InvalidSiteUrl)?;

        Ok(Self {
            site_url,
            username,
            password,
        })
    }

    fn site_url(&self) -> ParsedUrl {
        self.site_url.clone()
    }

    fn username(&self) -> String {
        self.username.clone()
    }

    fn password(&self) -> String {
        self.password.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    fn test_auth_url() {
        let login_url = ParsedUrl::parse("https://example.com/wp-login.php").unwrap();
        let request = ApplicationPasswordAuthenticationRequest::new(
            "AppName".to_string(),
            Some("app-id".to_string()),
            Some("https://example.com/success".to_string()),
            Some("https://example.com/reject".to_string()),
        );
        let auth_url = request.auth_url(Arc::new(login_url));
        assert_eq!(
            auth_url,
            ParsedUrl::parse(
                "https://example.com/wp-login.php?app_name=AppName&app_id=app-id&success_url=https%3A%2F%2Fexample.com%2Fsuccess&reject_url=https%3A%2F%2Fexample.com%2Freject"
            )
            .unwrap()
        );
    }

    #[rstest]
    #[case("login://callback?user_login=admin&password=123456", "site_url")]
    #[case(
        "login://callback?site_url=https%3A%2F%2Fexample.com&password=123456",
        "user_login"
    )]
    #[case(
        "login://callback?site_url=https%3A%2F%2Fexample.com&user_login=admin",
        "password"
    )]
    fn test_missing_query_error(#[case] url: String, #[case] missing_query: String) {
        let url = ParsedUrl::parse(url.as_str()).unwrap();
        let error = ApplicationPasswordAuthenticationSuccess::new(url.into()).unwrap_err();
        assert_eq!(
            error,
            ApplicationPasswordAuthenticationError::MissingQuery {
                query: missing_query
            }
        );
    }

    #[rstest]
    fn test_invalid_site_url_error() {
        let url =
            ParsedUrl::parse("login://callback?site_url=invalid&user_login=admin&password=123456")
                .unwrap();
        let error = ApplicationPasswordAuthenticationSuccess::new(url.into()).unwrap_err();
        assert_eq!(
            error,
            ApplicationPasswordAuthenticationError::InvalidSiteUrl
        );
    }

    #[rstest]
    fn test_successful_result_can_be_parsed() {
        let url = ParsedUrl::parse("x-wordpress-app://login-callback?site_url=https%3A%2F%2Fexample.com&user_login=admin&password=123456").unwrap();
        let success = ApplicationPasswordAuthenticationSuccess::new(url.into()).unwrap();
        assert_eq!(
            success.site_url(),
            ParsedUrl::parse("https://example.com").unwrap()
        );
        assert_eq!(success.username(), "admin");
        assert_eq!(success.password(), "123456");
    }

    fn test_rejected_result_can_be_parsed() {
        let url = ParsedUrl::parse("x-wordpress-app://login-callback?success=false").unwrap();
        let error = ApplicationPasswordAuthenticationSuccess::new(url.into()).unwrap_err();
        assert_eq!(
            error,
            ApplicationPasswordAuthenticationError::RequestRejected
        );
    }
}
