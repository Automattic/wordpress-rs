use serde::{Deserialize, Serialize};
use std::fmt::Display;

uniffi::custom_newtype!(WidgetTypeId, String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WidgetTypeId(pub String);

impl Display for WidgetTypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
