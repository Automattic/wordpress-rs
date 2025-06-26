use crate::{
    WpApiParamOrder, impl_as_query_value_from_to_string,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
};
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
    pub id_base: Option<String>,
    #[WpContext(edit, embed, view)]
    pub sidebar: Option<String>,
    #[WpContext(edit, embed, view)]
    pub rendered: Option<String>,
    #[WpContext(edit)]
    pub rendered_form: Option<String>,
    #[WpContext(edit)]
    pub instance: Option<WidgetInstance>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WidgetInstance {
    pub encoded: String,
    pub hash: String,
    pub raw: HashMap<String, String>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum, strum_macros::EnumString, strum_macros::Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum WpApiParamWidgetsOrderBy {
    Id,
    Include,
}

impl_as_query_value_from_to_string!(WpApiParamWidgetsOrderBy);

#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct WidgetListParams {
    pub order: Option<WpApiParamOrder>,
    pub orderby: Option<WpApiParamWidgetsOrderBy>,
    pub sidebar: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, IntoStaticStr)]
enum WidgetListParamsField {
    #[strum(serialize = "order")]
    Order,
    #[strum(serialize = "orderby")]
    Orderby,
    #[strum(serialize = "sidebar")]
    Sidebar,
}

impl AppendUrlQueryPairs for WidgetListParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair(WidgetListParamsField::Order, self.order.as_ref())
            .append_option_query_value_pair(WidgetListParamsField::Orderby, self.orderby.as_ref())
            .append_option_query_value_pair(WidgetListParamsField::Sidebar, self.sidebar.as_ref());
    }
}
impl FromUrlQueryPairs for WidgetListParams {
    fn from_url_query_pairs(query_pairs: UrlQueryPairsMap) -> Option<Self> {
        Some(Self {
            order: query_pairs.get(WidgetListParamsField::Order),
            orderby: query_pairs.get(WidgetListParamsField::Orderby),
            sidebar: query_pairs.get(WidgetListParamsField::Sidebar),
        })
    }

    // TODO: Check if true
    fn supports_pagination() -> bool {
        true
    }
}

#[derive(Debug, Clone, uniffi::Record, Serialize, Deserialize)]
pub struct WidgetCreateParams {
    pub id_base: String,
    pub sidebar: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<HashMap<String, String>>,
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct WidgetUpdateParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidebar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct WidgetDeleteResponse {
    pub deleted: bool,
    pub previous: WidgetWithEditContext,
}
