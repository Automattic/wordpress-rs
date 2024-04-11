use std::collections::HashMap;

use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use wp_derive::WPContextual;

use crate::{parse_response_for_generic_errors, WPApiError, WPApiParamOrder, WPNetworkResponse};

#[uniffi::export]
pub fn parse_list_users_response_with_edit_context(
    response: &WPNetworkResponse,
) -> Result<Vec<UserWithEditContext>, WPApiError> {
    parse_users_response(response)
}

#[uniffi::export]
pub fn parse_list_users_response_with_embed_context(
    response: &WPNetworkResponse,
) -> Result<Vec<UserWithEmbedContext>, WPApiError> {
    parse_users_response(response)
}

#[uniffi::export]
pub fn parse_list_users_response_with_view_context(
    response: &WPNetworkResponse,
) -> Result<Vec<UserWithViewContext>, WPApiError> {
    parse_users_response(response)
}

#[uniffi::export]
pub fn parse_retrieve_user_response_with_edit_context(
    response: &WPNetworkResponse,
) -> Result<UserWithEditContext, WPApiError> {
    parse_users_response(response)
}

#[uniffi::export]
pub fn parse_retrieve_user_response_with_embed_context(
    response: &WPNetworkResponse,
) -> Result<UserWithEmbedContext, WPApiError> {
    parse_users_response(response)
}

#[uniffi::export]
pub fn parse_retrieve_user_response_with_view_context(
    response: &WPNetworkResponse,
) -> Result<UserWithViewContext, WPApiError> {
    parse_users_response(response)
}

pub fn parse_users_response<'de, T: Deserialize<'de>>(
    response: &'de WPNetworkResponse,
) -> Result<T, WPApiError> {
    parse_response_for_generic_errors(response)?;
    serde_json::from_slice(&response.body).map_err(|err| WPApiError::ParsingError {
        reason: err.to_string(),
        response: String::from_utf8_lossy(&response.body).to_string(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WPApiParamUsersOrderBy {
    Id,
    Include,
    Name,
    RegisteredDate,
    Slug,
    IncludeSlugs,
    Email,
    Url,
}

impl Default for WPApiParamUsersOrderBy {
    fn default() -> Self {
        Self::Name
    }
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

#[derive(Default, uniffi::Record)]
pub struct UserListParams {
    // TODO: Implement the `_filter`
    /// Current page of the collection.
    /// Default: `1`
    pub page: Option<u32>,
    /// Maximum number of items to be returned in result set.
    /// Default: `10`
    pub per_page: Option<u32>,
    /// Limit results to those matching a string.
    pub search: Option<String>,
    /// Ensure result set excludes specific IDs.
    pub exclude: Option<String>,
    /// Limit result set to specific IDs.
    pub include: Option<String>,
    /// Offset the result set by a specific number of items.
    pub offset: Option<u32>,
    /// Order sort attribute ascending or descending.
    /// Default: `asc`
    /// One of: `asc`, `desc`
    pub order: Option<WPApiParamOrder>,
    /// Sort collection by user attribute.
    /// Default: `name`
    /// One of: `id`, `include`, `name`, `registered_date`, `slug`, `include_slugs`, `email`, `url`
    pub order_by: Option<WPApiParamUsersOrderBy>,
    /// Limit result set to users with one or more specific slugs.
    pub slug: Vec<String>,
    /// Limit result set to users matching at least one specific role provided. Accepts csv list or single role.
    pub roles: Vec<String>,
    /// Limit result set to users matching at least one specific capability provided. Accepts csv list or single capability.
    pub capabilities: Vec<String>,
    /// Limit result set to users who are considered authors.
    /// One of: `authors`
    pub who: Option<String>,
    /// Limit result set to users who have published posts.
    pub has_published_posts: Option<bool>,
}

impl UserListParams {
    pub fn query_pairs(&self) -> impl IntoIterator<Item = (&str, String)> {
        [
            ("page", self.page.map(|x| x.to_string())),
            ("per_page", self.per_page.map(|x| x.to_string())),
            ("search", self.search.clone()),
            ("exclude", self.exclude.clone()),
            ("include", self.include.clone()),
            ("offset", self.offset.map(|x| x.to_string())),
            ("order", self.order.map(|x| x.as_str().to_string())),
            ("order_by", self.order_by.map(|x| x.as_str().to_string())),
            ("slug", Some(self.slug.join(","))),
            ("roles", Some(self.roles.join(","))),
            ("capabilities", Some(self.capabilities.join(","))),
            ("who", self.who.clone()),
            (
                "has_published_post",
                self.has_published_posts.map(|x| x.to_string()),
            ),
        ]
        .into_iter()
        // Remove `None` values
        .filter_map(|(k, opt_v)| opt_v.map(|v| (k, v)))
    }
}

#[derive(Builder, Serialize, uniffi::Record)]
pub struct UserCreateParams {
    /// Login name for the user.
    pub username: String,
    /// The email address for the user.
    pub email: String,
    /// Password for the user (never included).
    pub password: String,
    /// Display name for the user.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// First name for the user.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    /// Last name for the user.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    /// URL of the user.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Description of the user.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Locale for the user.
    /// One of: , `en_US`
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    /// The nickname for the user.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    /// An alphanumeric identifier for the user.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// Roles assigned to the user.
    #[builder(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    /// Meta fields.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<String>,
}

#[derive(Builder, Serialize, uniffi::Record)]
pub struct UserUpdateParams {
    /// Display name for the user.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// First name for the user.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    /// Last name for the user.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    /// The email address for the user.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// URL of the user.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Description of the user.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Locale for the user.
    /// One of: , `en_US`
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    /// The nickname for the user.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    /// An alphanumeric identifier for the user.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// Roles assigned to the user.
    #[builder(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    /// Password for the user (never included).
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// Meta fields.
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<String>,
}

#[derive(uniffi::Record)]
pub struct UserDeleteParams {
    /// Reassign the deleted user's posts and links to this user ID.
    pub reassign: UserId,
}

impl UserDeleteParams {
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

uniffi::custom_newtype!(UserId, i32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserId(i32);

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
    pub avatar_urls: Option<HashMap<String, String>>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/57
}
