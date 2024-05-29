use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use wp_contextual::WPContextual;

use crate::{
    add_uniffi_exported_parser, SparseField, WPApiError, WPApiParamOrder, WpNetworkResponse,
};

add_uniffi_exported_parser!(parse_filter_users_response, Vec<SparseUser>);
add_uniffi_exported_parser!(parse_filter_retrieve_user_response, SparseUser);
add_uniffi_exported_parser!(
    parse_list_users_response_with_edit_context,
    Vec<UserWithEditContext>
);
add_uniffi_exported_parser!(
    parse_list_users_response_with_embed_context,
    Vec<UserWithEmbedContext>
);
add_uniffi_exported_parser!(
    parse_list_users_response_with_view_context,
    Vec<UserWithViewContext>
);
add_uniffi_exported_parser!(
    parse_retrieve_user_response_with_edit_context,
    UserWithEditContext
);
add_uniffi_exported_parser!(
    parse_retrieve_user_response_with_embed_context,
    UserWithEmbedContext
);
add_uniffi_exported_parser!(
    parse_retrieve_user_response_with_view_context,
    UserWithViewContext
);
add_uniffi_exported_parser!(parse_delete_user_response, UserDeleteResponse);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WPApiParamUsersOrderBy {
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

impl WPApiParamUsersOrderBy {
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
pub enum WPApiParamUsersWho {
    #[default]
    All,
    Authors,
}

impl WPApiParamUsersWho {
    // The only valid value for this parameter is "authors"
    fn as_str(&self) -> Option<&str> {
        match self {
            Self::All => None,
            Self::Authors => Some("authors"),
        }
    }
}

#[derive(Debug, Default, uniffi::Record)]
pub struct UserListParams {
    /// Current page of the collection.
    /// Default: `1`
    pub page: Option<u32>,
    /// Maximum number of items to be returned in result set.
    /// Default: `10`
    pub per_page: Option<u32>,
    /// Limit results to those matching a string.
    pub search: Option<String>,
    /// Ensure result set excludes specific IDs.
    pub exclude: Vec<UserId>,
    /// Limit result set to specific IDs.
    pub include: Vec<UserId>,
    /// Offset the result set by a specific number of items.
    pub offset: Option<u32>,
    /// Order sort attribute ascending or descending.
    /// Default: `asc`
    /// One of: `asc`, `desc`
    pub order: Option<WPApiParamOrder>,
    /// Sort collection by user attribute.
    /// Default: `name`
    /// One of: `id`, `include`, `name`, `registered_date`, `slug`, `include_slugs`, `email`, `url`
    pub orderby: Option<WPApiParamUsersOrderBy>,
    /// Limit result set to users with one or more specific slugs.
    pub slug: Vec<String>,
    /// Limit result set to users matching at least one specific role provided. Accepts csv list or single role.
    pub roles: Vec<String>,
    /// Limit result set to users matching at least one specific capability provided. Accepts csv list or single capability.
    pub capabilities: Vec<String>,
    /// Limit result set to users who are considered authors.
    /// One of: `authors`
    pub who: Option<WPApiParamUsersWho>,
    /// Limit result set to users who have published posts.
    pub has_published_posts: Option<bool>,
}

impl UserListParams {
    pub fn query_pairs(&self) -> impl IntoIterator<Item = (&str, String)> {
        [
            ("page", self.page.map(|x| x.to_string())),
            ("per_page", self.per_page.map(|x| x.to_string())),
            ("search", self.search.clone()),
            (
                "exclude",
                (!self.exclude.is_empty()).then_some(
                    self.exclude
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>()
                        .join(","),
                ),
            ),
            (
                "include",
                (!self.include.is_empty()).then_some(
                    self.include
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>()
                        .join(","),
                ),
            ),
            ("offset", self.offset.map(|x| x.to_string())),
            ("order", self.order.map(|x| x.as_str().to_string())),
            ("orderby", self.orderby.map(|x| x.as_str().to_string())),
            (
                "slug",
                (!self.slug.is_empty()).then_some(self.slug.join(",")),
            ),
            (
                "roles",
                (!self.roles.is_empty()).then_some(self.roles.join(",")),
            ),
            (
                "capabilities",
                (!self.capabilities.is_empty()).then_some(self.capabilities.join(",")),
            ),
            (
                "who",
                self.who.and_then(|x| x.as_str().map(|s| s.to_string())),
            ),
            (
                "has_published_posts",
                self.has_published_posts.map(|x| x.to_string()),
            ),
        ]
        .into_iter()
        // Remove `None` values
        .filter_map(|(k, opt_v)| opt_v.map(|v| (k, v)))
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// First name for the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    /// Last name for the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    /// The email address for the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// URL of the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Description of the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Locale for the user.
    /// One of: , `en_US`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    /// The nickname for the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    /// An alphanumeric identifier for the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// Roles assigned to the user.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    /// Password for the user (never included).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// Meta fields.
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

    pub fn query_pairs(&self) -> impl IntoIterator<Item = (&str, String)> {
        [
            ("reassign", self.reassign.to_string()),
            // From the [documentation](https://developer.wordpress.org/rest-api/reference/users/#delete-a-user):
            // > Required to be true, as users do not support trashing.
            // Since this argument always has to be `true`, we don't include it in the parameter
            // fields
            ("force", true.to_string()),
        ]
        .into_iter()
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct UserDeleteResponse {
    pub deleted: bool,
    pub previous: UserWithEditContext,
}

uniffi::custom_newtype!(UserId, i32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserId(pub i32);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WPContextual)]
pub struct SparseUser {
    #[WPContext(edit, embed, view)]
    pub id: Option<UserId>,
    #[WPContext(edit)]
    pub username: Option<String>,
    #[WPContext(edit, embed, view)]
    pub name: Option<String>,
    #[WPContext(edit)]
    pub first_name: Option<String>,
    #[WPContext(edit)]
    pub last_name: Option<String>,
    #[WPContext(edit)]
    pub email: Option<String>,
    #[WPContext(edit, embed, view)]
    pub url: Option<String>,
    #[WPContext(edit, embed, view)]
    pub description: Option<String>,
    #[WPContext(edit, embed, view)]
    pub link: Option<String>,
    #[WPContext(edit)]
    pub locale: Option<String>,
    #[WPContext(edit)]
    pub nickname: Option<String>,
    #[WPContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WPContext(edit)]
    pub registered_date: Option<String>,
    #[WPContext(edit)]
    pub roles: Option<Vec<String>>,
    #[WPContext(edit)]
    pub capabilities: Option<HashMap<String, bool>>,
    #[WPContext(edit)]
    pub extra_capabilities: Option<HashMap<String, bool>>,
    #[WPContext(edit, embed, view)]
    // According to our tests, `avatar_urls` is not available for all site types. It's marked with
    // `#[WPContextualOption]` which will make it an `Option` in the generated contextual types.
    #[WPContextualOption]
    pub avatar_urls: Option<HashMap<String, String>>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/57
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SparseUserField {
    Id,
    Username,
    Name,
    LastName,
    Email,
    Url,
    Description,
    Link,
    Locale,
    Nickname,
    Slug,
    RegisteredDate,
    Roles,
    Capabilities,
    ExtraCapabilities,
    AvatarUrls,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/57
}

impl SparseField for SparseUserField {
    fn as_str(&self) -> &str {
        match self {
            Self::Id => "id",
            Self::Username => "username",
            Self::Name => "name",
            Self::LastName => "last_name",
            Self::Email => "email",
            Self::Url => "url",
            Self::Description => "description",
            Self::Link => "link",
            Self::Locale => "locale",
            Self::Nickname => "nickname",
            Self::Slug => "slug",
            Self::RegisteredDate => "registered_date",
            Self::Roles => "roles",
            Self::Capabilities => "capabilities",
            Self::ExtraCapabilities => "extra_capabilities",
            Self::AvatarUrls => "avatar_urls",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate, unit_test_common::assert_expected_query_pairs};
    use rstest::*;

    #[rstest]
    #[case(UserListParams::default(), &[])]
    #[case(generate!(UserListParams, (page, Some(1))), &[("page", "1")])]
    #[case(generate!(UserListParams, (page, Some(2)), (per_page, Some(5))), &[("page", "2"), ("per_page", "5")])]
    #[case(generate!(UserListParams, (search, Some("foo".to_string()))), &[("search", "foo")])]
    #[case(generate!(UserListParams, (exclude, vec![UserId(1), UserId(2)])), &[("exclude", "1,2")])]
    #[case(generate!(UserListParams, (include, vec![UserId(1)])), &[("include", "1")])]
    #[case(generate!(UserListParams, (per_page, Some(100)), (offset, Some(20))), &[("per_page", "100"), ("offset", "20")])]
    #[case(generate!(UserListParams, (order, Some(WPApiParamOrder::Asc))), &[("order", "asc")])]
    #[case(generate!(UserListParams, (orderby, Some(WPApiParamUsersOrderBy::Id))), &[("orderby", "id")])]
    #[case(generate!(UserListParams, (order, Some(WPApiParamOrder::Desc)), (orderby, Some(WPApiParamUsersOrderBy::Email))), &[("order", "desc"), ("orderby", "email")])]
    #[case(generate!(UserListParams, (slug, vec!["foo".to_string(), "bar".to_string()])), &[("slug", "foo,bar")])]
    #[case(generate!(UserListParams, (roles, vec!["author".to_string(), "editor".to_string()])), &[("roles", "author,editor")])]
    #[case(generate!(UserListParams, (slug, vec!["foo".to_string(), "bar".to_string()]), (roles, vec!["author".to_string(), "editor".to_string()])), &[("slug", "foo,bar"), ("roles", "author,editor")])]
    #[case(generate!(UserListParams, (capabilities, vec!["edit_themes".to_string(), "delete_pages".to_string()])), &[("capabilities", "edit_themes,delete_pages")])]
    #[case::who_all_param_should_be_empty(generate!(UserListParams, (who, Some(WPApiParamUsersWho::All))), &[])]
    #[case(generate!(UserListParams, (who, Some(WPApiParamUsersWho::Authors))), &[("who", "authors")])]
    #[case(generate!(UserListParams, (has_published_posts, Some(true))), &[("has_published_posts", "true")])]
    #[trace]
    fn test_user_list_params(
        #[case] params: UserListParams,
        #[case] expected_pairs: &[(&str, &str)],
    ) {
        assert_expected_query_pairs(params.query_pairs(), expected_pairs);
    }

    #[test]
    fn test_user_delete_params() {
        let params = UserDeleteParams::new(UserId(987));
        assert_expected_query_pairs(
            params.query_pairs(),
            &[("force", "true"), ("reassign", "987")],
        );
    }
}
