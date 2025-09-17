use crate::{
    UserId, WpApiParamOrder,
    date::WpGmtDateTime,
    impl_as_query_value_from_to_string,
    media::MediaId,
    posts::WpApiParamPostsSearchColumn,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_i64_id,
};
use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;
use wp_derive::WpDeriveParamsField;
use wp_serde_helper::{deserialize_from_string_of_json_array, serialize_as_json_string};

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum WpApiParamPagesOrderBy {
    Author,
    #[default]
    Date,
    Id,
    Include,
    IncludeSlugs,
    MenuOrder,
    Modified,
    Parent,
    Relevance,
    Slug,
    Title,
}

impl_as_query_value_from_to_string!(WpApiParamPagesOrderBy);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct PageListParams {
    /// Current page of the collection.
    /// Default: `1`
    #[uniffi(default = None)]
    pub page: Option<u32>,
    /// Maximum number of items to be returned in result set.
    /// Default: `10`
    #[uniffi(default = None)]
    pub per_page: Option<u32>,
    /// Limit results to those matching a string.
    #[uniffi(default = None)]
    pub search: Option<String>,
    /// Limit response to pages published after a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub after: Option<WpGmtDateTime>,
    /// Limit response to pages modified after a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub modified_after: Option<WpGmtDateTime>,
    /// Limit result set to pages assigned to specific authors.
    #[uniffi(default = [])]
    pub author: Vec<UserId>,
    /// Ensure result set excludes pages assigned to specific authors.
    #[uniffi(default = [])]
    pub author_exclude: Vec<UserId>,
    /// Limit response to pages published before a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    /// Limit response to pages modified before a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub modified_before: Option<WpGmtDateTime>,
    /// Ensure result set excludes specific IDs.
    #[uniffi(default = [])]
    pub exclude: Vec<PageId>,
    /// Limit result set to specific IDs.
    #[uniffi(default = [])]
    pub include: Vec<PageId>,
    /// Offset the result set by a specific number of items.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    /// Order sort attribute ascending or descending.
    /// Default: desc
    /// One of: asc, desc
    #[uniffi(default = None)]
    pub order: Option<WpApiParamOrder>,
    /// Sort collection by page attribute.
    /// Default: date
    /// One of: author, date, id, include, modified, parent, relevance, slug, include_slugs, title, menu_order
    #[uniffi(default = None)]
    #[field_name("orderby")]
    pub orderby: Option<WpApiParamPagesOrderBy>,
    /// Array of column names to be searched.
    #[uniffi(default = [])]
    pub search_columns: Vec<WpApiParamPostsSearchColumn>,
    /// Limit result set to pages with one or more specific slugs.
    #[uniffi(default = [])]
    pub slug: Vec<String>,
    /// Limit result set to pages assigned one or more statuses.
    /// Default: publish
    #[uniffi(default = [])]
    pub status: Vec<PageStatus>,
    /// Limit result set to pages with a specific parent.
    #[uniffi(default = None)]
    pub parent: Option<PageId>,
    /// Limit result set to pages except those of a specific parent.
    #[uniffi(default = [])]
    pub parent_exclude: Vec<PageId>,
    /// Limit result set by menu order.
    #[uniffi(default = None)]
    pub menu_order: Option<u32>,
}

#[derive(Debug, Default, uniffi::Record)]
pub struct PageRetrieveParams {
    /// The password for the page if it is password protected.
    #[uniffi(default = None)]
    pub password: Option<String>,
}

