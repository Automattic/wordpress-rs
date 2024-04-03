use serde::{Deserialize, Serialize};
use wp_derive::WPContextual;

#[derive(Default, uniffi::Record)]
pub struct UserListParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

impl UserListParams {
    pub fn query_pairs(&self) -> impl IntoIterator<Item = (&str, String)> {
        [
            self.page.as_ref().map(|p| ("page", p.to_string())),
            self.per_page.as_ref().map(|p| ("per_page", p.to_string())),
        ]
        .into_iter()
        .flatten()
    }
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
