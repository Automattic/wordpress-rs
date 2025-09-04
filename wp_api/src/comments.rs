use crate::{
    AnyJson, UserAvatarSize, UserId, WpApiParamOrder, WpResponseString,
    date::WpGmtDateTime,
    impl_as_query_value_from_to_string,
    posts::PostId,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_i64_id,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr, sync::Arc};
use wp_contextual::WpContextual;
use wp_derive::WpDeriveParamsField;

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
pub enum WpApiParamCommentsOrderBy {
    Date,
    #[default]
    DateGmt,
    Id,
    Include,
    Post,
    Parent,
    Type,
}

impl_as_query_value_from_to_string!(WpApiParamCommentsOrderBy);

wp_content_i64_id!(CommentId);

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
pub enum CommentType {
    #[default]
    Comment,
    Pingback,
    Trackback,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

impl_as_query_value_from_to_string!(CommentType);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct CommentListParams {
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
    /// Limit response to comments published after a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub after: Option<WpGmtDateTime>,
    /// Limit result set to comments assigned to specific user IDs. Requires authorization.
    #[uniffi(default = [])]
    pub author: Vec<UserId>,
    /// Ensure result set excludes comments assigned to specific user IDs. Requires authorization.
    #[uniffi(default = [])]
    pub author_exclude: Vec<UserId>,
    /// Limit result set to that from a specific author email. Requires authorization.
    #[uniffi(default = None)]
    pub author_email: Option<String>,
    /// Limit response to comments published before a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    /// Ensure result set excludes specific IDs.
    #[uniffi(default = [])]
    pub exclude: Vec<CommentId>,
    /// Limit result set to specific IDs.
    #[uniffi(default = [])]
    pub include: Vec<CommentId>,
    /// Offset the result set by a specific number of items.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    /// Order sort attribute ascending or descending.
    /// Default: desc
    /// One of: asc, desc
    #[uniffi(default = None)]
    pub order: Option<WpApiParamOrder>,
    /// Sort collection by comment attribute.
    /// Default: date_gmt
    /// One of: date, date_gmt, id, include, post, parent, type
    #[uniffi(default = None)]
    #[field_name("orderby")]
    pub orderby: Option<WpApiParamCommentsOrderBy>,
    /// Limit result set to comments of specific parent IDs.
    #[uniffi(default = [])]
    pub parent: Vec<CommentId>,
    /// Ensure result set excludes specific parent IDs.
    #[uniffi(default = [])]
    pub parent_exclude: Vec<CommentId>,
    /// Limit result set to comments assigned to specific post IDs.
    #[uniffi(default = [])]
    pub post: Vec<PostId>,
    /// Limit result set to comments assigned a specific status. Requires authorization.
    /// Default: approve
    #[uniffi(default = None)]
    pub status: Option<CommentStatus>,
    /// Limit result set to comments assigned a specific type. Requires authorization.
    /// Default: comment
    #[uniffi(default = None)]
    #[field_name("type")]
    pub comment_type: Option<CommentType>,
    /// The password for the post if it is password protected.
    #[uniffi(default = None)]
    pub password: Option<String>,
}

#[derive(Debug, Default, uniffi::Record)]
pub struct CommentRetrieveParams {
    /// The password for the parent post of the comment (if the post is password protected).
    #[uniffi(default = None)]
    pub password: Option<String>,
}

impl AppendUrlQueryPairs for CommentRetrieveParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut.append_option_query_value_pair("password", self.password.as_ref());
    }
}

#[derive(Debug, Default, uniffi::Record)]
pub struct CommentDeleteParams {
    /// The password for the parent post of the comment (if the post is password protected).
    #[uniffi(default = None)]
    pub password: Option<String>,
}

impl CommentDeleteParams {
    pub fn new(password: Option<String>) -> Self {
        Self { password }
    }
}

impl AppendUrlQueryPairs for CommentDeleteParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut.append_option_query_value_pair("password", self.password.as_ref());
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct CommentDeleteResponse {
    pub deleted: bool,
    pub previous: CommentWithEditContext,
}

