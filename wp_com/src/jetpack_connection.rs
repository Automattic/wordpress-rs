use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, uniffi::Record)]
pub struct JetpackRemoteConnectionParams {
    /// The value returned by `remote_provision`` endpoint
    pub secret: String,
    /// Tthe value returned by remote_provision endpoint
    pub scope: String,
    /// User ID from the Jetpack site
    pub external_user_id: String,
    /// The Jetpack site URL
    pub redirect_uri: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct JetpackRemoteConnectionResult {
    pub code: String,
    pub message: String,
}
