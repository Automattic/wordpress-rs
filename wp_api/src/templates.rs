use crate::{
    impl_as_query_value_from_as_str,
    post_types::PostType,
    posts::PostId,
    url_query::{
        AppendUrlQueryPairs, AsQueryValue, FromUrlQueryPairs, QueryPairs, QueryPairsExtension,
        UrlQueryPairsMap,
    },
    EnumFromStrParsingError, UserId,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum_macros::IntoStaticStr;
use wp_contextual::WpContextual;

#[derive(
    Debug,
    Default,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum TemplateStatus {
    Draft,
    Future,
    Pending,
    Private,
    #[default]
    Publish,
    #[serde(untagged)]
    Custom(String),
}

impl_as_query_value_from_as_str!(TemplateStatus);

impl TemplateStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Draft => "draft",
            Self::Future => "future",
            Self::Pending => "pending",
            Self::Private => "private",
            Self::Publish => "publish",
            Self::Custom(status) => status,
        }
    }
}

impl FromStr for TemplateStatus {
    type Err = EnumFromStrParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "future" => Ok(Self::Future),
            "pending" => Ok(Self::Pending),
            "private" => Ok(Self::Private),
            "publish" => Ok(Self::Publish),
            value => Ok(Self::Custom(value.to_string())),
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum TemplateArea {
    Header,
    Footer,
    Uncategorized,
    #[serde(untagged)]
    Custom(String),
}

impl_as_query_value_from_as_str!(TemplateArea);

impl TemplateArea {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Header => "header",
            Self::Footer => "footer",
            Self::Uncategorized => "uncategorized",
            Self::Custom(area) => area,
        }
    }
}

impl FromStr for TemplateArea {
    type Err = EnumFromStrParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "header" => Ok(Self::Header),
            "footer" => Ok(Self::Footer),
            "uncategorized" => Ok(Self::Uncategorized),
            value => Ok(Self::Custom(value.to_string())),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
pub struct TemplateListParams {
    /// Limit to the specified post id.
    #[uniffi(default = None)]
    pub post_id: Option<PostId>,
    /// Limit to the specified template part area.
    #[uniffi(default = None)]
    pub area: Option<TemplateArea>,
    /// Post type to get the templates for.
    #[uniffi(default = None)]
    pub post_type: Option<PostType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, IntoStaticStr)]
enum TemplateListParamsField {
    #[strum(serialize = "wp_id")]
    PostId,
    #[strum(serialize = "area")]
    Area,
    #[strum(serialize = "post_type")]
    PostType,
}

impl AppendUrlQueryPairs for TemplateListParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair(TemplateListParamsField::PostId, self.post_id.as_ref())
            .append_option_query_value_pair(TemplateListParamsField::Area, self.area.as_ref())
            .append_option_query_value_pair(
                TemplateListParamsField::PostType,
                self.post_type.as_ref(),
            );
    }
}

impl FromUrlQueryPairs for TemplateListParams {
    fn from_url_query_pairs(query_pairs: UrlQueryPairsMap) -> Option<Self> {
        Some(Self {
            post_id: query_pairs.get(TemplateListParamsField::PostId),
            area: query_pairs.get(TemplateListParamsField::Area),
            post_type: query_pairs.get(TemplateListParamsField::PostType),
        })
    }

    fn supports_pagination() -> bool {
        false
    }
}

#[derive(Debug, Serialize, Deserialize, WpContextual)]
pub struct SparseTemplate {
    #[WpContext(edit, embed, view)]
    pub id: Option<String>,
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WpContext(edit, embed, view)]
    pub theme: Option<String>,
    #[serde(rename = "type")]
    #[WpContext(edit, embed, view)]
    pub template_type: Option<String>,
    #[WpContext(edit, embed, view)]
    pub source: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub origin: Option<String>,
    // TODO: Object or String or it's `[]`
    #[WpContext(edit, embed, view)]
    pub content: Option<SparseTemplateContentWrapper>,
    //// TODO: Object or String
    //#[WpContext(edit, embed, view)]
    //pub title: Option<String>,
    #[WpContext(edit, embed, view)]
    pub description: Option<String>,
    #[WpContext(edit, embed, view)]
    pub status: Option<TemplateStatus>,
    #[serde(rename = "wp_id")]
    #[WpContext(edit, embed, view)]
    pub post_id: Option<PostId>,
    #[WpContext(edit, embed, view)]
    pub has_theme_file: Option<bool>,
    #[WpContext(edit, embed, view)]
    pub author: Option<UserId>,
    #[WpContext(edit, view)]
    pub modified: Option<bool>,
    #[WpContext(edit, view, embed)]
    pub is_custom: Option<bool>,
    // TODO: `author_text` & `original_source` fields are missing from documentation
    #[WpContext(edit, view, embed)]
    pub author_text: Option<String>,
    #[WpContext(edit, view, embed)]
    pub original_source: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Enum)]
#[serde(untagged)]
pub enum SparseTemplateContentWrapper {
    Object(SparseTemplateContent),
    String(String),
}

#[derive(Debug, Serialize, wp_derive::WpDeserialize, uniffi::Record)]
pub struct SparseTemplateContent {
    pub raw: Option<String>,
    pub rendered: Option<String>,
    pub protected: Option<bool>,
    pub block_version: Option<u32>,
}