#[derive(Debug, Serialize, uniffi::Record)]
pub struct CommentCreateParams {
    /// The ID of the associated post object.
    pub post: PostId,
    /// The ID of the user object, if author was a user.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<UserId>,
    /// Email address for the comment author.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_email: Option<String>,
    /// IP address for the comment author.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_ip: Option<String>,
    /// Display name for the comment author.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    /// URL for the comment author.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_url: Option<String>,
    /// User agent for the comment author.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_user_agent: Option<String>,
    /// The content for the comment.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// The date the comment was published, in the site's timezone.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// The date the comment was published, as GMT.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_gmt: Option<WpGmtDateTime>,
    /// The ID for the parent of the comment.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<CommentId>,
    /// State of the comment.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<CommentStatus>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/422
}

impl CommentCreateParams {
    pub fn new(post: PostId, content: String) -> Self {
        Self {
            post,
            author: None,
            author_email: None,
            author_ip: None,
            author_name: None,
            author_url: None,
            author_user_agent: None,
            content: Some(content),
            date: None,
            date_gmt: None,
            parent: None,
            status: None,
        }
    }
}

#[derive(Debug)]
pub struct CommentCreateParamsBuilder {
    params: CommentCreateParams,
}

impl CommentCreateParamsBuilder {
    pub fn new(post: PostId, content: String) -> Self {
        Self {
            params: CommentCreateParams::new(post, content),
        }
    }

    pub fn author(mut self, author: Option<UserId>) -> Self {
        self.params.author = author;
        self
    }

    pub fn author_email(mut self, author_email: Option<String>) -> Self {
        self.params.author_email = author_email;
        self
    }

    pub fn author_ip(mut self, author_ip: Option<String>) -> Self {
        self.params.author_ip = author_ip;
        self
    }

    pub fn author_name(mut self, author_name: Option<String>) -> Self {
        self.params.author_name = author_name;
        self
    }
    pub fn author_url(mut self, author_url: Option<String>) -> Self {
        self.params.author_url = author_url;
        self
    }

    pub fn author_user_agent(mut self, author_user_agent: Option<String>) -> Self {
        self.params.author_user_agent = author_user_agent;
        self
    }
    pub fn date(mut self, date: Option<String>) -> Self {
        self.params.date = date;
        self
    }

    pub fn date_gmt(mut self, date_gmt: Option<WpGmtDateTime>) -> Self {
        self.params.date_gmt = date_gmt;
        self
    }

    pub fn parent(mut self, parent: Option<CommentId>) -> Self {
        self.params.parent = parent;
        self
    }

    pub fn status(mut self, status: Option<CommentStatus>) -> Self {
        self.params.status = status;
        self
    }

