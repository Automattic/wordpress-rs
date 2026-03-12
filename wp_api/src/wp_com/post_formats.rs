use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WpComPostFormatsResponse {
    pub formats: HashMap<String, String>,
}
