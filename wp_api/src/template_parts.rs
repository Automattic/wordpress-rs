use crate::{
    UserId,
    post_types::PostType,
    posts::PostId,
    templates::{
        SparseTemplateContentWrapper, SparseTemplateTitleWrapper, TemplateArea, TemplateStatus,
    },
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_string_id,
};
use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;
use wp_derive::WpDeriveParamsField;

wp_content_string_id!(TemplatePartId);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)]
pub struct TemplatePartListParams {
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
pub struct SparseTemplatePart {
    #[WpContext(edit, embed, view)]
    pub id: Option<TemplatePartId>,
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
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
    #[WpContext(edit, embed, view)]
    pub area: Option<String>,
}

#[derive(Debug, Serialize, uniffi::Record)]
pub struct TemplatePartCreateParams {
    // Unique slug identifying the template part.
    pub slug: String,
    // Theme identifier for the template part.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    // Content of template part.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    // Title of template part.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    // Description of template part.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    // The ID for the author of the template part.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<UserId>,
    // Where the template part is intended for use (header, footer, etc.)
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area: Option<String>,
}

impl TemplatePartCreateParams {
    pub fn new(slug: String) -> Self {
        Self {
            slug,
            theme: None,
            content: None,
            title: None,
            description: None,
            author: None,
            area: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct TemplatePartDeleteResponse {
    pub deleted: bool,
    pub previous: TemplatePartWithEditContext,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SparseField;
    use rstest::*;

    #[rstest]
    #[case(SparseTemplatePartFieldWithEditContext::Id, "id")]
    #[case(SparseTemplatePartFieldWithEditContext::TemplateType, "type")]
    #[case(SparseTemplatePartFieldWithEditContext::PostId, "wp_id")]
    fn test_as_mapped_field_name_for_edit_context(
        #[case] field: SparseTemplatePartFieldWithEditContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }

    #[rstest]
    #[case(SparseTemplatePartFieldWithEmbedContext::Id, "id")]
    #[case(SparseTemplatePartFieldWithEmbedContext::TemplateType, "type")]
    #[case(SparseTemplatePartFieldWithEmbedContext::PostId, "wp_id")]
    fn test_as_mapped_field_name_for_embed_context(
        #[case] field: SparseTemplatePartFieldWithEmbedContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }

    #[rstest]
    #[case(SparseTemplatePartFieldWithViewContext::Id, "id")]
    #[case(SparseTemplatePartFieldWithViewContext::TemplateType, "type")]
    #[case(SparseTemplatePartFieldWithViewContext::PostId, "wp_id")]
    fn test_as_mapped_field_name_for_view_context(
        #[case] field: SparseTemplatePartFieldWithViewContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }
}
