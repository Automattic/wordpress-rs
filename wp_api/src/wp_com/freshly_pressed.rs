use std::collections::HashMap;

use crate::{
    date::WpGmtDateTime, posts::PostId, url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    }, users::UserId, wp_com::WpComSiteId, JsonValue
};
use serde::{Deserialize, Serialize};
use strum_macros::IntoStaticStr;
use wp_serde_helper::{deserialize_false_or_string, deserialize_u64_or_none,deserialize_empty_array_or_hashmap};

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct FreshlyPressedPostList {
    pub date_range: FreshlyPressedDateRange,
    pub number: u32,
    pub posts: Vec<FreshlyPressedPost>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct FreshlyPressedDateRange {
    pub before: WpGmtDateTime,
    pub after: WpGmtDateTime,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct FreshlyPressedPost {
    #[serde(rename = "ID")]
    pub id: PostId,
    #[serde(alias = "site_ID")]
    pub site_id: WpComSiteId,
    pub author: FreshlyPressedAuthor,
    pub date: String,
    pub modified: String,
    pub title: String,
    #[serde(rename = "URL")]
    pub url: String,
    #[serde(rename = "short_URL")]
    pub short_url: String,
    pub content: String,
    pub excerpt: String,
    pub slug: String,
    pub guid: String,
    pub status: String,
    pub sticky: bool,
    pub password: String,
    #[serde(deserialize_with = "deserialize_u64_or_none")]
    pub parent: Option<u64>,
    pub r#type: String,
    pub likes_enabled: bool,
    pub sharing_enabled: bool,
    pub like_count: u32,
    pub i_like: bool,
    pub is_reblogged: bool,
    pub is_following: bool,
    #[serde(rename = "global_ID")]
    pub global_id: String,
    pub featured_image: String,
    pub post_thumbnail: Option<FreshlyPressedPostThumbnail>,
    pub format: String,
    // pub geo: bool // TODO: need sample data to figure out the shape of this
    pub menu_order: u32,
    pub page_template: String,
    #[serde(rename = "publicize_URLs")]
    pub publicize_urls: Vec<String>,
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    pub terms: HashMap<String, HashMap<String, FreshlyPressedTerm>>,
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    pub tags: HashMap<String, FreshlyPressedTerm>,
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    pub categories: HashMap<String, FreshlyPressedTerm>,
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    pub attachments: HashMap<String, FreshlyPressedAttachment>,
    pub attachment_count: u64,
    pub metadata: Vec<FreshlyPressedKeyValuePair>,
    pub meta: FreshlyPressedObjectMeta,
    pub capabilities: HashMap<String, bool>,
    #[serde(alias = "other_URLs")]
    pub other_urls: HashMap<String, String>,
    #[serde(alias = "pseudo_ID")]
    pub pseudo_id: String,
    pub is_external: bool,
    pub site_name: String,
    #[serde(alias = "site_URL")]
    pub site_url: String,
    pub site_is_private: bool,
    #[serde(deserialize_with = "deserialize_empty_array_or_hashmap")]
    pub site_icon: HashMap<String, String>,
    pub featured_media: HashMap<String, String>,
    #[serde(alias = "feed_ID")]
    pub feed_id: u64,
    #[serde(alias = "feed_URL")]
    pub feed_url: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct FreshlyPressedPostThumbnail {
    #[serde(rename = "ID")]
    pub id: u64,
    #[serde(rename = "URL")]
    pub url: String,
    pub guid: String,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct FreshlyPressedTerm {
    #[serde(rename = "ID")]
    pub id: u64,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub post_count: u32,
    pub parent: Option<u64>,
    pub meta: FreshlyPressedObjectMeta,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct FreshlyPressedObjectMeta {
    pub links: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct FreshlyPressedKeyValuePair{
    pub id: String,
    pub key: String,
    pub value: JsonValue,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct FreshlyPressedAttachment {
    #[serde(alias = "ID")]
    pub id: u64,
    #[serde(alias = "URL")]
    pub url: String,
    pub guid: String,
    pub date: String,
    #[serde(alias = "post_ID")]
    pub post_id: u64,
    #[serde(alias = "author_ID")]
    pub author_id: u64,
    pub file: String,
    pub mime_type: String,
    pub extension: String,
    pub title: String,
    pub caption: String,
    pub description: String,
    pub alt: String,
    pub thumbnails: HashMap<String, String>,
    pub height: u32,
    pub width: u32,
    pub exif: FreshlyPressedAttachmentExifData,
    pub meta: FreshlyPressedObjectMeta,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct FreshlyPressedAttachmentExifData {
    pub aperture: String,
    pub credit: String,
    pub camera: String,
    pub caption: String,
    pub created_timestamp: String,
    pub copyright: String,
    pub focal_length: String,
    pub iso: String,
    pub shutter_speed: String,
    pub title: String,
    pub orientation: String,
    pub keywords: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct FreshlyPressedAuthor {
    #[serde(rename = "ID")]
    pub id: UserId,
    pub login: String,
    #[serde(default, deserialize_with = "deserialize_false_or_string")]
    pub email: Option<String>,
    pub name: String,
    pub first_name: String,
    pub last_name: String,
    pub nice_name: String,
    #[serde(alias = "URL")]
    pub url: String,
    #[serde(alias = "avatar_URL")]
    pub avatar_url: String,
    #[serde(alias = "profile_URL")]
    pub profile_url: String,
    #[serde(alias = "site_ID", deserialize_with = "deserialize_u64_or_none")]
    pub site_id: Option<u64>,
    pub has_avatar: bool,
    pub wpcom_id: u64,
    pub wpcom_login: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct FreshlyPressedDiscussionSettings {
    pub comments_open: bool,
    pub comment_status: String,
    pub pings_open: bool,
    pub ping_status: String,
    pub comment_count: u32,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct FreshlyPressedEditorialSettings {
    pub blog_id: String,
    pub post_id: String,
    pub image: String,
    pub custom_headline: String,
    pub custom_blog_title: String,

    pub displayed_on: WpGmtDateTime,
    pub picked_on: WpGmtDateTime,
    pub highlight_topic: String,
    pub highlight_topic_title: String,
    pub screen_offset: String,
    pub blog_name: String,
    pub site_id: String,
}

#[derive(Debug, Default, Serialize, PartialEq, Eq, uniffi::Record)]
pub struct FreshlyPressedListParams {
    #[uniffi(default = None)]
    pub number: Option<u32>,
    #[uniffi(default = None)]
    pub page: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, IntoStaticStr)]
enum FreshlyPressedListParamsField {
    #[strum(serialize = "number")]
    Number,
    #[strum(serialize = "page")]
    Page,
}

impl AppendUrlQueryPairs for FreshlyPressedListParams {
    fn append_query_pairs(&self, query_pairs_mut: &mut QueryPairs) {
        query_pairs_mut
            .append_option_query_value_pair(FreshlyPressedListParamsField::Number, self.number.as_ref())
            .append_option_query_value_pair(FreshlyPressedListParamsField::Page, self.page.as_ref());
    }
}

impl FromUrlQueryPairs for FreshlyPressedListParams {
    fn from_url_query_pairs(query_pairs: UrlQueryPairsMap) -> Option<Self> {
        Some(Self {
            number: query_pairs.get(FreshlyPressedListParamsField::Number),
            page: query_pairs.get(FreshlyPressedListParamsField::Page),
        })
    }

    fn supports_pagination() -> bool {
        true
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freshly_pressed_post_deserialization() {
        let json = include_str!("../../tests/wpcom/freshly_pressed/post-list-1.json");
        let conversation: FreshlyPressedPostList =
            serde_json::from_str(json).expect("Failed to deserialize freshly pressed post list");
        assert_eq!(conversation.number, 10);
        assert_eq!(conversation.posts[0].id, crate::posts::PostId(16283));
        assert_eq!(conversation.posts[0].site_id, crate::wp_com::WpComSiteId(121838035));
        assert_eq!(conversation.posts[0].author.id, crate::users::UserId(1));
    }
}