    pub fn build(self) -> CommentCreateParams {
        self.params
    }
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct CommentUpdateParams {
    /// The ID of the user object, if author was a user.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<UserId>,
    /// Email address for the comment author.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_email: Option<String>,
    /// IP address for the comment author.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_ip: Option<String>,
    /// Display name for the comment author.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    /// URL for the comment author.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_url: Option<String>,
    /// User agent for the comment author.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_user_agent: Option<String>,
    /// The content for the comment.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// The date the comment was published, in the site's timezone.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// The date the comment was published, as GMT.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_gmt: Option<WpGmtDateTime>,
    /// The ID for the parent of the comment.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<CommentId>,
    /// The ID of the associated post object.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<PostId>,
    /// State of the comment.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<CommentStatus>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/422
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseComment {
    #[WpContext(edit, embed, view)]
    pub id: Option<CommentId>,
    #[WpContext(edit, embed, view)]
    pub author: Option<UserId>,
    #[WpContext(edit)]
    pub author_email: Option<String>,
    #[WpContext(edit)]
    pub author_ip: Option<String>,
    #[WpContext(edit, embed, view)]
    pub author_name: Option<String>,
    #[WpContext(edit, embed, view)]
    pub author_url: Option<String>,
    #[WpContext(edit)]
    pub author_user_agent: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualField]
    pub content: Option<SparseCommentContent>,
    #[WpContext(edit, embed, view)]
    pub date: Option<String>,
    #[WpContext(edit, view)]
    pub date_gmt: Option<WpGmtDateTime>,
    #[WpContext(edit, embed, view)]
    pub link: Option<String>,
    #[WpContext(edit, embed, view)]
    pub parent: Option<CommentId>,
    #[WpContext(edit, view)]
    pub post: Option<PostId>,
    #[WpContext(edit, view)]
    pub status: Option<CommentStatus>,
    #[serde(rename = "type")]
    #[WpContext(edit, embed, view)]
    pub comment_type: Option<CommentType>,
    #[WpContext(edit, embed, view)]
    pub author_avatar_urls: Option<HashMap<UserAvatarSize, WpResponseString>>,
    #[serde(flatten)]
    #[WpContext(edit, embed, view)]
    #[WpContextualExcludeFromFields]
    pub additional_fields: Option<Arc<AnyJson>>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/422
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseCommentContent {
    #[WpContext(edit)]
    pub raw: Option<String>,
    #[WpContext(edit, view, embed)]
    pub rendered: Option<String>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(untagged)]
pub enum CommentAuthorAvatarUrlSize {
    // Server may return `false` instead of an empty string or null value
    Bool(bool),
    String(String),
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
pub enum CommentStatus {
    Hold,
    #[default]
    Approved,
    Spam,
    Trash,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

impl_as_query_value_from_to_string!(CommentStatus);

#[uniffi::export]
fn comment_status_from_string(value: String) -> CommentStatus {
    CommentStatus::from_str(value.as_str()).unwrap_or(CommentStatus::Custom(value))
}

#[uniffi::export]
fn comment_status_to_string(status: CommentStatus) -> String {
    status.to_string()
}

#[uniffi::export]
fn comment_type_from_string(value: String) -> CommentType {
    CommentType::from_str(value.as_str()).unwrap_or(CommentType::Custom(value))
}

#[uniffi::export]
fn comment_type_to_string(comment_type: CommentType) -> String {
    comment_type.to_string()
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
    #[case(CommentListParams::default(), "")]
    #[case(generate!(CommentListParams, (page, Some(2))), "page=2")]
    #[case(generate!(CommentListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(CommentListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(CommentListParams, (search, Some("foo + bar".to_string()))), "search=foo+%2B+bar")]
    #[case(generate!(CommentListParams, (after, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("after"))]
    #[case(generate!(CommentListParams, (author, vec![UserId(1), UserId(2)])), "author=1%2C2")]
    #[case(generate!(CommentListParams, (author_exclude, vec![UserId(1), UserId(2)])), "author_exclude=1%2C2")]
    #[case(generate!(CommentListParams, (author_email, Some("foo".to_string()))), "author_email=foo")]
    #[case(generate!(CommentListParams, (author_email, Some("foo+bar@example.com".to_string()))), "author_email=foo%2Bbar%40example.com")]
    #[case(generate!(CommentListParams, (before, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("before"))]
    #[case(generate!(CommentListParams, (exclude, vec![CommentId(1), CommentId(2)])), "exclude=1%2C2")]
    #[case(generate!(CommentListParams, (include, vec![CommentId(1), CommentId(2)])), "include=1%2C2")]
    #[case(generate!(CommentListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(CommentListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(CommentListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(CommentListParams, (orderby, Some(WpApiParamCommentsOrderBy::Date))), "orderby=date")]
    #[case(generate!(CommentListParams, (orderby, Some(WpApiParamCommentsOrderBy::DateGmt))), "orderby=date_gmt")]
    #[case(generate!(CommentListParams, (orderby, Some(WpApiParamCommentsOrderBy::Id))), "orderby=id")]
    #[case(generate!(CommentListParams, (orderby, Some(WpApiParamCommentsOrderBy::Include))), "orderby=include")]
    #[case(generate!(CommentListParams, (orderby, Some(WpApiParamCommentsOrderBy::Post))), "orderby=post")]
    #[case(generate!(CommentListParams, (orderby, Some(WpApiParamCommentsOrderBy::Parent))), "orderby=parent")]
    #[case(generate!(CommentListParams, (orderby, Some(WpApiParamCommentsOrderBy::Type))), "orderby=type")]
    #[case(generate!(CommentListParams, (parent, vec![CommentId(44444), CommentId(44445)])), "parent=44444%2C44445")]
    #[case(generate!(CommentListParams, (parent_exclude, vec![CommentId(55555), CommentId(55556)])), "parent_exclude=55555%2C55556")]
    #[case(generate!(CommentListParams, (post, vec![PostId(66666), PostId(66667)])), "post=66666%2C66667")]
    #[case(generate!(CommentListParams, (status, Some(CommentStatus::Hold))), "status=hold")]
    #[case(generate!(CommentListParams, (status, Some(CommentStatus::Approved))), "status=approved")]
    #[case(generate!(CommentListParams, (status, Some(CommentStatus::Spam))), "status=spam")]
    #[case(generate!(CommentListParams, (status, Some(CommentStatus::Trash))), "status=trash")]
    #[case(generate!(CommentListParams, (status, Some(CommentStatus::Custom("foo".to_string())))), "status=foo")]
    #[case(generate!(CommentListParams, (comment_type, Some(CommentType::Comment))), "type=comment")]
    #[case(generate!(CommentListParams, (comment_type, Some(CommentType::Pingback))), "type=pingback")]
    #[case(generate!(CommentListParams, (comment_type, Some(CommentType::Trackback))), "type=trackback")]
    #[case(generate!(CommentListParams, (comment_type, Some(CommentType::Custom("foo".to_string())))), "type=foo")]
    #[case(generate!(CommentListParams, (password, Some("foo".to_string()))), "password=foo")]
    #[case(CommentListParams {
            page: Some(11),
            per_page: Some(22),
            search: Some("s_q".to_string()),
            after: unit_test_example_date_as_option(),
            author: vec![UserId(111), UserId(112)],
            author_exclude: vec![UserId(211), UserId(212)],
            author_email: Some("a_email@example.com".to_string()),
            before: unit_test_example_date_as_option(),
            exclude: vec![CommentId(1111), CommentId(1112)],
            include: vec![CommentId(2111), CommentId(2112)],
            offset: Some(11111),
            order: Some(WpApiParamOrder::Desc),
            orderby: Some(WpApiParamCommentsOrderBy::Type),
            parent: vec![CommentId(44444), CommentId(44445)],
            parent_exclude: vec![CommentId(55555), CommentId(55556)],
            post: vec![PostId(66666), PostId(66667)],
            status: Some(CommentStatus::Spam),
            comment_type: Some(CommentType::Pingback),
            password: Some("p_q".to_string()),
        },
        &expected_query_pairs_for_comment_list_params_with_all_fields()
    )]
    #[trace]
    fn test_comment_list_query_pairs(
        #[case] params: CommentListParams,
        #[case] expected_query: &str,
    ) {
        assert_expected_and_from_query_pairs(params, expected_query);
    }

    fn expected_query_pairs_for_comment_list_params_with_all_fields() -> String {
        let after = unit_test_example_date_as_query_value("after");
        let before = unit_test_example_date_as_query_value("before");
        format!(
            "page=11&per_page=22&search=s_q&{after}&author=111%2C112&author_exclude=211%2C212&author_email=a_email%40example.com&{before}&exclude=1111%2C1112&include=2111%2C2112&offset=11111&order=desc&orderby=type&parent=44444%2C44445&parent_exclude=55555%2C55556&post=66666%2C66667&status=spam&type=pingback&password=p_q"
        )
    }

    #[rstest]
    #[case(SparseCommentFieldWithEditContext::Id, "id")]
    #[case(SparseCommentFieldWithEditContext::CommentType, "type")]
    fn test_as_mapped_field_name_for_edit_context(
        #[case] field: SparseCommentFieldWithEditContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }

    #[rstest]
    #[case(SparseCommentFieldWithEmbedContext::Id, "id")]
    #[case(SparseCommentFieldWithEmbedContext::CommentType, "type")]
    fn test_as_mapped_field_name_for_embed_context(
        #[case] field: SparseCommentFieldWithEmbedContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }

    #[rstest]
    #[case(SparseCommentFieldWithViewContext::Id, "id")]
    #[case(SparseCommentFieldWithViewContext::CommentType, "type")]
    fn test_as_mapped_field_name_for_view_context(
        #[case] field: SparseCommentFieldWithViewContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }
}
