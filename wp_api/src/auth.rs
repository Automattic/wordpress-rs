use http::{HeaderMap, HeaderValue};
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

use crate::{
    login::{nonce::WpRestNonceRetrieval, url_discovery::AutoDiscoveryAttemptSuccess},
    request::RequestExecutor,
};

#[derive(Debug, Clone, uniffi::Enum)]
pub enum WpAuthentication {
    AuthorizationHeader { token: String },
    Bearer { token: String },
    // Cookies+nonce authentication.
    // The "cookies" part is implicitly handled by the HTTP client.
    // Since nonce is refreshed often, when using this authentication method,
    // the caller should not keep using the same nonce for a long time.
    Nonce { nonce: String },
    None,
}

#[uniffi::export]
fn wp_authentication_from_username_and_password(
    username: String,
    password: String,
) -> WpAuthentication {
    WpAuthentication::from_username_and_password(username, password)
}

impl WpAuthentication {
    pub fn from_username_and_password(username: String, password: String) -> Self {
        use base64::prelude::*;
        WpAuthentication::AuthorizationHeader {
            token: BASE64_STANDARD.encode(format!("{username}:{password}")),
        }
    }

    pub fn insert_header(&self, headers: &mut http::HeaderMap) {
        match self {
            Self::None => {}
            Self::AuthorizationHeader { token } => {
                let value = HeaderValue::from_str(&format!("Basic {token}"))
                .expect("It shouldn't be possible to build WpAuthentication::AuthorizationHeader with an invalid token");
                headers.insert(http::header::AUTHORIZATION, value);
            }
            Self::Bearer { token } => {
                let value = HeaderValue::from_str(&format!("Bearer {token}")).expect("It shouldn't be possible to build WpAuthentication::Bearer with an invalid token");
                headers.insert(http::header::AUTHORIZATION, value);
            }
            Self::Nonce { nonce } => {
                let value = HeaderValue::from_str(nonce).expect("It shouldn't be possible to build WpAuthentication::Nonce with an invalid nonce");
                headers.insert("X-WP-Nonce", value);
            }
        }
    }
}

#[derive(uniffi::Object)]
pub struct ModifiableAuthenticationProvider {
    auth: RwLock<WpAuthentication>,
}

#[uniffi::export]
impl ModifiableAuthenticationProvider {
    #[uniffi::constructor]
    pub fn new(authentication: WpAuthentication) -> Self {
        Self {
            auth: RwLock::new(authentication),
        }
    }

    pub fn set_authentication(&self, new_authentication: WpAuthentication) {
        *self
            .auth
            .write()
            .expect("If the lock is poisoned, there isn't much we can do") = new_authentication;
    }
}

impl ModifiableAuthenticationProvider {
    pub fn insert_header(&self, headers: &mut http::HeaderMap) {
        self.auth
            .read()
            .expect("If the lock is poisoned, there isn't much we can do")
            .insert_header(headers)
    }
}

#[derive(Debug, uniffi::Object)]
pub struct CookiesNonceAuthenticationProvider {
    username: String,
    password: String,
    nonce_retrieval: Arc<WpRestNonceRetrieval>,
    auth: RwLock<WpAuthentication>,
}

#[uniffi::export]
impl CookiesNonceAuthenticationProvider {
    #[uniffi::constructor]
    pub fn new(
        username: String,
        password: String,
        details: AutoDiscoveryAttemptSuccess,
        request_executor: Arc<dyn RequestExecutor>,
    ) -> Self {
        Self {
            username,
            password,
            nonce_retrieval: Arc::new(WpRestNonceRetrieval::new(details, request_executor)),
            auth: RwLock::new(WpAuthentication::None),
        }
    }
}

#[async_trait::async_trait]
impl WpDynamicAuthenticationProvider for CookiesNonceAuthenticationProvider {
    fn auth(&self) -> WpAuthentication {
        self.auth
            .read()
            .expect("If the lock is poisoned, there isn't much we can do")
            .clone()
    }

    async fn refresh(&self) -> bool {
        match self
            .nonce_retrieval
            .get_nonce(self.username.clone(), self.password.clone())
            .await
        {
            Ok(nonce) => {
                *self
                    .auth
                    .write()
                    .expect("If the lock is poisoned, there isn't much we can do") =
                    WpAuthentication::Nonce { nonce };
                true
            }
            Err(_) => false,
        }
    }
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait WpDynamicAuthenticationProvider: Send + Sync + Debug {
    fn auth(&self) -> WpAuthentication;

    /// Refresh the authentication token. The implementation should only return true
    /// if the authentication was successfully refreshed.
    ///
    /// **Concurrency:** This method may be called concurrently by multiple request
    /// executors. Implementations must handle concurrent calls safely and avoid
    /// unnecessary duplicate refresh operations.
    async fn refresh(&self) -> bool;
}

#[derive(uniffi::Object)]
pub enum WpAuthenticationProvider {
    StaticAuthenticationProvider {
        auth: WpAuthentication,
    },
    DynamicAuthenticationProvider {
        inner: Arc<dyn WpDynamicAuthenticationProvider>,
    },
    Modifiable {
        inner: Arc<ModifiableAuthenticationProvider>,
    },
}

#[uniffi::export]
impl WpAuthenticationProvider {
    #[uniffi::constructor]
    pub fn static_with_username_and_password(username: String, password: String) -> Self {
        Self::StaticAuthenticationProvider {
            auth: WpAuthentication::from_username_and_password(username, password),
        }
    }

    #[uniffi::constructor]
    pub fn static_with_auth(auth: WpAuthentication) -> Self {
        Self::StaticAuthenticationProvider { auth }
    }

    #[uniffi::constructor]
    pub fn dynamic(
        dynamic_authentication_provider: Arc<dyn WpDynamicAuthenticationProvider>,
    ) -> Self {
        Self::DynamicAuthenticationProvider {
            inner: dynamic_authentication_provider,
        }
    }

    #[uniffi::constructor]
    pub fn none() -> Self {
        Self::StaticAuthenticationProvider {
            auth: WpAuthentication::None,
        }
    }

    #[uniffi::constructor]
    pub fn modifiable(modifiable_auth: Arc<ModifiableAuthenticationProvider>) -> Self {
        Self::Modifiable {
            inner: modifiable_auth,
        }
    }
}

impl WpAuthenticationProvider {
    pub fn insert_header(&self, headers: &mut HeaderMap) {
        match self {
            WpAuthenticationProvider::StaticAuthenticationProvider { auth } => {
                auth.insert_header(headers)
            }
            WpAuthenticationProvider::DynamicAuthenticationProvider { inner } => {
                inner.auth().insert_header(headers)
            }
            WpAuthenticationProvider::Modifiable { inner } => inner.insert_header(headers),
        }
    }

    pub(crate) async fn refresh(&self) -> bool {
        match self {
            WpAuthenticationProvider::StaticAuthenticationProvider { .. } => false,
            WpAuthenticationProvider::DynamicAuthenticationProvider { inner } => {
                inner.refresh().await
            }
            WpAuthenticationProvider::Modifiable { .. } => false,
        }
    }
}