impl AppendUrlQueryPairs for PageRetrieveParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut.append_option_query_value_pair("password", self.password.as_ref());
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PageDeleteResponse {
    pub deleted: bool,
    pub previous: PageWithEditContext,
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct PageCreateParams {
    // The date the page was published, in the site's timezone.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    // The date the page was published, as GMT.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_gmt: Option<WpGmtDateTime>,
    // An alphanumeric identifier for the page unique to its type.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    // A named status for the page.
    // One of: publish, future, draft, pending, private
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<PageStatus>,
    // A password to protect access to the content and excerpt.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    // The ID for the parent of the page.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<PageId>,
    // The order of the page in relation to other pages.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu_order: Option<u32>,
    // The title for the page.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    // The content for the page.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    // The ID for the author of the page.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<UserId>,
    // The excerpt for the page.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
    // The ID of the featured media for the page.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub featured_media: Option<MediaId>,
    // Whether or not comments are open on the page.
    // One of: open, closed
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_status: Option<PageCommentStatus>,
    // Whether or not the page can be pinged.
    // One of: open, closed
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ping_status: Option<PagePingStatus>,
    // Meta fields.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<PageMeta>,
    // The theme file to use to display the page.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct PageUpdateParams {
    // The date the page was published, in the site's timezone.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    // The date the page was published, as GMT.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_gmt: Option<WpGmtDateTime>,
    // An alphanumeric identifier for the page unique to its type.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    // A named status for the page.
    // One of: publish, future, draft, pending, private
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<PageStatus>,
    // A password to protect access to the content and excerpt.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    // The ID for the parent of the page.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<PageId>,
    // The order of the page in relation to other pages.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu_order: Option<u32>,
    // The title for the page.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    // The content for the page.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    // The ID for the author of the page.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<UserId>,
    // The excerpt for the page.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
    // The ID of the featured media for the page.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub featured_media: Option<MediaId>,
    // Whether or not comments are open on the page.
    // One of: open, closed
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_status: Option<PageCommentStatus>,
    // Whether or not the page can be pinged.
    // One of: open, closed
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ping_status: Option<PagePingStatus>,
    // Meta fields.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<PageMeta>,
    // The theme file to use to display the page.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

wp_content_i64_id!(PageId);

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePage {
    #[WpContext(edit, embed, view)]
    pub id: Option<PageId>,
    #[WpContext(edit, embed, view)]
    pub date: Option<String>,
    #[WpContext(edit, view)]
    pub date_gmt: Option<WpGmtDateTime>,
    #[WpContext(edit, view)]
    #[WpContextualField]
    pub guid: Option<SparsePageGuid>,
    #[WpContext(edit, embed, view)]
    pub link: Option<String>,
    #[WpContext(edit, view)]
    pub modified: Option<String>,
    #[WpContext(edit, view)]
    pub modified_gmt: Option<WpGmtDateTime>,
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WpContext(edit, view)]
    pub status: Option<PageStatus>,
    #[serde(rename = "type")]
    #[WpContext(edit, embed, view)]
    pub page_type: Option<String>,
    #[WpContext(edit)]
    pub password: Option<String>,
    #[WpContext(edit)]
    pub permalink_template: Option<String>,
    #[WpContext(edit)]
    pub generated_slug: Option<String>,
    #[WpContext(edit, view)]
    pub parent: Option<PageId>,
    #[WpContext(edit, view)]
    pub menu_order: Option<u32>,
    #[WpContext(edit, embed, view)]
    #[WpContextualField]
    pub title: Option<SparsePageTitle>,
    #[WpContext(edit, view)]
    #[WpContextualField]
    pub content: Option<SparsePageContent>,
    #[WpContext(edit, embed, view)]
    pub author: Option<UserId>,
    #[WpContext(edit, embed, view)]
    #[WpContextualField]
    pub excerpt: Option<SparsePageExcerpt>,
    #[WpContext(edit, embed, view)]
    pub featured_media: Option<MediaId>,
    #[WpContext(edit, view)]
    pub comment_status: Option<PageCommentStatus>,
    #[WpContext(edit, view)]
    pub ping_status: Option<PagePingStatus>,
    #[WpContext(edit, view)]
    pub meta: Option<PageMeta>,
    #[WpContext(edit, view)]
    pub template: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePageGuid {
    #[WpContext(edit)]
    #[WpContextualOption]
    pub raw: Option<String>,
    #[WpContext(edit, view)]
    pub rendered: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePageTitle {
    #[WpContext(edit)]
    #[WpContextualOption]
    pub raw: Option<String>,
    #[WpContext(edit, embed, view)]
    pub rendered: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePageContent {
    #[WpContext(edit)]
    #[WpContextualOption]
    pub raw: Option<String>,
    #[WpContext(edit, view)]
    pub rendered: Option<String>,
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub protected: Option<bool>,
    #[WpContext(edit)]
    #[WpContextualOption]
    pub block_version: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePageExcerpt {
    #[WpContext(edit)]
    #[WpContextualOption]
    pub raw: Option<String>,
    #[WpContext(edit, embed, view)]
    pub rendered: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub protected: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct PageMeta {
    #[serde(deserialize_with = "deserialize_from_string_of_json_array")]
    #[serde(serialize_with = "serialize_as_json_string")]
    pub footnotes: Vec<PageFootnote>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, uniffi::Record)]
pub struct PageFootnote {
    pub id: String,
    pub content: String,
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
pub enum PageStatus {
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

impl_as_query_value_from_to_string!(PageStatus);

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
pub enum PageCommentStatus {
    Open,
    Closed,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

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
pub enum PagePingStatus {
    Open,
    Closed,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        SparseField, generate,
        unit_test_common::{
            assert_expected_and_from_query_pairs, unit_test_example_date_as_option,
            unit_test_example_date_as_query_value,
        },
    };
    use rstest::*;

    #[rstest]
    #[case(PageListParams::default(), "")]
    #[case(generate!(PageListParams, (page, Some(2))), "page=2")]
    #[case(generate!(PageListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(PageListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(PageListParams, (after, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("after"))]
    #[case(generate!(PageListParams, (modified_after, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("modified_after"))]
    #[case(generate!(PageListParams, (author, vec![UserId(1), UserId(2)])), "author=1%2C2")]
    #[case(generate!(PageListParams, (author_exclude, vec![UserId(1), UserId(2)])), "author_exclude=1%2C2")]
    #[case(generate!(PageListParams, (before, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("before"))]
    #[case(generate!(PageListParams, (modified_before, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("modified_before"))]
    #[case(generate!(PageListParams, (exclude, vec![PageId(1), PageId(2)])), "exclude=1%2C2")]
    #[case(generate!(PageListParams, (include, vec![PageId(1), PageId(2)])), "include=1%2C2")]
    #[case(generate!(PageListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(PageListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(PageListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Author))), "orderby=author")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Date))), "orderby=date")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Id))), "orderby=id")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Include))), "orderby=include")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::IncludeSlugs))), "orderby=include_slugs")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::MenuOrder))), "orderby=menu_order")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Modified))), "orderby=modified")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Parent))), "orderby=parent")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Relevance))), "orderby=relevance")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Slug))), "orderby=slug")]
    #[case(generate!(PageListParams, (orderby, Some(WpApiParamPagesOrderBy::Title))), "orderby=title")]
    #[case(generate!(PageListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostContent])), "search_columns=post_content")]
    #[case(generate!(PageListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostExcerpt])), "search_columns=post_excerpt")]
    #[case(generate!(PageListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostTitle])), "search_columns=post_title")]
    #[case(generate!(PageListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostContent, WpApiParamPostsSearchColumn::PostExcerpt, WpApiParamPostsSearchColumn::PostTitle])), "search_columns=post_content%2Cpost_excerpt%2Cpost_title")]
    #[case(generate!(PageListParams, (slug, vec!["foo".to_string(), "bar".to_string()])), "slug=foo%2Cbar")]
    #[case(generate!(PageListParams, (status, vec![PageStatus::Draft])), "status=draft")]
    #[case(generate!(PageListParams, (status, vec![PageStatus::Future])), "status=future")]
    #[case(generate!(PageListParams, (status, vec![PageStatus::Pending])), "status=pending")]
    #[case(generate!(PageListParams, (status, vec![PageStatus::Private])), "status=private")]
    #[case(generate!(PageListParams, (status, vec![PageStatus::Publish])), "status=publish")]
    #[case(generate!(PageListParams, (status, vec![PageStatus::Custom("foo".to_string())])), "status=foo")]
    #[case(generate!(PageListParams, (status, vec![PageStatus::Custom("foo-bar".to_string())])), "status=foo-bar")]
    #[case(generate!(PageListParams, (status, vec![PageStatus::Custom("foo_bar".to_string())])), "status=foo_bar")]
    #[case(generate!(PageListParams, (status, vec![PageStatus::Custom("FooBar".to_string())])), "status=FooBar")]
    #[case(generate!(PageListParams, (status, vec![PageStatus::Draft, PageStatus::Future, PageStatus::Pending, PageStatus::Private, PageStatus::Publish, PageStatus::Custom("foo".to_string())])), "status=draft%2Cfuture%2Cpending%2Cprivate%2Cpublish%2Cfoo")]
    #[case(generate!(PageListParams, (parent, Some(PageId(1)))), "parent=1")]
    #[case(generate!(PageListParams, (parent_exclude, vec![PageId(1), PageId(2)])), "parent_exclude=1%2C2")]
    #[case(generate!(PageListParams, (menu_order, Some(1))), "menu_order=1")]
    #[case(
        page_list_params_with_all_fields(),
        &expected_query_pairs_for_page_list_params_with_all_fields()
    )]
    #[trace]
    fn test_page_list_query_pairs(#[case] params: PageListParams, #[case] expected_query: &str) {
        assert_expected_and_from_query_pairs(params, expected_query);
    }

    #[rstest]
    #[case(SparsePageFieldWithEditContext::Id, "id")]
    #[case(SparsePageFieldWithEditContext::PageType, "type")]
    fn test_as_mapped_field_name_for_edit_context(
        #[case] field: SparsePageFieldWithEditContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }

    #[rstest]
    #[case(SparsePageFieldWithEmbedContext::Id, "id")]
    #[case(SparsePageFieldWithEmbedContext::PageType, "type")]
    fn test_as_mapped_field_name_for_embed_context(
        #[case] field: SparsePageFieldWithEmbedContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }

    #[rstest]
    #[case(SparsePageFieldWithViewContext::Id, "id")]
    #[case(SparsePageFieldWithViewContext::PageType, "type")]
    fn test_as_mapped_field_name_for_view_context(
        #[case] field: SparsePageFieldWithViewContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }

    fn expected_query_pairs_for_page_list_params_with_all_fields() -> String {
        let after = unit_test_example_date_as_query_value("after");
        let modified_after = unit_test_example_date_as_query_value("modified_after");
        let before = unit_test_example_date_as_query_value("before");
        let modified_before = unit_test_example_date_as_query_value("modified_before");
        format!(
            "page=2&per_page=2&search=foo&{after}&{modified_after}&author=1%2C2&author_exclude=1%2C2&{before}&{modified_before}&exclude=1%2C2&include=1%2C2&offset=2&order=asc&orderby=author&search_columns=post_content%2Cpost_excerpt%2Cpost_title&slug=foo%2Cbar&status=draft%2Cfuture%2Cpending%2Cprivate%2Cpublish%2Cfoo&parent=1&parent_exclude=1%2C2&menu_order=1"
        )
    }

    fn page_list_params_with_all_fields() -> PageListParams {
        PageListParams {
            after: unit_test_example_date_as_option(),
            author: vec![UserId(1), UserId(2)],
            author_exclude: vec![UserId(1), UserId(2)],
            before: unit_test_example_date_as_option(),
            exclude: vec![PageId(1), PageId(2)],
            include: vec![PageId(1), PageId(2)],
            modified_after: unit_test_example_date_as_option(),
            modified_before: unit_test_example_date_as_option(),
            offset: Some(2),
            order: Some(WpApiParamOrder::Asc),
            orderby: Some(WpApiParamPagesOrderBy::Author),
            page: Some(2),
            per_page: Some(2),
            search: Some("foo".to_string()),
            search_columns: vec![
                WpApiParamPostsSearchColumn::PostContent,
                WpApiParamPostsSearchColumn::PostExcerpt,
                WpApiParamPostsSearchColumn::PostTitle,
            ],
            slug: vec!["foo".to_string(), "bar".to_string()],
            status: vec![
                PageStatus::Draft,
                PageStatus::Future,
                PageStatus::Pending,
                PageStatus::Private,
                PageStatus::Publish,
                PageStatus::Custom("foo".to_string()),
            ],
            parent: Some(PageId(1)),
            parent_exclude: vec![PageId(1), PageId(2)],
            menu_order: Some(1),
        }
    }
}
