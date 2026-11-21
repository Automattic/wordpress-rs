use http::{HeaderMap, HeaderValue};
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

use crate::login::url_discovery::AutoDiscoveryAttemptFailure;
use crate::prelude::WpLoginClient;
use crate::{
    login::{nonce::WpRestNonceRetrieval, url_discovery::AutoDiscoveryAttemptSuccess},
    request::RequestExecutor,
};

// Has custom `Debug` trait implementation, which leaves out the credential
#[derive(Clone, uniffi::Enum)]
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

/// Names the kind of authentication, never the credential itself.
impl Debug for WpAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const REDACTED: &str = "<redacted>";
        match self {
            Self::AuthorizationHeader { .. } => f
                .debug_struct("AuthorizationHeader")
                .field("token", &REDACTED)
                .finish(),
            Self::Bearer { .. } => f.debug_struct("Bearer").field("token", &REDACTED).finish(),
            Self::Nonce { .. } => f.debug_struct("Nonce").field("nonce", &REDACTED).finish(),
            Self::None => f.write_str("None"),
        }
    }
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
            // Each value is marked sensitive, so that `http` and the HTTP clients built
            // on it neither print it nor add it to an HTTP/2 header compression table.
            Self::AuthorizationHeader { token } => {
                let mut value = HeaderValue::from_str(&format!("Basic {token}"))
                .expect("It shouldn't be possible to build WpAuthentication::AuthorizationHeader with an invalid token");
                value.set_sensitive(true);
                headers.insert(http::header::AUTHORIZATION, value);
            }
            Self::Bearer { token } => {
                let mut value = HeaderValue::from_str(&format!("Bearer {token}")).expect("It shouldn't be possible to build WpAuthentication::Bearer with an invalid token");
                value.set_sensitive(true);
                headers.insert(http::header::AUTHORIZATION, value);
            }
            Self::Nonce { nonce } => {
                let mut value = HeaderValue::from_str(nonce).expect("It shouldn't be possible to build WpAuthentication::Nonce with an invalid nonce");
                value.set_sensitive(true);
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

    fn current_auth(&self) -> WpAuthentication {
        self.auth
            .read()
            .expect("If the lock is poisoned, there isn't much we can do")
            .clone()
    }
}

#[derive(uniffi::Object)]
pub struct CookiesNonceAuthenticationProvider {
    site_url: String,
    username: String,
    password: String,
    request_executor: Arc<dyn RequestExecutor>,
    nonce_retrieval: RwLock<Option<Arc<WpRestNonceRetrieval>>>,
    auth: RwLock<WpAuthentication>,
}

#[uniffi::export]
impl CookiesNonceAuthenticationProvider {
    #[uniffi::constructor(name = "new")]
    pub fn new(
        username: String,
        password: String,
        details: AutoDiscoveryAttemptSuccess,
        request_executor: Arc<dyn RequestExecutor>,
    ) -> Self {
        Self {
            site_url: details.api_details.site_url_string(),
            username,
            password,
            request_executor: request_executor.clone(),
            nonce_retrieval: RwLock::new(Some(Arc::new(WpRestNonceRetrieval::new(
                details,
                request_executor.clone(),
            )))),
            auth: RwLock::new(WpAuthentication::None),
        }
    }

    #[uniffi::constructor(name = "with_site_url")]
    pub fn with_site_url(
        url: String,
        username: String,
        password: String,
        request_executor: Arc<dyn RequestExecutor>,
    ) -> Self {
        Self {
            site_url: url,
            username,
            password,
            request_executor,
            nonce_retrieval: RwLock::new(None),
            auth: RwLock::new(WpAuthentication::None),
        }
    }
}

impl CookiesNonceAuthenticationProvider {
    fn get_nonce_retrieval(&self) -> Option<Arc<WpRestNonceRetrieval>> {
        self.nonce_retrieval
            .read()
            .expect("If the lock is poisoned, there isn't much we can do")
            .as_ref()
            .cloned()
    }

    async fn create_nonce_retrieval_if_needed(
        &self,
    ) -> Result<Arc<WpRestNonceRetrieval>, AutoDiscoveryAttemptFailure> {
        if let Some(nonce_retrieval) = self.get_nonce_retrieval() {
            return Ok(nonce_retrieval);
        }

        let client =
            WpLoginClient::new_with_default_middleware_pipeline(self.request_executor.clone());
        let result = client.api_discovery(self.site_url.clone(), None).await;
        let details = match result.combined_result() {
            Ok(details) => details.clone(),
            Err(err) => {
                return Err(err.clone());
            }
        };

        let nonce_retrieval = Arc::new(WpRestNonceRetrieval::new(
            details,
            self.request_executor.clone(),
        ));

        *self
            .nonce_retrieval
            .write()
            .expect("If the lock is poisoned, there isn't much we can do") =
            Some(nonce_retrieval.clone());
        Ok(nonce_retrieval)
    }
}

#[uniffi::export]
#[async_trait::async_trait]
impl WpDynamicAuthenticationProvider for CookiesNonceAuthenticationProvider {
    fn auth(&self) -> WpAuthentication {
        self.auth
            .read()
            .expect("If the lock is poisoned, there isn't much we can do")
            .clone()
    }

    async fn refresh(&self) -> bool {
        let nonce_retrieval = match self.create_nonce_retrieval_if_needed().await {
            Ok(value) => value,
            Err(_) => return false,
        };

        match nonce_retrieval
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

impl Debug for CookiesNonceAuthenticationProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CookiesNonceAuthenticationProvider")
            .field("site_url", &self.site_url)
            .field("username", &self.username)
            .finish()
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
    /// The credential currently in use, so callers can tell which authentication
    /// scheme a request was made with (e.g. an application password versus a
    /// bearer token).
    pub(crate) fn current_auth(&self) -> WpAuthentication {
        match self {
            WpAuthenticationProvider::StaticAuthenticationProvider { auth } => auth.clone(),
            WpAuthenticationProvider::DynamicAuthenticationProvider { inner } => inner.auth(),
            WpAuthenticationProvider::Modifiable { inner } => inner.current_auth(),
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    const NOT_A_REAL_TOKEN: &str = "not-a-real-token";

    #[test]
    fn debug_names_the_kind_of_authentication_but_not_the_credential() {
        for (authentication, kind) in [
            (
                WpAuthentication::Bearer {
                    token: NOT_A_REAL_TOKEN.to_string(),
                },
                "Bearer",
            ),
            (
                WpAuthentication::AuthorizationHeader {
                    token: NOT_A_REAL_TOKEN.to_string(),
                },
                "AuthorizationHeader",
            ),
            (
                WpAuthentication::Nonce {
                    nonce: NOT_A_REAL_TOKEN.to_string(),
                },
                "Nonce",
            ),
        ] {
            let printed = format!("{authentication:?}");
            assert!(!printed.contains(NOT_A_REAL_TOKEN), "{printed}");
            assert!(printed.starts_with(kind), "{printed}");
            assert!(printed.contains("<redacted>"), "{printed}");
        }
    }

    #[test]
    fn inserted_credentials_are_marked_sensitive() {
        let mut headers = HeaderMap::new();
        WpAuthentication::Bearer {
            token: NOT_A_REAL_TOKEN.to_string(),
        }
        .insert_header(&mut headers);
        let value = headers
            .get(http::header::AUTHORIZATION)
            .expect("the header was inserted");

        assert!(value.is_sensitive());
        assert!(!format!("{headers:?}").contains(NOT_A_REAL_TOKEN));
    }
}
