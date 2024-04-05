use std::{collections::HashMap, fmt::Display};

use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use url::Url;
use wp_derive::WPContextual;

use crate::{ClientErrorType, WPApiError, WPContext, WPNetworkResponse};

#[uniffi::export]
pub fn parse_list_users_response_with_edit_context(
    response: &WPNetworkResponse,
) -> Result<Vec<UserWithEditContext>, WPApiError> {
    parse_list_users_response(response)
}

#[uniffi::export]
pub fn parse_list_users_response_with_embed_context(
    response: &WPNetworkResponse,
) -> Result<Vec<UserWithEmbedContext>, WPApiError> {
    parse_list_users_response(response)
}

#[uniffi::export]
pub fn parse_list_users_response_with_view_context(
    response: &WPNetworkResponse,
) -> Result<Vec<UserWithViewContext>, WPApiError> {
    parse_list_users_response(response)
}

#[uniffi::export]
pub fn parse_retrieve_user_response_with_edit_context(
    response: &WPNetworkResponse,
) -> Result<Option<UserWithEditContext>, WPApiError> {
    parse_retrieve_user_response(response)
}

#[uniffi::export]
pub fn parse_retrieve_user_response_with_embed_context(
    response: &WPNetworkResponse,
) -> Result<Option<UserWithEmbedContext>, WPApiError> {
    parse_retrieve_user_response(response)
}

#[uniffi::export]
pub fn parse_retrieve_user_response_with_view_context(
    response: &WPNetworkResponse,
) -> Result<Option<UserWithViewContext>, WPApiError> {
    parse_retrieve_user_response(response)
}

pub fn parse_list_users_response<'de, T: Deserialize<'de>>(
    response: &'de WPNetworkResponse,
) -> Result<Vec<T>, WPApiError> {
    if let Some(client_error_type) = ClientErrorType::from_status_code(response.status_code) {
        return Err(WPApiError::ClientError {
            error_type: client_error_type,
            status_code: response.status_code,
        });
    }
    let status = http::StatusCode::from_u16(response.status_code).unwrap();
    if status.is_server_error() {
        return Err(WPApiError::ServerError {
            status_code: response.status_code,
        });
    }

    let user_list: Vec<T> =
        serde_json::from_slice(&response.body).map_err(|err| WPApiError::ParsingError {
            reason: err.to_string(),
            response: std::str::from_utf8(&response.body).unwrap().to_string(),
        })?;

    Ok(user_list)
}

pub fn parse_retrieve_user_response<'de, T: Deserialize<'de> + std::fmt::Debug>(
    response: &'de WPNetworkResponse,
) -> Result<T, WPApiError> {
    if let Some(client_error_type) = ClientErrorType::from_status_code(response.status_code) {
        return Err(WPApiError::ClientError {
            error_type: client_error_type,
            status_code: response.status_code,
        });
    }
    let status = http::StatusCode::from_u16(response.status_code).unwrap();
    if status.is_server_error() {
        return Err(WPApiError::ServerError {
            status_code: response.status_code,
        });
    }

    let user: T =
        serde_json::from_slice(&response.body).map_err(|err| WPApiError::ParsingError {
            reason: err.to_string(),
            response: std::str::from_utf8(&response.body).unwrap().to_string(),
        })?;
    Ok(user)
}

pub struct UsersEndpoint {}

impl UsersEndpoint {
    pub fn list_users(site_url: &Url, context: WPContext, params: Option<&UserListParams>) -> Url {
        let mut url = site_url.join("/wp-json/wp/v2/users").unwrap();
        url.query_pairs_mut()
            .append_pair("context", &context.to_string());
        if let Some(params) = params {
            url.query_pairs_mut().extend_pairs(params.query_pairs());
        }
        url
    }

    pub fn retrieve_user(site_url: &Url, user_id: UserId, context: WPContext) -> Url {
        let mut url = site_url
            .join(format!("/wp-json/wp/v2/users/{}", user_id).as_str())
            .unwrap();
        url.query_pairs_mut()
            .append_pair("context", &context.to_string());
        url
    }

    pub fn retrieve_current_user(site_url: &Url, context: WPContext) -> Url {
        let mut url = site_url.join("/wp-json/wp/v2/users/me").unwrap();
        url.query_pairs_mut()
            .append_pair("context", &context.to_string());
        url
    }

    pub fn create_user(site_url: &Url) -> Url {
        site_url.join("/wp-json/wp/v2/users").unwrap()
    }

    pub fn update_user(site_url: &Url, user_id: UserId, params: &UserUpdateParams) -> Url {
        site_url
            .join(format!("/wp-json/wp/v2/users/{}", user_id).as_str())
            .unwrap()
    }

    pub fn update_current_user(site_url: &Url) -> Url {
        site_url.join("/wp-json/wp/v2/users/me").unwrap()
    }

    pub fn delete_user(site_url: &Url, user_id: UserId, params: &UserDeleteParams) -> Url {
        let mut url = site_url
            .join(format!("/wp-json/wp/v2/users/{}", user_id).as_str())
            .unwrap();
        url.query_pairs_mut().extend_pairs(params.query_pairs());
        url
    }

    pub fn delete_current_user(site_url: &Url, params: &UserDeleteParams) -> Url {
        let mut url = site_url.join("/wp-json/wp/v2/users/me").unwrap();
        url.query_pairs_mut().extend_pairs(params.query_pairs());
        url
    }
}

// TODO: Should be in a centralized mod
// TODO: Need a better name
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum WPApiOrderParam {
    Asc,
    Desc,
}

impl Default for WPApiOrderParam {
    fn default() -> Self {
        Self::Asc
    }
}

impl Display for WPApiOrderParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Asc => "asc",
                Self::Desc => "desc",
            }
        )
    }
}

// TODO: Need a better name?
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum UserListParamOrderBy {
    Id,
    Include,
    Name,
    RegisteredDate,
    Slug,
    IncludeSlugs,
    Email,
    Url,
}

impl Default for UserListParamOrderBy {
    fn default() -> Self {
        Self::Name
    }
}

impl Display for UserListParamOrderBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
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
        )
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
    pub order: Option<WPApiOrderParam>,
    /// Sort collection by user attribute.
    /// Default: `name`
    /// One of: `id`, `include`, `name`, `registered_date`, `slug`, `include_slugs`, `email`, `url`
    pub order_by: Option<UserListParamOrderBy>,
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
            self.page.as_ref().map(|x| ("page", x.to_string())),
            self.per_page.as_ref().map(|x| ("per_page", x.to_string())),
            self.search.as_ref().map(|x| ("search", x.clone())),
            self.exclude.as_ref().map(|x| ("exclude", x.clone())),
            self.include.as_ref().map(|x| ("include", x.clone())),
            self.offset.as_ref().map(|x| ("offset", x.to_string())),
            self.order.as_ref().map(|x| ("order", x.to_string())),
            self.order_by.as_ref().map(|x| ("order_by", x.to_string())),
            Some(("slug", self.slug.join(","))),
            Some(("roles", self.roles.join(","))),
            Some(("capabilities", self.capabilities.join(","))),
            self.who.as_ref().map(|x| ("who", x.clone())),
            self.has_published_posts
                .as_ref()
                .map(|x| ("has_published_posts", x.to_string())),
        ]
        .into_iter()
        .flatten()
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
