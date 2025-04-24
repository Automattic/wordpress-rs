use http::HeaderValue;

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

#[uniffi::export]
fn wp_authentication_from_username_and_password(
    username: String,
    password: String,
) -> WpAuthentication {
    WpAuthentication::from_username_and_password(username, password)
}
