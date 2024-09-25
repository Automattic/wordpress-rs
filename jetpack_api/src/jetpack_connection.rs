use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct JetpackConnectionStatus {
    #[serde(rename = "isActive")]
    pub is_active: bool,
}
