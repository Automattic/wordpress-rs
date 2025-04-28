use crate::client::JetpackApiClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wp_api::{
    ParsedUrl, WpApiClientDelegate, WpApiError, WpErrorCode,
    auth::{WpAuthentication, WpAuthenticationProvider},
    users::UserId,
};
use wp_com::{
    WpComSiteId, client::WpComApiClient, jetpack_connection::JetpackRemoteConnectionParams,
};

#[derive(Debug, Serialize, uniffi::Record)]
pub struct JetpackConnectionParams {
    /// origination of the request, e.g. "jetpack-app"
    pub from: String,
    /// the plugin being connected, e.g. "jetpack"
    pub plugin_slug: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct JetpackConnectionRegisterResult {
    pub authorize_url: String,
}

impl JetpackConnectionRegisterResult {
    fn blog_id(&self) -> Option<WpComSiteId> {
        self.authorize_url
            .parse::<url::Url>()
            .ok()
            .and_then(|url| {
                url.query_pairs()
                    .find(|(key, _)| key == "client_id")
                    .map(|(_, value)| value.to_string())
            })
            .and_then(|str| str.parse().ok())
    }
}

#[derive(Debug, Serialize, uniffi::Record)]
pub struct JetpackRemoteProvisionParams;

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct JetpackRemoteProvisionResult {
    pub jp_version: String,
    pub redirect_uri: String,
    pub user_id: UserId,
    pub user_email: String,
    pub user_login: String,
    pub scope: String,
    pub secret: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct JetpackConnection {
    pub is_active: bool,
    pub is_user_connected: bool,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct JetpackConnectionData {
    pub current_user: JetpackConnectionUser,
    pub connection_owner: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct JetpackConnectionUser {
    pub id: UserId,
    pub username: String,
    pub is_connected: bool,
    pub is_master: bool,
    pub blog_id: Option<WpComSiteId>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct JetpackConnectionCheck {
    pub status: String,
}

/// Connect a WordPress site to Jetpack.
///
/// Please note, the endpoints used are available on Jetpack 14.2 and above.
#[derive(uniffi::Object)]
pub struct JetpackConnectionClient {
    delegate: WpApiClientDelegate,
    jetpack_client: JetpackApiClient,
}

#[uniffi::export]
impl JetpackConnectionClient {
    #[uniffi::constructor]
    pub fn new(api_root_url: Arc<ParsedUrl>, delegate: WpApiClientDelegate) -> Self {
        Self {
            delegate: delegate.clone(),
            jetpack_client: JetpackApiClient::new(api_root_url, delegate),
        }
    }

    pub async fn status(&self) -> Result<JetpackConnectionStatus, JetpackConnectionClientError> {
        let info = self
            .jetpack_client
            .connection()
            .connection()
            .await
            .map_err(JetpackConnectionClientError::Unhandled)?;
        if !info.data.is_active {
            return Ok(JetpackConnectionStatus::NotConnected);
        }

        let status = self
            .jetpack_client
            .connection()
            .connection_data()
            .await
            .map_err(JetpackConnectionClientError::Unhandled)?;

        if let Some(blog_id) = status.data.current_user.blog_id {
            if let Some(owner_site_username) = status.data.connection_owner {
                return Ok(JetpackConnectionStatus::User {
                    owner_site_username,
                    blog_id,
                });
            }

            return Ok(JetpackConnectionStatus::Site { blog_id });
        }

        Ok(JetpackConnectionStatus::NotConnected)
    }

    pub async fn connect_site(
        &self,
        from: String,
    ) -> Result<WpComSiteId, JetpackConnectionClientError> {
        match self.status().await {
            Ok(JetpackConnectionStatus::NotConnected) => { /* continue */ }
            Ok(JetpackConnectionStatus::Site { blog_id }) => return Ok(blog_id),
            Ok(JetpackConnectionStatus::User { blog_id, .. }) => return Ok(blog_id),
            Err(error) => return Err(error),
        }

        let params = JetpackConnectionParams {
            from,
            plugin_slug: "jetpack".to_string(),
        };
        let result = self
            .jetpack_client
            .connection()
            .register(&params)
            .await
            .map_err(JetpackConnectionClientError::Unhandled)?;
        let blog_id =
            result
                .data
                .blog_id()
                .ok_or_else(|| JetpackConnectionClientError::BlogIdMissing {
                    url: result.data.authorize_url.clone(),
                })?;

        Ok(blog_id)
    }

    pub async fn connect_user(
        &self,
        wp_com_authentication: WpAuthentication,
        from: String,
    ) -> Result<WpComSiteId, JetpackConnectionClientError> {
        let blog_id = self.connect_site(from).await?;

        let provision_info = self
            .jetpack_client
            .connection()
            .remote_provision(&JetpackRemoteProvisionParams {})
            .await
            .map_err(JetpackConnectionClientError::Unhandled)?
            .data;

        let wp_com_client = WpComApiClient::new(wp_api::WpApiClientDelegate {
            auth_provider: WpAuthenticationProvider::static_with_auth(wp_com_authentication).into(),
            request_executor: self.delegate.request_executor.clone(),
            middleware_pipeline: self.delegate.middleware_pipeline.clone(),
        });
        let params = JetpackRemoteConnectionParams {
            secret: provision_info.secret,
            scope: provision_info.scope,
            external_user_id: provision_info.user_id.0.to_string(),
            redirect_uri: provision_info.redirect_uri,
        };
        let result = wp_com_client
            .jetpack_connection()
            .remote_connect_user(&blog_id, &params)
            .await;

        // At the time of writing, `"code": "success"` is parsed as an error case.
        if let Err(WpApiError::WpError {
            error_code: WpErrorCode::CustomError(code),
            ..
        }) = &result
        {
            if code == "success" || code == "already_connected" {
                return Ok(blog_id);
            }
        }

        let result = result
            .map_err(JetpackConnectionClientError::Unhandled)?
            .data;

        if result.code == "success" {
            Ok(blog_id)
        } else {
            Err(JetpackConnectionClientError::UserConnectionFailed {
                message: result.message,
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum JetpackConnectionStatus {
    NotConnected,
    Site {
        blog_id: WpComSiteId,
    },
    User {
        owner_site_username: String,
        blog_id: WpComSiteId,
    },
}

#[derive(Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum JetpackConnectionClientError {
    #[error("Can't find blog id")]
    BlogIdMissing { url: String },
    #[error("Failed to connect to WordPress.com: {message}")]
    UserConnectionFailed { message: String },
    #[error("Unhandled error: {0}")]
    Unhandled(WpApiError),
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case(
        "https://jetpack.wordpress.com/jetpack.authorize/1/?response_type=code&client_id=1234567890&redirect_uri=uri",
        Some(WpComSiteId(1234567890))
    )]
    #[case(
        "https://jetpack.wordpress.com/jetpack.authorize/1/?response_type=code&client_id=abc&redirect_uri=uri",
        None
    )]
    fn test_parsing_blog_id(#[case] url: &str, #[case] expected: Option<WpComSiteId>) {
        let result = JetpackConnectionRegisterResult {
            authorize_url: url.to_string(),
        };
        assert_eq!(result.blog_id(), expected);
    }

    #[test]
    fn test_parsing_register_result() {
        let json = r#"
        {
            "alternateAuthorizeUrl":"",
            "authorizeUrl":"https://jetpack.wordpress.com/jetpack.authorize/1/?response_type=code&client_id=1234567890&redirect_uri=..."
        }
        "#;
        assert!(serde_json::from_str::<JetpackConnectionRegisterResult>(json).is_ok());
    }

    #[test]
    fn test_parsing_provision_result() {
        let json = r#"
        {
            "jp_version":"14.4-a.5",
            "redirect_uri":"https://your-jetpack-site.example.com/wp-login.php?action=jetpack-sso&redirect_to=url",
            "user_id":1,
            "user_email":"example@example.com",
            "user_login":"your-username",
            "scope":"administrator:1234567890abcde",
            "secret":"supersecret",
            "is_active":false
        }
        "#;
        assert!(serde_json::from_str::<JetpackRemoteProvisionResult>(json).is_ok());
    }

    #[test]
    fn test_parsing_connection() {
        let json = r#"
        {
            "isActive": true,
            "isStaging": false,
            "isRegistered": true,
            "isUserConnected": false,
            "hasConnectedOwner": true,
            "offlineMode": {
                "isActive": false,
                "constant": false,
                "url": false,
                "filter": false,
                "wpLocalConstant": false
            },
            "isPublic": false
        }
        "#;
        let connection = serde_json::from_str::<JetpackConnection>(json).unwrap();
        assert!(connection.is_active);
        assert!(!connection.is_user_connected);
    }

    #[test]
    fn test_parsing_connection_data() {
        let json = r#"
        {
            "currentUser": {
                "isConnected": false,
                "isMaster": false,
                "username": "other",
                "id": 2,
                "blogId": 2416127,
                "wpcomUser": {
                    "avatar": false
                },
                "gravatar": "https://secure.gravatar.com/avatar/link?s=96&d=mm&r=g",
                "permissions": {
                    "connect": true,
                    "connect_user": true,
                    "disconnect": true,
                    "admin_page": true,
                    "manage_modules": true,
                    "network_admin": false,
                    "network_sites_page": false,
                    "edit_posts": true,
                    "publish_posts": true,
                    "manage_options": true,
                    "view_stats": true,
                    "manage_plugins": true
                }
            },
            "connectionOwner": "demo"
        }
        "#;
        let connection_data = serde_json::from_str::<JetpackConnectionData>(json).unwrap();
        assert_eq!(connection_data.current_user.id, UserId(2));
        assert_eq!(connection_data.current_user.username, "other");
        assert_eq!(
            connection_data.current_user.blog_id,
            Some(WpComSiteId(2416127))
        );
        assert_eq!(connection_data.connection_owner, Some("demo".to_string()));
    }
}
