use std::fmt::Display;

use serde::{Deserialize, Serialize};
use wp_contextual::WpContextual;

use crate::SparseField;

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
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
}

impl Display for PostType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Post => "post",
            Self::Page => "page",
            Self::Attachment => "attachment",
            Self::NavMenuItem => "nav_menu_item",
            Self::WpBlock => "wp_block",
            Self::WpTemplate => "wp_template",
            Self::WpTemplatePart => "wp_template_part",
            Self::WpNavigation => "wp_navigation",
            Self::WpFontFamily => "wp_font_family",
            Self::WpFontFace => "wp_font_face",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePostTypeResponse {
    #[WpContext(edit, embed, view)]
    pub post: SparsePostTypeDetails,
    #[WpContext(edit, embed, view)]
    pub page: SparsePostTypeDetails,
    #[WpContext(edit, embed, view)]
    pub attachment: SparsePostTypeDetails,
    #[WpContext(edit, embed, view)]
    pub nav_menu_item: SparsePostTypeDetails,
    #[WpContext(edit, embed, view)]
    pub wp_block: SparsePostTypeDetails,
    #[WpContext(edit, embed, view)]
    pub wp_template: SparsePostTypeDetails,
    #[WpContext(edit, embed, view)]
    pub wp_template_part: SparsePostTypeDetails,
    #[WpContext(edit, embed, view)]
    pub wp_navigation: SparsePostTypeDetails,
    #[WpContext(edit, embed, view)]
    pub wp_font_family: SparsePostTypeDetails,
    #[WpContext(edit, embed, view)]
    pub wp_font_face: SparsePostTypeDetails,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparsePostTypeDetails {
    #[WpContext(edit)]
    pub capabilities: Capabilities,
    #[WpContext(edit, view)]
    pub description: Option<String>,
    #[WpContext(edit, view)]
    pub hierarchical: Option<bool>,
    #[WpContext(edit, view)]
    pub viewable: Option<bool>,
    #[WpContext(edit)]
    pub labels: Labels,
    #[WpContext(edit, embed, view)]
    pub name: String,
    #[WpContext(edit, embed, view)]
    pub slug: String,
    #[WpContext(edit, embed, view)]
    pub supports: Supports,
    #[WpContext(edit, view)]
    pub has_archive: bool,
    #[WpContext(edit, view)]
    pub taxonomies: Vec<String>,
    #[WpContext(edit, embed, view)]
    pub rest_base: String,
    #[WpContext(edit, embed, view)]
    pub rest_namespace: String,
    #[WpContext(edit)]
    pub visibility: PostTypeVisibility,
    #[WpContext(edit, embed, view)]
    pub icon: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct Capabilities {
    pub edit_post: String,
    pub read_post: String,
    pub delete_post: String,
    pub edit_posts: String,
    pub edit_others_posts: String,
    pub delete_posts: String,
    pub publish_posts: String,
    pub read_private_posts: String,
    pub read: String,
    pub delete_private_posts: String,
    pub delete_published_posts: String,
    pub delete_others_posts: String,
    pub edit_private_posts: String,
    pub edit_published_posts: String,
    pub create_posts: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct Labels {
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
    pub parent_item_colon: String,
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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct Supports {
    pub title: bool,
    pub editor: bool,
    pub author: bool,
    pub thumbnail: bool,
    pub excerpt: bool,
    pub trackbacks: bool,
    #[serde(rename = "custom-fields")]
    pub custom_fields: bool,
    pub comments: bool,
    pub revisions: bool,
    #[serde(rename = "post-formats")]
    pub post_formats: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct PostTypeVisibility {
    pub show_in_nav_menus: bool,
    pub show_ui: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SparsePostTypeDetailsField {
    Capabilities,
    Description,
    Hierarchical,
    Viewable,
    Labels,
    Name,
    Slug,
    Supports,
    HasArchive,
    Taxonomies,
    RestBase,
    RestNamespace,
    Visibility,
    Icon,
}

impl SparseField for SparsePostTypeDetailsField {
    fn as_str(&self) -> &str {
        match self {
            Self::Capabilities => "capabilities",
            Self::Description => "description",
            Self::Hierarchical => "hierarchical",
            Self::Viewable => "viewable",
            Self::Labels => "labels",
            Self::Name => "name",
            Self::Slug => "slug",
            Self::Supports => "supports",
            Self::HasArchive => "has_archive",
            Self::Taxonomies => "taxonomies",
            Self::RestBase => "rest_base",
            Self::RestNamespace => "rest_namespace",
            Self::Visibility => "visibility",
            Self::Icon => "icon",
        }
    }
}
