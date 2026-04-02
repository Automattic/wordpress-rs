use crate::{
    EnumFromStrParsingError, JsonValue, OptionFromStr, WpApiParamOrder, WpResponseString,
    impl_as_query_value_from_to_string,
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_i64_id,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::Infallible, fmt::Display, str::FromStr};
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

impl_as_query_value_from_to_string!(WpApiParamUsersOrderBy);

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

impl OptionFromStr for WpApiParamUsersWho {
    type Err = EnumFromStrParsingError;

    fn option_from_str(s: &str) -> Result<Option<Self>, Self::Err> {
        match s {
            "authors" => Ok(Some(Self::Authors)),
            value => Err(EnumFromStrParsingError::UnknownVariant {
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum WpApiParamUsersHasPublishedPosts {
    True,
    False,
    PostTypes(Vec<String>),
}

impl FromStr for WpApiParamUsersHasPublishedPosts {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "true" => Ok(Self::True),
            "false" => Ok(Self::False),
            value => Ok(Self::PostTypes(
                value.split(',').map(|s| s.to_string()).collect(),
            )),
        }
    }
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

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum UserAvatarSize {
    #[serde(rename = "24")]
    Size24,
    #[serde(rename = "48")]
    Size48,
    #[serde(rename = "96")]
    Size96,
    #[serde(untagged)]
    Custom(String),
}

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
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
    #[field_name("orderby")]
    pub orderby: Option<WpApiParamUsersOrderBy>,
    /// Limit result set to users with one or more specific slugs.
    #[uniffi(default = [])]
    pub slug: Vec<String>,
    /// Limit result set to users matching at least one specific role provided. Accepts csv list or single role.
    #[uniffi(default = [])]
    pub roles: Vec<UserRole>,
    /// Limit result set to users matching at least one specific capability provided. Accepts csv list or single capability.
    #[uniffi(default = [])]
    pub capabilities: Vec<UserCapability>,
    /// Limit result set to users who are considered authors.
    /// One of: `authors`
    #[uniffi(default = None)]
    #[from_query_method("get_using_option_from_str")]
    #[append_query_custom("self.who.as_ref().and_then(|w| w.as_str()).as_ref()")]
    pub who: Option<WpApiParamUsersWho>,
    /// Limit result set to users who have published posts.
    #[uniffi(default = None)]
    pub has_published_posts: Option<WpApiParamUsersHasPublishedPosts>,
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
#[uniffi::export(Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum UserCapability {
    CreateSites,
    DeleteSites,
    ManageNetwork,
    ManageSites,
    ManageNetworkUsers,
    ManageNetworkPlugins,
    ManageNetworkThemes,
    ManageNetworkOptions,
    UpgradeNetwork,
    SetupNetwork,
    ActivatePlugins,
    DeleteOthersPages,
    DeleteOthersPosts,
    DeletePages,
    DeletePosts,
    DeletePrivatePages,
    DeletePrivatePosts,
    DeletePublishedPages,
    DeletePublishedPosts,
    EditDashboard,
    EditOthersPages,
    EditOthersPosts,
    EditPages,
    EditPosts,
    EditPrivatePages,
    EditPrivatePosts,
    EditPublishedPages,
    EditPublishedPosts,
    EditThemeOptions,
    Export,
    Import,
    ListUsers,
    ManageCategories,
    ManageLinks,
    ManageOptions,
    ModerateComments,
    PromoteUsers,
    PublishPages,
    PublishPosts,
    ReadPrivatePages,
    ReadPrivatePosts,
    Read,
    RemoveUseres,
    SwitchThemes,
    UploadFiles,
    Customize,
    DeleteSite,
    UpdateCore,
    UpdatePlugins,
    UpdateThemes,
    InstallPlugins,
    InstallThemes,
    DeleteThemes,
    DeletePlugins,
    EditPlugins,
    EditThemes,
    EditFiles,
    EditUsers,
    AddUsers,
    CreateUsers,
    DeleteUsers,
    UnfilteredHtml,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

impl_as_query_value_from_to_string!(UserCapability);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, uniffi::Record)]
#[serde(transparent)]
pub struct UserCapabilitiesMap {
    #[serde(deserialize_with = "wp_serde_helper::deserialize_empty_array_or_hashmap")]
    #[serde(flatten)]
    #[serde(rename = "capabilities")]
    pub map: HashMap<UserCapability, JsonValue>,
}

#[uniffi::export]
impl UserCapabilitiesMap {
    /// Check if the user has a specific capability.
    ///
    /// Returns `true` only if the capability is present and its value is a boolean `true`.
    /// Capabilities with non-boolean values (e.g., strings like "custom" or "any" added by
    /// plugins such as WPBakery) return `false`.
    pub fn has_cap(&self, capability: UserCapability) -> bool {
        self.map.get(&capability) == Some(&JsonValue::Bool(true))
    }
}

#[uniffi::export]
fn user_capability_from_string(value: String) -> UserCapability {
    UserCapability::from_str(value.as_str()).unwrap_or(UserCapability::Custom(value))
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
#[uniffi::export(Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum UserRole {
    SuperAdmin,
    Administrator,
    Editor,
    Author,
    Contributor,
    Subscriber,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

#[uniffi::export]
fn user_role_from_string(value: String) -> UserRole {
    UserRole::from_str(value.as_str()).unwrap_or(UserRole::Custom(value))
}

impl_as_query_value_from_to_string!(UserRole);

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
    pub roles: Vec<UserRole>,
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
    pub roles: Vec<UserRole>,
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

wp_content_i64_id!(UserId);

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
    pub roles: Option<Vec<UserRole>>,
    #[WpContext(edit)]
    pub capabilities: Option<UserCapabilitiesMap>,
    #[WpContext(edit)]
    pub extra_capabilities: Option<HashMap<String, bool>>,
    #[WpContext(edit, embed, view)]
    // According to our tests, `avatar_urls` is not available for all site types. It's marked with
    // `#[WpContextual]` which will make it an `Option` in the generated contextual types.
    #[WpContextualOption]
    pub avatar_urls: Option<HashMap<UserAvatarSize, WpResponseString>>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/57
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        generate,
        unit_test_common::{assert_expected_and_from_query_pairs, assert_expected_query_pairs},
    };
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
    #[case(generate!(UserListParams, (roles, vec![UserRole::Author, UserRole::Editor])), "roles=author%2Ceditor")]
    #[case(generate!(UserListParams, (slug, vec!["foo".to_string(), "bar".to_string()]), (roles, vec![UserRole::Author, UserRole::Editor])), "slug=foo%2Cbar&roles=author%2Ceditor")]
    #[case(generate!(UserListParams, (capabilities, vec![UserCapability::EditThemes, UserCapability::DeletePages])), "capabilities=edit_themes%2Cdelete_pages")]
    #[case(generate!(UserListParams, (who, Some(WpApiParamUsersWho::Authors))), "who=authors")]
    #[case(generate!(UserListParams, (has_published_posts, Some(WpApiParamUsersHasPublishedPosts::True))), "has_published_posts=true")]
    #[case(generate!(UserListParams, (has_published_posts, Some(WpApiParamUsersHasPublishedPosts::PostTypes(vec!["post".to_string(), "page".to_string()])))), "has_published_posts=post%2Cpage")]
    #[case(UserListParams {
            page: Some(11),
            per_page: Some(22),
            search: Some("s_q".to_string()),
            exclude: vec![UserId(111), UserId(112)],
            include: vec![UserId(211), UserId(212)],
            offset: Some(311),
            order: Some(WpApiParamOrder::Asc),
            orderby: Some(WpApiParamUsersOrderBy::Email),
            slug: vec!["s1".to_string(), "s2".to_string()],
            roles: vec![UserRole::Custom("r1".to_string()), UserRole::Custom("r2".to_string())],
            capabilities: vec![UserCapability::Custom("c1".to_string()), UserCapability::Custom("c2".to_string())],
            who: Some(WpApiParamUsersWho::Authors),
            has_published_posts: Some(WpApiParamUsersHasPublishedPosts::True),
        }, "page=11&per_page=22&search=s_q&exclude=111%2C112&include=211%2C212&offset=311&order=asc&orderby=email&slug=s1%2Cs2&roles=r1%2Cr2&capabilities=c1%2Cc2&who=authors&has_published_posts=true")]
    #[trace]
    fn test_user_list_query_pairs(#[case] params: UserListParams, #[case] expected_query: &str) {
        assert_expected_and_from_query_pairs(params, expected_query);
    }

    #[test]
    fn test_user_list_params_who_all_should_be_empty() {
        // This test is separate from `test_user_list_query_pairs` because converting the empty
        // query back to `UserListParams` will result in `who: None` instead of
        // `who: WpApiParamUsersWho::All`.
        assert_expected_query_pairs(
            UserListParams {
                who: Some(WpApiParamUsersWho::All),
                ..Default::default()
            },
            "",
        );
    }

    #[test]
    fn test_user_delete_params() {
        let params = UserDeleteParams::new(UserId(987));
        assert_expected_query_pairs(params, "force=true&reassign=987");
    }

    #[rstest]
    #[case(UserCapability::Import, "import")]
    #[case(UserCapability::CreateSites, "create_sites")]
    #[case(UserCapability::ManageNetworkUsers, "manage_network_users")]
    #[case(UserCapability::Custom("do_the_thing".to_string()), "do_the_thing")]
    fn test_user_capability(#[case] capability: UserCapability, #[case] expected_str: &str) {
        assert_eq!(capability.to_string(), expected_str);
    }

    #[rstest]
    #[case(UserCapability::Import, "import")]
    #[case(UserCapability::CreateSites, "create_sites")]
    #[case(UserCapability::ManageNetworkUsers, "manage_network_users")]
    #[case(UserCapability::Custom("do_the_thing".to_string()), "do_the_thing")]
    fn test_user_capability_from_str(
        #[case] capability: UserCapability,
        #[case] expected_str: &str,
    ) {
        assert_eq!(UserCapability::from_str(expected_str), Ok(capability));
    }

    #[test]
    fn test_user_capabilities_map_with_non_boolean_values() {
        let json = r#"{
            "edit_posts": true,
            "edit_users": false,
            "vc_access_rules_post_types": "custom"
        }"#;
        let caps: UserCapabilitiesMap = serde_json::from_str(json)
            .expect("Failed to deserialize capabilities with non-boolean values");
        assert_eq!(caps.map.len(), 3);
    }

    #[rstest]
    #[case(UserRole::SuperAdmin, "super_admin")]
    #[case(UserRole::Administrator, "administrator")]
    #[case(UserRole::Editor, "editor")]
    #[case(UserRole::Author, "author")]
    #[case(UserRole::Contributor, "contributor")]
    #[case(UserRole::Subscriber, "subscriber")]
    #[case(UserRole::Custom("custom".to_string()), "custom")]
    fn test_user_role(#[case] role: UserRole, #[case] expected_str: &str) {
        assert_eq!(role.to_string(), expected_str);
    }

    #[rstest]
    #[case(UserRole::SuperAdmin, "super_admin")]
    #[case(UserRole::Administrator, "administrator")]
    #[case(UserRole::Editor, "editor")]
    #[case(UserRole::Author, "author")]
    #[case(UserRole::Contributor, "contributor")]
    #[case(UserRole::Subscriber, "subscriber")]
    #[case(UserRole::Custom("custom".to_string()), "custom")]
    fn test_user_role_from_str(#[case] role: UserRole, #[case] expected_str: &str) {
        assert_eq!(UserRole::from_str(expected_str), Ok(role));
    }
}
