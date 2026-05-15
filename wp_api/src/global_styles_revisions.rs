use crate::{
    JsonValue, UserId,
    date::WpGmtDateTime,
    global_styles::GlobalStylesId,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_i64_id,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wp_contextual::WpContextual;
use wp_derive::WpDeriveParamsField;

wp_content_i64_id!(GlobalStylesRevisionId);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct GlobalStylesRevisionListParams {
    /// Current page of the collection.
    /// Default: `1`
    #[uniffi(default = None)]
    pub page: Option<u32>,
    /// Maximum number of items to be returned in result set.
    #[uniffi(default = None)]
    pub per_page: Option<u32>,
    /// Offset the result set by a specific number of items.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseGlobalStylesRevision {
    #[WpContext(edit, embed, view)]
    pub id: Option<GlobalStylesRevisionId>,
    #[WpContext(edit, embed, view)]
    pub author: Option<UserId>,
    #[WpContext(edit, embed, view)]
    pub date: Option<String>,
    #[WpContext(edit, view)]
    pub date_gmt: Option<WpGmtDateTime>,
    #[WpContext(edit, view)]
    pub modified: Option<String>,
    #[WpContext(edit, view)]
    pub modified_gmt: Option<WpGmtDateTime>,
    #[WpContext(edit, embed, view)]
    pub parent: Option<GlobalStylesId>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub settings: Option<HashMap<String, JsonValue>>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub styles: Option<HashMap<String, JsonValue>>,
}
