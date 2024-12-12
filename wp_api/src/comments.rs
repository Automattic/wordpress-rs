use std::{collections::HashMap, num::ParseIntError, str::FromStr};

use serde::{Deserialize, Serialize};
use strum_macros::IntoStaticStr;
use wp_contextual::WpContextual;

use crate::{
    impl_as_query_value_for_new_type, impl_as_query_value_from_as_str,
    posts::PostId,
    url_query::{
        AppendUrlQueryPairs, AsQueryValue, FromUrlQueryPairs, QueryPairs, QueryPairsExtension,
        UrlQueryPairsMap,
    },
    EnumFromStrParsingError, UserAvatarSize, UserId, WpApiParamOrder, WpResponseString,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
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

impl_as_query_value_from_as_str!(WpApiParamCommentsOrderBy);

impl WpApiParamCommentsOrderBy {
    fn as_str(&self) -> &str {
        match self {
            Self::Date => "date",
            Self::DateGmt => "date_gmt",
            Self::Id => "id",
            Self::Include => "include",
            Self::Post => "post",
            Self::Parent => "parent",
            Self::Type => "type",
        }
    }
}

impl FromStr for WpApiParamCommentsOrderBy {
    type Err = EnumFromStrParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "date" => Ok(Self::Date),
            "date_gmt" => Ok(Self::DateGmt),
            "id" => Ok(Self::Id),
            "include" => Ok(Self::Include),
            "post" => Ok(Self::Post),
            "parent" => Ok(Self::Parent),
            "type" => Ok(Self::Type),
            value => Err(EnumFromStrParsingError::UnknownVariant {
                value: value.to_string(),
            }),
        }
    }
}

impl_as_query_value_for_new_type!(CommentId);
uniffi::custom_newtype!(CommentId, i64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentId(pub i64);

impl FromStr for CommentId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl std::fmt::Display for CommentId {
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
)]
#[serde(rename_all = "snake_case")]
pub enum CommentType {
    #[default]
    Comment,
    Pingback,
    Trackback,
    #[serde(untagged)]
    Custom(String),
}

impl_as_query_value_from_as_str!(CommentType);

impl CommentType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Comment => "comment",
            Self::Pingback => "pingback",
            Self::Trackback => "trackback",
            Self::Custom(comment_type) => comment_type,
        }
    }
}

impl FromStr for CommentType {
    type Err = EnumFromStrParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "comment" => Ok(Self::Comment),
            "pingback" => Ok(Self::Pingback),
            "trackback" => Ok(Self::Trackback),
            value => Ok(Self::Custom(value.to_string())),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record)]
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
    pub after: Option<String>,
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
    pub before: Option<String>,
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
    pub comment_type: Option<CommentType>,
    /// The password for the post if it is password protected.
    #[uniffi(default = None)]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, IntoStaticStr)]
enum CommentListParamsField {
    #[strum(serialize = "page")]
    Page,
    #[strum(serialize = "per_page")]
    PerPage,
    #[strum(serialize = "search")]
    Search,
    #[strum(serialize = "after")]
    After,
    #[strum(serialize = "author")]
    Author,
    #[strum(serialize = "author_exclude")]
    AuthorExclude,
    #[strum(serialize = "author_email")]
    AuthorEmail,
    #[strum(serialize = "before")]
    Before,
    #[strum(serialize = "exclude")]
    Exclude,
    #[strum(serialize = "include")]
    Include,
    #[strum(serialize = "offset")]
    Offset,
    #[strum(serialize = "order")]
    Order,
    #[strum(serialize = "orderby")]
    Orderby,
    #[strum(serialize = "parent")]
    Parent,
    #[strum(serialize = "parent_exclude")]
    ParentExclude,
    #[strum(serialize = "post")]
    Post,
    #[strum(serialize = "status")]
    Status,
    #[strum(serialize = "type")]
    CommentType,
    #[strum(serialize = "password")]
    Password,
}

impl AppendUrlQueryPairs for CommentListParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair(CommentListParamsField::Page, self.page.as_ref())
            .append_option_query_value_pair(CommentListParamsField::PerPage, self.per_page.as_ref())
            .append_option_query_value_pair(CommentListParamsField::Search, self.search.as_ref())
            .append_option_query_value_pair(CommentListParamsField::After, self.after.as_ref())
            .append_vec_query_value_pair(CommentListParamsField::Author, &self.author)
            .append_vec_query_value_pair(
                CommentListParamsField::AuthorExclude,
                &self.author_exclude,
            )
            .append_option_query_value_pair(
                CommentListParamsField::AuthorEmail,
                self.author_email.as_ref(),
            )
            .append_option_query_value_pair(CommentListParamsField::Before, self.before.as_ref())
            .append_vec_query_value_pair(CommentListParamsField::Exclude, &self.exclude)
            .append_vec_query_value_pair(CommentListParamsField::Include, &self.include)
            .append_option_query_value_pair(CommentListParamsField::Offset, self.offset.as_ref())
            .append_option_query_value_pair(CommentListParamsField::Order, self.order.as_ref())
            .append_option_query_value_pair(CommentListParamsField::Orderby, self.orderby.as_ref())
            .append_vec_query_value_pair(CommentListParamsField::Parent, self.parent.as_ref())
            .append_vec_query_value_pair(
                CommentListParamsField::ParentExclude,
                self.parent_exclude.as_ref(),
            )
            .append_vec_query_value_pair(CommentListParamsField::Post, self.post.as_ref())
            .append_option_query_value_pair(CommentListParamsField::Status, self.status.as_ref())
            .append_option_query_value_pair(
                CommentListParamsField::CommentType,
                self.comment_type.as_ref(),
            )
            .append_option_query_value_pair(
                CommentListParamsField::Password,
                self.password.as_ref(),
            );
    }
}

