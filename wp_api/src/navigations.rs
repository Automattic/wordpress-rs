use crate::{
    WpApiParamOrder, impl_as_query_value_from_to_string,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_i64_id,
};
use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;
use wp_derive::WpDeriveParamsField;

wp_content_i64_id!(NavigationId);

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
pub enum NavigationStatus {
    Draft,
    Future,
    Pending,
    Private,
    #[default]
    Publish,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

impl_as_query_value_from_to_string!(NavigationStatus);

#[derive(Debug, Serialize, Deserialize, WpContextual)]
pub struct SparseNavigation {
    #[WpContext(edit, embed, view)]
    pub date: Option<String>,
    #[WpContext(edit, view)]
    pub date_gmt: Option<String>,
    #[WpContext(edit, view)]
    pub guid: Option<SparseNavigationGuid>,
    #[WpContext(edit, embed, view)]
    pub id: Option<NavigationId>,
    #[WpContext(edit, embed, view)]
    pub link: Option<String>,
    #[WpContext(edit, view)]
    pub modified: Option<String>,
    #[WpContext(edit, view)]
    pub modified_gmt: Option<String>,
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WpContext(edit, embed, view)]
    pub status: Option<NavigationStatus>,
    #[WpContext(edit, embed, view)]
    #[serde(rename = "type")]
    pub navigation_type: Option<String>,
    #[WpContext(edit)]
    #[WpContextualOption]
    pub password: Option<String>,
    #[WpContext(edit, embed, view)]
    pub title: Option<SparseNavigationTitleWrapper>,
    #[WpContext(edit, embed, view)]
    pub content: Option<SparseNavigationContentWrapper>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub template: Option<String>,
    // _meta field omitted
}

#[derive(Debug, Serialize, PartialEq, PartialOrd, Eq, wp_derive::WpDeserialize, uniffi::Record)]
pub struct SparseNavigationGuid {
    pub rendered: Option<String>,
    pub raw: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, PartialOrd, Eq, Deserialize, uniffi::Enum)]
#[serde(untagged)]
pub enum SparseNavigationTitleWrapper {
    Object(SparseNavigationTitle),
    String(String),
}

#[derive(Debug, Serialize, PartialEq, PartialOrd, Eq, wp_derive::WpDeserialize, uniffi::Record)]
pub struct SparseNavigationTitle {
    pub raw: Option<String>,
    pub rendered: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, PartialOrd, Eq, Deserialize, uniffi::Enum)]
#[serde(untagged)]
pub enum SparseNavigationContentWrapper {
    Object(SparseNavigationContent),
    String(String),
}

#[derive(Debug, Serialize, PartialEq, PartialOrd, Eq, wp_derive::WpDeserialize, uniffi::Record)]
pub struct SparseNavigationContent {
    pub raw: Option<String>,
    pub rendered: Option<String>,
    pub protected: Option<bool>,
    pub block_version: Option<u32>,
}

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct NavigationListParams {
    /// Current page of the collection.
    #[uniffi(default = None)]
    pub page: Option<u32>,
    /// Maximum number of items to be returned in result set.
    #[uniffi(default = None)]
    pub per_page: Option<u32>,
    /// Limit results to those matching a string.
    #[uniffi(default = None)]
    pub search: Option<String>,
    /// Limit response to posts published after a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub after: Option<String>,
    /// Limit response to posts modified after a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub modified_after: Option<String>,
    /// Limit response to posts published before a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub before: Option<String>,
    /// Limit response to posts modified before a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub modified_before: Option<String>,
    /// Ensure result set excludes specific IDs.
    #[uniffi(default = [])]
    pub exclude: Vec<NavigationId>,
    /// Limit result set to specific IDs.
    #[uniffi(default = [])]
    pub include: Vec<NavigationId>,
    /// Offset the result set by a specific number of items.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    /// Order sort attribute ascending or descending.
    #[uniffi(default = None)]
    pub order: Option<WpApiParamOrder>,
    /// Sort collection by post attribute.
    #[uniffi(default = None)]
    #[field_name("orderby")]
    pub order_by: Option<NavigationOrderBy>,
    /// Array of column names to be searched.
    #[uniffi(default = [])]
    pub search_columns: Vec<String>,
    /// Limit result set to posts with one or more specific slugs.
    #[uniffi(default = [])]
    pub slug: Vec<String>,
    /// Limit result set to posts assigned one or more statuses.
    #[uniffi(default = [])]
    pub status: Vec<NavigationStatus>,
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
pub enum NavigationOrderBy {
    Author,
    #[default]
    Date,
    Id,
    Include,
    Modified,
    Parent,
    Relevance,
    Slug,
    IncludeSlugs,
    Title,
}

impl_as_query_value_from_to_string!(NavigationOrderBy);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(false)]
pub struct NavigationRetrieveParams {
    /// The password for the post if it is password protected.
    #[uniffi(default = None)]
    pub password: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct NavigationDeleteResponse {
    pub deleted: bool,
    pub previous: NavigationWithEditContext,
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct NavigationUpdateParams {
    /// The date the post was published, in the site's timezone.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// The date the post was published, as GMT.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_gmt: Option<String>,
    /// An alphanumeric identifier for the post unique to its type.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// A named status for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<NavigationStatus>,
    /// A password to protect access to the content and excerpt.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// The title for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The content for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// The theme file to use to display the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct NavigationCreateParams {
    /// The date the post was published, in the site's timezone.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// The date the post was published, as GMT.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_gmt: Option<String>,
    /// An alphanumeric identifier for the post unique to its type.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// A named status for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<NavigationStatus>,
    /// A password to protect access to the content and excerpt.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// The title for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The content for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// The theme file to use to display the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SparseField;
    use rstest::*;

    #[rstest]
    #[case(SparseNavigationFieldWithEditContext::Id, "id")]
    #[case(SparseNavigationFieldWithEditContext::NavigationType, "type")]
    fn test_as_mapped_field_name_for_edit_context(
        #[case] field: SparseNavigationFieldWithEditContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }

    #[rstest]
    #[case(SparseNavigationFieldWithEmbedContext::Id, "id")]
    #[case(SparseNavigationFieldWithEmbedContext::NavigationType, "type")]
    fn test_as_mapped_field_name_for_embed_context(
        #[case] field: SparseNavigationFieldWithEmbedContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }

    #[rstest]
    #[case(SparseNavigationFieldWithViewContext::Id, "id")]
    #[case(SparseNavigationFieldWithViewContext::NavigationType, "type")]
    fn test_as_mapped_field_name_for_view_context(
        #[case] field: SparseNavigationFieldWithViewContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }
}
