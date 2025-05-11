use crate::{
    UserId, impl_as_query_value_from_to_string,
    post_types::PostType,
    posts::PostId,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use strum_macros::IntoStaticStr;
use wp_contextual::WpContextual;

uniffi::custom_newtype!(TemplateId, String);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateId(pub String);

impl Display for TemplateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

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
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TemplateStatus {
    Draft,
    Future,
    Pending,
    Private,
    #[default]
    Publish,
    Trash,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

impl_as_query_value_from_to_string!(TemplateStatus);

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TemplateArea {
    Header,
    Footer,
    Uncategorized,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

impl_as_query_value_from_to_string!(TemplateArea);

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
    pub id: Option<TemplateId>,
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
    #[WpContext(edit, embed, view)]
    pub content: Option<SparseTemplateContentWrapper>,
    #[WpContext(edit, embed, view)]
    pub title: Option<SparseTemplateTitleWrapper>,
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
    #[WpContextualOption]
    #[serde(
        default,
        deserialize_with = "wp_serde_helper::deserialize_false_or_string"
    )]
    pub modified: Option<String>,
    #[WpContext(edit, view, embed)]
    pub is_custom: Option<bool>,
    #[WpContext(edit, view, embed)]
    pub author_text: Option<String>,
    #[WpContext(edit, view, embed)]
    pub original_source: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, PartialOrd, Eq, Deserialize, uniffi::Enum)]
#[serde(untagged)]
pub enum SparseTemplateContentWrapper {
    Object(SparseTemplateContent),
    String(String),
}

#[derive(Debug, Serialize, PartialEq, PartialOrd, Eq, wp_derive::WpDeserialize, uniffi::Record)]
pub struct SparseTemplateContent {
    pub raw: Option<String>,
    pub rendered: Option<String>,
    pub protected: Option<bool>,
    pub block_version: Option<u32>,
}

#[derive(Debug, Serialize, PartialEq, PartialOrd, Eq, Deserialize, uniffi::Enum)]
#[serde(untagged)]
pub enum SparseTemplateTitleWrapper {
    Object(SparseTemplateTitle),
    String(String),
}

#[derive(Debug, Serialize, PartialEq, PartialOrd, Eq, wp_derive::WpDeserialize, uniffi::Record)]
pub struct SparseTemplateTitle {
    pub raw: Option<String>,
    pub rendered: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct TemplateDeleteResponse {
    pub deleted: bool,
    pub previous: TemplateWithEditContext,
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct TemplateUpdateParams {
    // Content of template.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    // Title of template.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    // Description of template.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    // The ID for the author of the template.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<UserId>,
    // https://developer.wordpress.org/rest-api/reference/wp_templates/#update-a-template
    // The documentation includes `slug`, `status`, `theme` & `type` parameters, but the updates
    // don't seem to take place when they are included in the request. So, we decided not to
    // include them until we figure out under which conditions they'll be allowed to update.
}

#[derive(Debug, Serialize, uniffi::Record)]
pub struct TemplateCreateParams {
    // Unique slug identifying the template.
    pub slug: String,
    // Theme identifier for the template.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    // Type of template.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_type: Option<String>,
    // Content of template.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    // Title of template.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    // Description of template.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    // Status of template.
    // One of: publish, future, draft, pending, private
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TemplateStatus>,
    // The ID for the author of the template.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<UserId>,
}

impl TemplateCreateParams {
    pub fn new(slug: String) -> Self {
        Self {
            slug,
            theme: None,
            template_type: None,
            content: None,
            title: None,
            description: None,
            status: None,
            author: None,
        }
    }
}
