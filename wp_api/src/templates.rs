use crate::{
    UserId, impl_as_query_value_from_to_string,
    post_types::PostType,
    posts::PostId,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_string_id,
};
use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;
use wp_derive::WpDeriveParamsField;

wp_content_string_id!(TemplateId);

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

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)]
pub struct TemplateListParams {
    /// Limit to the specified post id.
    #[uniffi(default = None)]
    #[field_name("wp_id")]
    pub post_id: Option<PostId>,
    /// Limit to the specified template part area.
    #[uniffi(default = None)]
    pub area: Option<TemplateArea>,
    /// Post type to get the templates for.
    #[uniffi(default = None)]
    pub post_type: Option<PostType>,
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
    // https://developer.wordpress.org/rest-api/reference/wp_templates/#create-a-template
    // The documentation includes `status` & `type` parameters, but the created templates don't
    // seem to take these into account. So, we decided not to include them until we figure out
    // under which conditions they'll be taken into account.
}

impl TemplateCreateParams {
    pub fn new(slug: String) -> Self {
        Self {
            slug,
            theme: None,
            content: None,
            title: None,
            description: None,
            author: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SparseField;
    use rstest::*;

    #[rstest]
    #[case(SparseTemplateFieldWithEditContext::Id, "id")]
    #[case(SparseTemplateFieldWithEditContext::TemplateType, "type")]
    #[case(SparseTemplateFieldWithEditContext::PostId, "wp_id")]
    fn test_as_mapped_field_name_for_edit_context(
        #[case] field: SparseTemplateFieldWithEditContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }

    #[rstest]
    #[case(SparseTemplateFieldWithEmbedContext::Id, "id")]
    #[case(SparseTemplateFieldWithEmbedContext::TemplateType, "type")]
    #[case(SparseTemplateFieldWithEmbedContext::PostId, "wp_id")]
    fn test_as_mapped_field_name_for_embed_context(
        #[case] field: SparseTemplateFieldWithEmbedContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }

    #[rstest]
    #[case(SparseTemplateFieldWithViewContext::Id, "id")]
    #[case(SparseTemplateFieldWithViewContext::TemplateType, "type")]
    #[case(SparseTemplateFieldWithViewContext::PostId, "wp_id")]
    fn test_as_mapped_field_name_for_view_context(
        #[case] field: SparseTemplateFieldWithViewContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }
}
