use http::HeaderValue;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, uniffi::Enum)]
pub enum WpAuthentication {
    AuthorizationHeader { token: String },
    Bearer { token: String },
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

    pub fn header_value(&self) -> Option<HeaderValue> {
        match self {
            Self::None => None,
            Self::AuthorizationHeader { token } => {
                Some(HeaderValue::from_str(&format!("Basic {token}"))
                .expect("It shouldn't be possible to build WpAuthentication::AuthorizationHeader with an invalid token"))
            }
            Self::Bearer { token } => {
                Some(HeaderValue::from_str(&format!("Bearer {token}")).expect("It shouldn't be possible to build WpAuthentication::Bearer with an invalid token"))
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
    pub fn header_value(&self) -> Option<HeaderValue> {
        self.auth
            .read()
            .expect("If the lock is poisoned, there isn't much we can do")
            .header_value()
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
    pub fn auth_header_value(&self) -> Option<HeaderValue> {
        match self {
            WpAuthenticationProvider::StaticAuthenticationProvider { auth } => auth.header_value(),
            WpAuthenticationProvider::DynamicAuthenticationProvider { inner } => {
                inner.auth().header_value()
            }
            WpAuthenticationProvider::Modifiable { inner } => inner.header_value(),
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
