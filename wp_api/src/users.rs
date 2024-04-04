use std::fmt::Display;

use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use wp_derive::WPContextual;

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
    pub name: Option<String>,
    /// First name for the user.
    #[builder(default)]
    pub first_name: Option<String>,
    /// Last name for the user.
    #[builder(default)]
    pub last_name: Option<String>,
    /// URL of the user.
    #[builder(default)]
    pub url: Option<String>,
    /// Description of the user.
    #[builder(default)]
    pub description: Option<String>,
    /// Locale for the user.
    /// One of: , `en_US`
    #[builder(default)]
    pub locale: Option<String>,
    /// The nickname for the user.
    #[builder(default)]
    pub nickname: Option<String>,
    /// An alphanumeric identifier for the user.
    #[builder(default)]
    pub slug: Option<String>,
    /// Roles assigned to the user.
    #[builder(default)]
    pub roles: Vec<String>,
    /// Meta fields.
    #[builder(default)]
    pub meta: Option<String>,
}

#[derive(Default, uniffi::Record)]
pub struct UserRetrieveParams {
    /// Unique identifier for the user.
    pub id: u32,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WPContextual)]
pub struct SparseUser {
    #[WPContext(edit, embed, view)]
    pub id: Option<u32>,
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
    pub capabilities: Option<UserCapabilities>,
    #[WPContext(edit)]
    pub extra_capabilities: Option<UserExtraCapabilities>,
    #[WPContext(edit, embed, view)]
    pub avatar_urls: Option<UserAvatarUrls>,
    #[WPContext(edit, view)]
    pub meta: Option<UserMeta>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct UserCapabilities {}
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct UserExtraCapabilities {}
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct UserAvatarUrls {}
#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct UserMeta {}
