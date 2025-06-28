use crate::impl_as_query_value_from_to_string;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use wp_contextual::WpContextual;

uniffi::custom_newtype!(WidgetTypeId, String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WidgetTypeId(pub String);

impl Display for WidgetTypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl_as_query_value_from_to_string!(WidgetTypeId);

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseWidgetType {
    #[WpContext(edit, embed, view)]
    pub id: Option<WidgetTypeId>,
    #[WpContext(edit, embed, view)]
    pub name: Option<String>,
    #[WpContext(edit, embed, view)]
    pub description: Option<String>,
    #[WpContext(edit, embed, view)]
    pub is_multi: Option<bool>,
    #[WpContext(edit, embed, view)]
    pub classname: Option<String>,
}
