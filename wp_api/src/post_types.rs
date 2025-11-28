use crate::{JsonValue, impl_as_query_value_from_to_string};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use wp_contextual::WpContextual;
use wp_serde_helper::deserialize_empty_array_or_hashmap;

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
pub enum PostType {
    Post,
    Page,
    Attachment,
    NavMenuItem,
    WpBlock,
    WpTemplate,
    WpTemplatePart,
    WpNavigation,
    WpFontFamily,
    WpFontFace,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

impl_as_query_value_from_to_string!(PostType);

#[uniffi::export]
fn post_type_from_string(value: String) -> PostType {
    PostType::from_str(value.as_str()).unwrap_or(PostType::Custom(value))
}

#[uniffi::export]
fn post_type_to_string(post_type: PostType) -> String {
    post_type.to_string()
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
#[serde(transparent)]
pub struct SparsePostTypesResponse {
    #[serde(flatten)]
    #[WpContext(edit, embed, view)]
    #[WpContextualField]
    pub post_types: Option<HashMap<PostType, SparsePostTypeDetails>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePostTypeDetails {
    #[WpContext(edit)]
    pub capabilities: Option<HashMap<PostTypeCapabilities, String>>,
    #[WpContext(edit, view)]
    pub description: Option<String>,
    #[WpContext(edit, view)]
    pub hierarchical: Option<bool>,
    #[WpContext(edit)]
    pub viewable: Option<bool>,
    #[WpContext(edit)]
    pub labels: Option<PostTypeLabels>,
    #[WpContext(edit, embed, view)]
    pub name: Option<String>,
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WpContext(edit)]
    pub supports: Option<PostTypeSupportsMap>,
    #[WpContext(edit, view)]
    pub has_archive: Option<bool>,
    #[WpContext(edit, view)]
    pub taxonomies: Option<Vec<String>>,
    #[WpContext(edit, embed, view)]
    pub rest_base: Option<String>,
    #[WpContext(edit, embed, view)]
    pub rest_namespace: Option<String>,
    #[WpContext(edit)]
    pub visibility: Option<PostTypeVisibility>,
    #[WpContext(edit, embed, view)]
    #[WpContextualOption]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, uniffi::Record)]
#[serde(transparent)]
pub struct PostTypeSupportsMap {
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    #[serde(flatten)]
    #[serde(rename = "supports")]
    pub map: HashMap<PostTypeSupports, Arc<PostTypeSupportsValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, uniffi::Object)]
#[serde(transparent)]
pub struct PostTypeSupportsValue {
    #[serde(flatten)]
    pub json_value: JsonValue,
}

#[uniffi::export]
impl PostTypeSupportsValue {
    fn as_json_value(&self) -> JsonValue {
        self.json_value.clone()
    }

    pub fn as_json_bool(&self) -> Option<bool> {
        match self.json_value {
            JsonValue::Bool(b) => Some(b),
            _ => None,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct PostTypeLabels {
    pub name: String,
    pub singular_name: String,
    pub add_new: String,
    pub add_new_item: String,
    pub edit_item: String,
    pub new_item: String,
    pub view_item: String,
    pub view_items: String,
    pub search_items: String,
    pub not_found: String,
    pub not_found_in_trash: String,
    pub parent_item_colon: Option<String>,
    pub all_items: String,
    pub archives: String,
    pub attributes: String,
    pub insert_into_item: String,
    pub uploaded_to_this_item: String,
    pub featured_image: String,
    pub set_featured_image: String,
    pub remove_featured_image: String,
    pub use_featured_image: String,
    pub filter_items_list: String,
    pub filter_by_date: String,
    pub items_list_navigation: String,
    pub items_list: String,
    pub item_published: String,
    pub item_published_privately: String,
    pub item_reverted_to_draft: String,
    pub item_trashed: String,
    pub item_scheduled: String,
    pub item_updated: String,
    pub item_link: String,
    pub item_link_description: String,
    pub menu_name: String,
    pub name_admin_bar: String,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum PostTypeCapabilities {
    CreatePosts,
    DeleteOthersPosts,
    DeletePost,
    DeletePosts,
    DeletePrivatePosts,
    DeletePublishedPosts,
    EditOthersPosts,
    EditPost,
    EditPosts,
    EditPrivatePosts,
    EditPublishedPosts,
    PublishPosts,
    Read,
    ReadPost,
    ReadPrivatePosts,
    #[serde(untagged)]
    Custom(String),
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "kebab-case")]
pub enum PostTypeSupports {
    Author,
    Comments,
    CustomFields,
    Editor,
    Excerpt,
    PageAttributes,
    PostFormats,
    Revisions,
    Slug,
    Thumbnail,
    Title,
    Trackbacks,
    #[serde(untagged)]
    Custom(String),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct PostTypeVisibility {
    pub show_in_nav_menus: bool,
    pub show_ui: bool,
}
