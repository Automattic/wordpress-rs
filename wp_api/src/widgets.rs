use crate::JsonValue;
use crate::url_query::{
    AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
};
use crate::widget_types::WidgetTypeId;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display};
use strum_macros::IntoStaticStr;
use wp_contextual::WpContextual;

uniffi::custom_newtype!(WidgetId, String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WidgetId(pub String);

impl Display for WidgetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseWidget {
    #[WpContext(edit, embed, view)]
    pub id: Option<WidgetId>,
    #[WpContext(edit, embed, view)]
    pub id_base: Option<WidgetTypeId>,
    #[WpContext(edit, embed, view)]
    pub sidebar: Option<String>,
    #[WpContext(edit, embed, view)]
    pub rendered: Option<String>,
    #[WpContext(edit)]
    pub rendered_form: Option<String>,
    #[WpContext(edit)]
    pub instance: Option<WidgetInstance>,
    #[WpContext(edit)]
    #[WpContextualOption]
    pub form_data: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WidgetInstance {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoded: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<HashMap<String, JsonValue>>,
}

#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct WidgetListParams {
    pub sidebar: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, IntoStaticStr)]
enum WidgetListParamsField {
    #[strum(serialize = "sidebar")]
    Sidebar,
}

impl AppendUrlQueryPairs for WidgetListParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair(WidgetListParamsField::Sidebar, self.sidebar.as_ref());
    }
}
impl FromUrlQueryPairs for WidgetListParams {
    fn from_url_query_pairs(query_pairs: UrlQueryPairsMap) -> Option<Self> {
        Some(Self {
            sidebar: query_pairs.get(WidgetListParamsField::Sidebar),
        })
    }

    fn supports_pagination() -> bool {
        false
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WidgetCreateParams {
    pub id_base: WidgetTypeId,
    pub sidebar: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<WidgetInstanceParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form_data: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Enum)]
#[serde(untagged)]
pub enum WidgetInstanceParams {
    Raw { raw: HashMap<String, JsonValue> },
    Encoded { encoded: String, hash: String },
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct WidgetUpdateParams {
    // Updating widget type's with `id_base` is not supported by the backend even though the field
    // is listed in the documentation: https://developer.wordpress.org/rest-api/reference/widgets/#update-a-widget
    // The field is omitted from the type to avoid confusion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidebar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<WidgetInstanceParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form_data: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WidgetDeleteResponse {
    pub deleted: bool,
    pub previous: WidgetWithEditContext,
}
