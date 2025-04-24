use http::HeaderValue;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, uniffi::Enum)]
pub enum WpAuthentication {
    AuthorizationHeader { token: String },
    Bearer { token: String },
    None,
}

impl WpAuthentication {
    pub fn from_username_and_password(username: String, password: String) -> Self {
        use base64::prelude::*;
        WpAuthentication::AuthorizationHeader {
            token: BASE64_STANDARD.encode(format!("{}:{}", username, password)),
        }
    }

    pub fn header_value(&self) -> Option<HeaderValue> {
        match self {
            Self::None => None,
            Self::AuthorizationHeader { token } => {
                Some(HeaderValue::from_str(&format!("Basic {}", token))
                .expect("It shouldn't be possible to build WpAuthentication::AuthorizationHeader with an invalid token"))
            }
            Self::Bearer { token } => {
                Some(HeaderValue::from_str(&format!("Bearer {}", token)).expect("It shouldn't be possible to build WpAuthentication::Bearer with an invalid token"))
            }
        }
    }
}

#[derive(uniffi::Object)]
pub struct ModifiableAuthenticationProvider {
    auth: Mutex<WpAuthentication>,
}

#[uniffi::export]
impl ModifiableAuthenticationProvider {
    #[uniffi::constructor]
    pub fn new(authentication: WpAuthentication) -> Self {
        Self {
            auth: Mutex::new(authentication),
        }
    }

    pub fn set_authentication(&self, new_authentication: WpAuthentication) {
        *self.auth.lock().unwrap() = new_authentication;
    }
}

impl ModifiableAuthenticationProvider {
    pub fn header_value(&self) -> Option<HeaderValue> {
        self.auth.lock().unwrap().header_value()
    }
}

#[uniffi::export(with_foreign)]
pub trait WpDynamicAuthenticationProvider: Send + Sync {
    fn auth(&self) -> WpAuthentication;
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
}
