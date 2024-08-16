use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;

use crate::{
    impl_as_query_value_for_new_type, impl_as_query_value_from_as_str,
    impl_as_query_value_from_to_string,
    url_query::{AppendUrlQueryPairs, AsQueryValue, QueryPairs, QueryPairsExtension},
    WpApiParamOrder,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WpApiParamUsersOrderBy {
    Id,
    Include,
    #[default]
    Name,
    RegisteredDate,
    Slug,
    IncludeSlugs,
    Email,
    Url,
}

impl_as_query_value_from_as_str!(WpApiParamUsersOrderBy);

impl WpApiParamUsersOrderBy {
    fn as_str(&self) -> &str {
        match self {
            Self::Id => "id",
            Self::Include => "include",
            Self::Name => "name",
            Self::RegisteredDate => "registered_date",
            Self::Slug => "slug",
            Self::IncludeSlugs => "include_slugs",
            Self::Email => "email",
            Self::Url => "url",
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WpApiParamUsersWho {
    #[default]
    All,
    Authors,
}

impl WpApiParamUsersWho {
    // The only valid value for this parameter is "authors"
    fn as_str(&self) -> Option<&str> {
        match self {
            Self::All => None,
            Self::Authors => Some("authors"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum WpApiParamUsersHasPublishedPosts {
    True,
    False,
    PostTypes(Vec<String>),
}

impl_as_query_value_from_to_string!(WpApiParamUsersHasPublishedPosts);

impl Display for WpApiParamUsersHasPublishedPosts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::True => true.to_string(),
                Self::False => false.to_string(),
                Self::PostTypes(post_types) => post_types.join(","),
            },
        )
    }
}

#[derive(Debug, Default, uniffi::Record)]
pub struct UserListParams {
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
    /// Ensure result set excludes specific IDs.
    #[uniffi(default = [])]
    pub exclude: Vec<UserId>,
    /// Limit result set to specific IDs.
    #[uniffi(default = [])]
    pub include: Vec<UserId>,
    /// Offset the result set by a specific number of items.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    /// Order sort attribute ascending or descending.
    /// Default: `asc`
    /// One of: `asc`, `desc`
    #[uniffi(default = None)]
    pub order: Option<WpApiParamOrder>,
    /// Sort collection by user attribute.
    /// Default: `name`
    /// One of: `id`, `include`, `name`, `registered_date`, `slug`, `include_slugs`, `email`, `url`
    #[uniffi(default = None)]
    pub orderby: Option<WpApiParamUsersOrderBy>,
    /// Limit result set to users with one or more specific slugs.
    #[uniffi(default = [])]
    pub slug: Vec<String>,
    /// Limit result set to users matching at least one specific role provided. Accepts csv list or single role.
    #[uniffi(default = [])]
    pub roles: Vec<String>,
    /// Limit result set to users matching at least one specific capability provided. Accepts csv list or single capability.
    #[uniffi(default = [])]
    pub capabilities: Vec<String>,
    /// Limit result set to users who are considered authors.
    /// One of: `authors`
    #[uniffi(default = None)]
    pub who: Option<WpApiParamUsersWho>,
    /// Limit result set to users who have published posts.
    #[uniffi(default = None)]
    pub has_published_posts: Option<WpApiParamUsersHasPublishedPosts>,
}

impl AppendUrlQueryPairs for UserListParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair("page", self.page.as_ref())
            .append_option_query_value_pair("per_page", self.per_page.as_ref())
            .append_option_query_value_pair("search", self.search.as_ref())
            .append_vec_query_value_pair("exclude", &self.exclude)
            .append_vec_query_value_pair("include", &self.include)
            .append_option_query_value_pair("offset", self.offset.as_ref())
            .append_option_query_value_pair("order", self.order.as_ref())
            .append_option_query_value_pair("orderby", self.orderby.as_ref())
            .append_vec_query_value_pair("slug", &self.slug)
            .append_vec_query_value_pair("roles", &self.roles)
            .append_vec_query_value_pair("capabilities", &self.capabilities)
            .append_option_query_value_pair(
                "who",
                self.who.as_ref().and_then(|w| w.as_str()).as_ref(),
            )
            .append_option_query_value_pair(
                "has_published_posts",
                self.has_published_posts.as_ref(),
            );
    }
}

#[derive(Debug, Serialize, uniffi::Record)]
pub struct UserCreateParams {
    /// Login name for the user.
    pub username: String,
    /// The email address for the user.
    pub email: String,
    /// Password for the user (never included).
    pub password: String,
    /// Display name for the user.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// First name for the user.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    /// Last name for the user.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    /// URL of the user.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Description of the user.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Locale for the user.
    /// One of: , `en_US`
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    /// The nickname for the user.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    /// An alphanumeric identifier for the user.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// Roles assigned to the user.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[uniffi(default = [])]
    pub roles: Vec<String>,
    /// Meta fields.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<String>,
}

impl UserCreateParams {
    pub fn new(username: String, email: String, password: String) -> Self {
        Self {
            username,
            email,
            password,
            name: None,
            first_name: None,
            last_name: None,
            url: None,
            description: None,
            locale: None,
            nickname: None,
            slug: None,
            roles: Vec::new(),
            meta: None,
        }
    }
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct UserUpdateParams {
    /// Display name for the user.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// First name for the user.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    /// Last name for the user.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    /// The email address for the user.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// URL of the user.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Description of the user.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Locale for the user.
    /// One of: , `en_US`
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    /// The nickname for the user.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    /// An alphanumeric identifier for the user.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// Roles assigned to the user.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[uniffi(default = [])]
    pub roles: Vec<String>,
    /// Password for the user (never included).
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// Meta fields.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<String>,
}

#[derive(Debug, uniffi::Record)]
pub struct UserDeleteParams {
    /// Reassign the deleted user's posts and links to this user ID.
    pub reassign: UserId,
}

impl UserDeleteParams {
    pub fn new(reassign: UserId) -> Self {
        Self { reassign }
    }
}

impl AppendUrlQueryPairs for UserDeleteParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        // From the [documentation](https://developer.wordpress.org/rest-api/reference/users/#delete-a-user):
        // > Required to be true, as users do not support trashing.
        // Since this argument always has to be `true`, we don't include it in the parameter
        // fields
        query_pairs_mut
            .append_query_value_pair("force", &true)
            .append_query_value_pair("reassign", &self.reassign);
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct UserDeleteResponse {
    pub deleted: bool,
    pub previous: UserWithEditContext,
}

impl_as_query_value_for_new_type!(UserId);
uniffi::custom_newtype!(UserId, i32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserId(pub i32);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseUser {
    #[WpContext(edit, embed, view)]
    pub id: Option<UserId>,
    #[WpContext(edit)]
    pub username: Option<String>,
    #[WpContext(edit, embed, view)]
    pub name: Option<String>,
    #[WpContext(edit)]
    pub first_name: Option<String>,
    #[WpContext(edit)]
    pub last_name: Option<String>,
    #[WpContext(edit)]
    pub email: Option<String>,
    #[WpContext(edit, embed, view)]
    pub url: Option<String>,
    #[WpContext(edit, embed, view)]
    pub description: Option<String>,
    #[WpContext(edit, embed, view)]
    pub link: Option<String>,
    #[WpContext(edit)]
    pub locale: Option<String>,
    #[WpContext(edit)]
    pub nickname: Option<String>,
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WpContext(edit)]
    pub registered_date: Option<String>,
    #[WpContext(edit)]
    pub roles: Option<Vec<String>>,
    #[WpContext(edit)]
    pub capabilities: Option<HashMap<String, bool>>,
    #[WpContext(edit)]
    pub extra_capabilities: Option<HashMap<String, bool>>,
    #[WpContext(edit, embed, view)]
    // According to our tests, `avatar_urls` is not available for all site types. It's marked with
    // `#[WpContextual]` which will make it an `Option` in the generated contextual types.
    #[WpContextualOption]
    pub avatar_urls: Option<HashMap<String, String>>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/57
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate, unit_test_common::assert_expected_query_pairs};
    use rstest::*;

    #[rstest]
    #[case(UserListParams::default(), "")]
    #[case(generate!(UserListParams, (page, Some(1))), "page=1")]
    #[case(generate!(UserListParams, (page, Some(2)), (per_page, Some(5))), "page=2&per_page=5")]
    #[case(generate!(UserListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(UserListParams, (exclude, vec![UserId(1), UserId(2)])), "exclude=1%2C2")]
    #[case(generate!(UserListParams, (include, vec![UserId(1)])), "include=1")]
    #[case(generate!(UserListParams, (per_page, Some(100)), (offset, Some(20))), "per_page=100&offset=20")]
    #[case(generate!(UserListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(UserListParams, (orderby, Some(WpApiParamUsersOrderBy::Id))), "orderby=id")]
    #[case(generate!(UserListParams, (order, Some(WpApiParamOrder::Desc)), (orderby, Some(WpApiParamUsersOrderBy::Email))), "order=desc&orderby=email")]
    #[case(generate!(UserListParams, (slug, vec!["foo".to_string(), "bar".to_string()])), "slug=foo%2Cbar")]
    #[case(generate!(UserListParams, (roles, vec!["author".to_string(), "editor".to_string()])), "roles=author%2Ceditor")]
    #[case(generate!(UserListParams, (slug, vec!["foo".to_string(), "bar".to_string()]), (roles, vec!["author".to_string(), "editor".to_string()])), "slug=foo%2Cbar&roles=author%2Ceditor")]
    #[case(generate!(UserListParams, (capabilities, vec!["edit_themes".to_string(), "delete_pages".to_string()])), "capabilities=edit_themes%2Cdelete_pages")]
    #[case::who_all_param_should_be_empty(generate!(UserListParams, (who, Some(WpApiParamUsersWho::All))), "")]
    #[case(generate!(UserListParams, (who, Some(WpApiParamUsersWho::Authors))), "who=authors")]
    #[case(generate!(UserListParams, (has_published_posts, Some(WpApiParamUsersHasPublishedPosts::True))), "has_published_posts=true")]
    #[case(generate!(UserListParams, (has_published_posts, Some(WpApiParamUsersHasPublishedPosts::PostTypes(vec!["post".to_string(), "page".to_string()])))), "has_published_posts=post%2Cpage")]
    #[trace]
    fn test_user_list_params2(#[case] params: UserListParams, #[case] expected_query: &str) {
        assert_expected_query_pairs(params, expected_query);
    }

    #[test]
    fn test_user_delete_params() {
        let params = UserDeleteParams::new(UserId(987));
        assert_expected_query_pairs(params, "force=true&reassign=987");
    }
}