impl FromUrlQueryPairs for CommentListParams {
    fn from_url_query_pairs(query_pairs: UrlQueryPairsMap) -> Option<Self> {
        Some(Self {
            page: query_pairs.get(CommentListParamsField::Page),
            per_page: query_pairs.get(CommentListParamsField::PerPage),
            search: query_pairs.get(CommentListParamsField::Search),
            after: query_pairs.get(CommentListParamsField::After),
            author: query_pairs.get_csv(CommentListParamsField::Author),
            author_exclude: query_pairs.get_csv(CommentListParamsField::AuthorExclude),
            author_email: query_pairs.get(CommentListParamsField::AuthorEmail),
            before: query_pairs.get(CommentListParamsField::Before),
            exclude: query_pairs.get_csv(CommentListParamsField::Exclude),
            include: query_pairs.get_csv(CommentListParamsField::Include),
            offset: query_pairs.get(CommentListParamsField::Offset),
            order: query_pairs.get(CommentListParamsField::Order),
            orderby: query_pairs.get(CommentListParamsField::Orderby),
            parent: query_pairs.get_csv(CommentListParamsField::Parent),
            parent_exclude: query_pairs.get_csv(CommentListParamsField::ParentExclude),
            post: query_pairs.get_csv(CommentListParamsField::Post),
            status: query_pairs.get(CommentListParamsField::Status),
            comment_type: query_pairs.get(CommentListParamsField::CommentType),
            password: query_pairs.get(CommentListParamsField::Password),
        })
    }

    fn supports_pagination() -> bool {
        true
    }
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
    pub date_gmt: Option<String>,
    #[WpContext(edit, embed, view)]
    pub link: Option<String>,
    #[WpContext(edit, embed, view)]
    pub parent: Option<CommentId>,
    #[WpContext(edit, view)]
    pub post: Option<PostId>,
    #[WpContext(edit, view)]
    pub status: Option<String>,
    #[serde(rename = "type")]
    #[WpContext(edit, embed, view)]
    pub comment_type: Option<CommentType>,
    #[WpContext(edit, embed, view)]
    pub author_avatar_urls: Option<HashMap<UserAvatarSize, WpResponseString>>,
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
)]
#[serde(rename_all = "snake_case")]
pub enum CommentStatus {
    Hold,
    #[default]
    Approve,
    Spam,
    Trash,
    #[serde(untagged)]
    Custom(String),
}

impl_as_query_value_from_as_str!(CommentStatus);

impl CommentStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Hold => "hold",
            Self::Approve => "approve",
            Self::Spam => "spam",
            Self::Trash => "trash",
            Self::Custom(status) => status,
        }
    }
}

impl FromStr for CommentStatus {
    type Err = EnumFromStrParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hold" => Ok(Self::Hold),
            "approve" => Ok(Self::Approve),
            "spam" => Ok(Self::Spam),
            "trash" => Ok(Self::Trash),
            value => Ok(Self::Custom(value.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate, unit_test_common::assert_expected_and_from_query_pairs};
    use rstest::*;

    #[rstest]
    #[case(CommentListParams::default(), "")]
    #[case(generate!(CommentListParams, (page, Some(2))), "page=2")]
    #[case(generate!(CommentListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(CommentListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(CommentListParams, (search, Some("foo + bar".to_string()))), "search=foo+%2B+bar")]
    #[case(generate!(CommentListParams, (after, Some("2023-08-14 17:00:00.000".to_string()))), "after=2023-08-14+17%3A00%3A00.000")]
    #[case(generate!(CommentListParams, (author, vec![UserId(1), UserId(2)])), "author=1%2C2")]
    #[case(generate!(CommentListParams, (author_exclude, vec![UserId(1), UserId(2)])), "author_exclude=1%2C2")]
    #[case(generate!(CommentListParams, (author_email, Some("foo".to_string()))), "author_email=foo")]
    #[case(generate!(CommentListParams, (author_email, Some("foo+bar@example.com".to_string()))), "author_email=foo%2Bbar%40example.com")]
    #[case(generate!(CommentListParams, (before, Some("2023-08-14 17:00:00.000".to_string()))), "before=2023-08-14+17%3A00%3A00.000")]
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
    #[case(generate!(CommentListParams, (status, Some(CommentStatus::Approve))), "status=approve")]
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
            after: Some("d_a".to_string()),
            author: vec![UserId(111), UserId(112)],
            author_exclude: vec![UserId(211), UserId(212)],
            author_email: Some("a_email@example.com".to_string()),
            before: Some("d_b".to_string()),
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
        "page=11&per_page=22&search=s_q&after=d_a&author=111%2C112&author_exclude=211%2C212&author_email=a_email%40example.com&before=d_b&exclude=1111%2C1112&include=2111%2C2112&offset=11111&order=desc&orderby=type&parent=44444%2C44445&parent_exclude=55555%2C55556&post=66666%2C66667&status=spam&type=pingback&password=p_q"
    )]
    #[trace]
    fn test_comment_list_query_pairs(
        #[case] params: CommentListParams,
        #[case] expected_query: &str,
    ) {
        assert_expected_and_from_query_pairs(params, expected_query);
    }
}
