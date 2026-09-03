use crate::{
    UserId, WpApiParamOrder,
    date::{WpDateString, WpGmtDateTime},
    impl_as_query_value_from_to_string,
    posts::{
        PostCommentStatus, PostId, PostPingStatus, PostStatus, WpApiParamPostsOrderBy,
        WpApiParamPostsSearchColumn,
    },
    request::{MultipartFormFile, RequiresMultipartForm},
    url_query::{
        AppendUrlQueryPairs, FromUrlQueryPairs, QueryPairs, QueryPairsExtension, UrlQueryPairsMap,
    },
    wp_content_i64_id,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::collections::HashMap;
use std::sync::Arc;
use wp_contextual::WpContextual;
use wp_derive::WpDeriveParamsField;

wp_content_i64_id!(MediaId);

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
pub enum MediaType {
    File,
    Image,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

impl_as_query_value_from_to_string!(MediaType);

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Object)]
#[serde(transparent)]
pub struct MediaTypeWrapper {
    inner: MediaType,
}

impl PartialEq for MediaTypeWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.inner.to_string() == other.inner.to_string()
    }
}

impl Eq for MediaTypeWrapper {}

impl std::fmt::Display for MediaTypeWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl std::str::FromStr for MediaTypeWrapper {
    type Err = <MediaType as std::str::FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        MediaType::from_str(s).map(|inner| Self { inner })
    }
}

impl MediaTypeWrapper {
    pub fn inner(&self) -> &MediaType {
        &self.inner
    }
}

impl From<MediaType> for MediaTypeWrapper {
    fn from(inner: MediaType) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl MediaTypeWrapper {
    fn is_media_type(&self, media_type: MediaType) -> bool {
        self.inner.to_string() == media_type.to_string()
    }
}

// A separate param type is implemented for `MediaType` because the API will accept types such as
// "audio" & "application" as a parameter, but it'll return "file" for the value of `media_type`
// field for these media types.
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
pub enum MediaTypeParam {
    Image,
    Video,
    Text,
    Application,
    Audio,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

impl_as_query_value_from_to_string!(MediaTypeParam);

#[derive(
    Debug,
    Default,
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
pub enum MediaStatus {
    #[default]
    Inherit,
    Private,
    Trash,
    #[serde(untagged)]
    #[strum(default)]
    Custom(String),
}

impl_as_query_value_from_to_string!(MediaStatus);

#[derive(Debug, Default, PartialEq, Eq, uniffi::Record, WpDeriveParamsField)]
#[supports_pagination(true)]
pub struct MediaListParams {
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
    /// Limit response to posts published after a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub after: Option<WpGmtDateTime>,
    /// Limit response to posts modified after a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub modified_after: Option<WpGmtDateTime>,
    /// Limit result set to posts assigned to specific authors.
    #[uniffi(default = [])]
    pub author: Vec<UserId>,
    /// Ensure result set excludes posts assigned to specific authors.
    #[uniffi(default = [])]
    pub author_exclude: Vec<UserId>,
    /// Limit response to posts published before a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub before: Option<WpGmtDateTime>,
    /// Limit response to posts modified before a given ISO8601 compliant date.
    #[uniffi(default = None)]
    pub modified_before: Option<WpGmtDateTime>,
    /// Ensure result set excludes specific IDs.
    #[uniffi(default = [])]
    pub exclude: Vec<MediaId>,
    /// Limit result set to specific IDs.
    #[uniffi(default = [])]
    pub include: Vec<MediaId>,
    /// Offset the result set by a specific number of items.
    #[uniffi(default = None)]
    pub offset: Option<u32>,
    /// Order sort attribute ascending or descending.
    /// Default: desc
    /// One of: asc, desc
    #[uniffi(default = None)]
    pub order: Option<WpApiParamOrder>,
    /// Sort collection by post attribute.
    /// Default: date
    /// One of: author, date, id, include, modified, parent, relevance, slug, include_slugs, title
    #[uniffi(default = None)]
    #[field_name("orderby")]
    pub orderby: Option<WpApiParamPostsOrderBy>,
    /// Limit result set to items with particular parent IDs.
    #[uniffi(default = [])]
    pub parent: Vec<PostId>,
    /// Limit result set to all items except those of a particular parent ID.
    #[uniffi(default = [])]
    pub parent_exclude: Vec<PostId>,
    /// Array of column names to be searched.
    #[uniffi(default = [])]
    pub search_columns: Vec<WpApiParamPostsSearchColumn>,
    /// Limit result set to posts with one or more specific slugs.
    #[uniffi(default = [])]
    pub slug: Vec<String>,
    /// Limit result set to posts assigned one or more statuses.
    /// Default: inherit
    #[uniffi(default = [])]
    pub status: Vec<MediaStatus>,
    /// Limit result set to attachments of a particular media type.
    /// One of: image, video, text, application, audio
    #[uniffi(default = None)]
    pub media_type: Option<MediaTypeParam>,
    /// Limit result set to attachments of a particular MIME type.
    #[uniffi(default = None)]
    pub mime_type: Option<String>,
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct MediaUpdateParams {
    /// The date the post was published, in the site's timezone.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<WpDateString>,
    /// The date the post was published, as GMT.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_gmt: Option<WpGmtDateTime>,
    /// An alphanumeric identifier for the post unique to its type.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// A named status for the post.
    /// One of: publish, future, draft, pending, private
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<PostStatus>,
    /// The title for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The ID for the author of the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<UserId>,
    /// Whether or not comments are open on the post.
    /// One of: open, closed
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_status: Option<PostCommentStatus>,
    /// Whether or not the post can be pinged.
    /// One of: open, closed
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ping_status: Option<PostPingStatus>,
    /// The theme file to use to display the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// Alternative text to display when attachment is not displayed.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_text: Option<String>,
    /// The attachment caption.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    /// The attachment description.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The ID for the associated post of the attachment.
    #[serde(rename = "post")]
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_id: Option<PostId>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/381
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct MediaDeleteResponse {
    pub deleted: bool,
    pub previous: MediaWithEditContext,
}

#[derive(Debug, Default, Serialize, uniffi::Record)]
pub struct MediaCreateParams {
    /// The date the post was published, in the site's timezone.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<WpDateString>,
    /// The date the post was published, as GMT.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_gmt: Option<WpGmtDateTime>,
    /// An alphanumeric identifier for the post unique to its type.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// A named status for the post.
    /// One of: publish, future, draft, pending, private
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<PostStatus>,
    /// The title for the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The ID for the author of the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<UserId>,
    /// Whether or not comments are open on the post.
    /// One of: open, closed
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_status: Option<PostCommentStatus>,
    /// Whether or not the post can be pinged.
    /// One of: open, closed
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ping_status: Option<PostPingStatus>,
    /// The theme file to use to display the post.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// Alternative text to display when attachment is not displayed.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_text: Option<String>,
    /// The attachment caption.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    /// The attachment description.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The ID for the associated post of the attachment.
    #[serde(rename = "post")]
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_id: Option<PostId>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/381
    #[serde(skip)]
    pub file_path: String,
}

impl RequiresMultipartForm for MediaCreateParams {
    fn multipart_form_files(&self) -> HashMap<String, MultipartFormFile> {
        let mut files = HashMap::new();
        files.insert(
            "file".to_string(),
            MultipartFormFile {
                file_path: self.file_path.clone(),
                mime_type: None,
                file_name: None,
            },
        );
        files
    }
}

impl From<MediaCreateParams> for HashMap<String, String> {
    fn from(params: MediaCreateParams) -> Self {
        let mut map = HashMap::new();
        let mut add = |k: &str, v: Option<String>| {
            if let Some(v) = v {
                map.insert(k.to_string(), v);
            }
        };
        add("date", params.date.map(|d| d.to_string()));
        add("date_gmt", params.date_gmt.map(|d| d.to_string()));
        add("slug", params.slug);
        add("status", params.status.map(|x| x.to_string()));
        add("title", params.title);
        add("author", params.author.map(|x| x.to_string()));
        add(
            "comment_status",
            params.comment_status.map(|x| x.to_string()),
        );
        add("ping_status", params.ping_status.map(|x| x.to_string()));
        add("template", params.template);
        add("alt_text", params.alt_text);
        add("caption", params.caption);
        add("description", params.description);
        add("post", params.post_id.map(|x| x.to_string()));
        map
    }
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseMedia {
    #[WpContext(edit, embed, view)]
    pub id: Option<MediaId>,
    #[WpContext(edit, embed, view)]
    pub date: Option<WpDateString>,
    #[WpContext(edit, view)]
    pub date_gmt: Option<WpGmtDateTime>,
    #[WpContext(edit, view)]
    #[WpContextualField]
    pub guid: Option<crate::posts::SparsePostGuid>,
    #[WpContext(edit, embed, view)]
    pub link: Option<String>,
    #[WpContext(edit, view)]
    pub modified: Option<WpDateString>,
    #[WpContext(edit, view)]
    pub modified_gmt: Option<WpGmtDateTime>,
    #[WpContext(edit, embed, view)]
    pub slug: Option<String>,
    #[WpContext(edit, view)]
    pub status: Option<MediaStatus>,
    #[serde(rename = "type")]
    #[WpContext(edit, embed, view)]
    pub post_type: Option<String>,
    #[WpContext(edit)]
    #[WpContextualOption]
    pub password: Option<String>,
    #[WpContext(edit)]
    pub permalink_template: Option<String>,
    #[WpContext(edit)]
    pub generated_slug: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualField]
    pub title: Option<crate::posts::SparsePostTitle>,
    #[WpContext(edit, embed, view)]
    pub author: Option<crate::UserId>,
    #[WpContext(edit, view)]
    pub comment_status: Option<PostCommentStatus>,
    #[WpContext(edit, view)]
    pub ping_status: Option<PostPingStatus>,
    #[WpContext(edit, view)]
    pub template: Option<String>,
    #[WpContext(edit, embed, view)]
    pub alt_text: Option<String>,
    #[WpContext(edit, embed, view)]
    #[WpContextualField]
    pub caption: Option<SparseMediaCaption>,
    #[WpContext(edit, view)]
    #[WpContextualField]
    pub description: Option<SparseMediaDescription>,
    #[WpContext(edit, embed, view)]
    pub media_type: Option<Arc<MediaTypeWrapper>>,
    #[WpContext(edit, embed, view)]
    pub mime_type: Option<String>,
    #[WpContext(edit, embed, view)]
    pub media_details: Option<Arc<MediaDetails>>,
    #[serde(rename = "post")]
    #[WpContext(edit, view)]
    #[WpContextualOption]
    pub post_id: Option<PostId>,
    #[WpContext(edit, embed, view)]
    pub source_url: Option<String>,
    #[WpContext(edit)]
    pub missing_image_sizes: Option<Vec<String>>,
    // meta field is omitted for now: https://github.com/Automattic/wordpress-rs/issues/381
}

#[derive(Debug, Serialize, Deserialize, uniffi::Object)]
#[serde(transparent)]
#[uniffi::export(Eq, Hash)]
pub struct MediaDetails {
    pub payload: Box<RawValue>,
}

#[uniffi::export]
impl MediaDetails {
    pub fn parse_as_mime_type(&self, mime_type: String) -> Option<MediaDetailsPayload> {
        if mime_type.starts_with("image/") {
            return serde_json::from_str::<ImageMediaDetails>(self.payload.get())
                .ok()
                .map(MediaDetailsPayload::Image);
        } else if mime_type.starts_with("video/") || mime_type.starts_with("audio/") {
            // Some audio files may be returned with a video MIME type, so we attempt to parse both.

            if let Ok(details) = serde_json::from_str::<VideoMediaDetails>(self.payload.get()) {
                return Some(MediaDetailsPayload::Video(details));
            }

            if let Ok(details) = serde_json::from_str::<AudioMediaDetails>(self.payload.get()) {
                return Some(MediaDetailsPayload::Audio(details));
            }
        }

        serde_json::from_str::<DocumentMediaDetails>(self.payload.get())
            .ok()
            .map(MediaDetailsPayload::Document)
    }
}

impl PartialEq for MediaDetails {
    fn eq(&self, other: &Self) -> bool {
        self.payload.get() == other.payload.get()
    }
}

impl Eq for MediaDetails {}

impl std::hash::Hash for MediaDetails {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.payload.get().hash(state);
    }
}

#[derive(Debug, uniffi::Enum)]
pub enum MediaDetailsPayload {
    Audio(AudioMediaDetails),
    Image(ImageMediaDetails),
    Video(VideoMediaDetails),
    Document(DocumentMediaDetails),
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct AudioMediaDetails {
    #[serde(rename = "filesize")]
    pub file_size: u64,
    pub length: u64,
    pub length_formatted: String,

    #[serde(rename = "dataformat")]
    pub data_format: Option<String>,
    pub codec: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub bits_per_sample: Option<u32>,
    pub lossless: Option<bool>,
    #[serde(rename = "channelmode")]
    pub channel_mode: Option<String>,
    pub bitrate: Option<f64>,
    pub compression_ratio: Option<f64>,
    #[serde(rename = "fileformat")]
    pub file_format: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct ImageMediaDetails {
    #[serde(rename = "filesize")]
    pub file_size: u64,

    pub width: u32,
    pub height: u32,
    pub file: String,
    pub sizes: Option<HashMap<String, ScaledImageDetails>>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct ScaledImageDetails {
    pub file: String,
    pub width: u32,
    pub height: u32,
    pub source_url: String,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct VideoMediaDetails {
    #[serde(rename = "filesize")]
    pub file_size: u64,
    pub length: u32,
    pub width: u32,
    pub height: u32,

    #[serde(rename = "fileformat")]
    pub file_format: Option<String>,
    #[serde(rename = "dataformat")]
    pub data_format: Option<String>,
    pub created_timestamp: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct DocumentMediaDetails {
    #[serde(rename = "filesize")]
    pub file_size: u64,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseMediaDescription {
    #[WpContext(edit)]
    pub raw: Option<String>,
    #[WpContext(edit, view)]
    pub rendered: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, uniffi::Record, WpContextual)]
pub struct SparseMediaCaption {
    #[WpContext(edit)]
    pub raw: Option<String>,
    #[WpContext(edit, embed, view)]
    pub rendered: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        SparseField, generate,
        posts::PostId,
        unit_test_common::{
            assert_expected_and_from_query_pairs, unit_test_example_date_as_option,
            unit_test_example_date_as_query_value,
        },
    };
    use rstest::*;

    #[rstest]
    #[case(MediaListParams::default(), "")]
    #[case(generate!(MediaListParams, (page, Some(2))), "page=2")]
    #[case(generate!(MediaListParams, (per_page, Some(2))), "per_page=2")]
    #[case(generate!(MediaListParams, (search, Some("foo".to_string()))), "search=foo")]
    #[case(generate!(MediaListParams, (after, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("after"))]
    #[case(generate!(MediaListParams, (modified_after, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("modified_after"))]
    #[case(generate!(MediaListParams, (author, vec![UserId(1), UserId(2)])), "author=1%2C2")]
    #[case(generate!(MediaListParams, (author_exclude, vec![UserId(1), UserId(2)])), "author_exclude=1%2C2")]
    #[case(generate!(MediaListParams, (before, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("before"))]
    #[case(generate!(MediaListParams, (modified_before, unit_test_example_date_as_option())), &unit_test_example_date_as_query_value("modified_before"))]
    #[case(generate!(MediaListParams, (exclude, vec![MediaId(1), MediaId(2)])), "exclude=1%2C2")]
    #[case(generate!(MediaListParams, (include, vec![MediaId(1), MediaId(2)])), "include=1%2C2")]
    #[case(generate!(MediaListParams, (offset, Some(2))), "offset=2")]
    #[case(generate!(MediaListParams, (order, Some(WpApiParamOrder::Asc))), "order=asc")]
    #[case(generate!(MediaListParams, (order, Some(WpApiParamOrder::Desc))), "order=desc")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Author))), "orderby=author")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Date))), "orderby=date")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Id))), "orderby=id")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Include))), "orderby=include")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::IncludeSlugs))), "orderby=include_slugs")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Modified))), "orderby=modified")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Parent))), "orderby=parent")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Relevance))), "orderby=relevance")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Slug))), "orderby=slug")]
    #[case(generate!(MediaListParams, (orderby, Some(WpApiParamPostsOrderBy::Title))), "orderby=title")]
    #[case(generate!(MediaListParams, (parent, vec![PostId(44444), PostId(44445)])), "parent=44444%2C44445")]
    #[case(generate!(MediaListParams, (parent_exclude, vec![PostId(55555), PostId(55556)])), "parent_exclude=55555%2C55556")]
    #[case(generate!(MediaListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostContent])), "search_columns=post_content")]
    #[case(generate!(MediaListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostExcerpt])), "search_columns=post_excerpt")]
    #[case(generate!(MediaListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostTitle])), "search_columns=post_title")]
    #[case(generate!(MediaListParams, (search_columns, vec![WpApiParamPostsSearchColumn::PostContent, WpApiParamPostsSearchColumn::PostExcerpt, WpApiParamPostsSearchColumn::PostTitle])), "search_columns=post_content%2Cpost_excerpt%2Cpost_title")]
    #[case(generate!(MediaListParams, (slug, vec!["foo".to_string(), "bar".to_string()])), "slug=foo%2Cbar")]
    #[case(generate!(MediaListParams, (status, vec![MediaStatus::Inherit])), "status=inherit")]
    #[case(generate!(MediaListParams, (status, vec![MediaStatus::Private])), "status=private")]
    #[case(generate!(MediaListParams, (status, vec![MediaStatus::Trash])), "status=trash")]
    #[case(generate!(MediaListParams, (status, vec![MediaStatus::Custom("foo".to_string())])), "status=foo")]
    #[case(generate!(MediaListParams, (status, vec![MediaStatus::Inherit, MediaStatus::Private, MediaStatus::Trash, MediaStatus::Custom("foo".to_string())])), "status=inherit%2Cprivate%2Ctrash%2Cfoo")]
    #[case(generate!(MediaListParams, (media_type, Some(MediaTypeParam::Image))), "media_type=image")]
    #[case(generate!(MediaListParams, (mime_type, Some("image/jpeg".to_string()))), "mime_type=image%2Fjpeg")]
    #[case(MediaListParams {
            page: Some(11),
            per_page: Some(22),
            search: Some("s_q".to_string()),
            after: unit_test_example_date_as_option(),
            modified_after: unit_test_example_date_as_option(),
            author: vec![UserId(111), UserId(112)],
            author_exclude: vec![UserId(211), UserId(212)],
            before: unit_test_example_date_as_option(),
            modified_before: unit_test_example_date_as_option(),
            exclude: vec![MediaId(1111), MediaId(1112)],
            include: vec![MediaId(2111), MediaId(2112)],
            offset: Some(11111),
            order: Some(WpApiParamOrder::Desc),
            orderby: Some(WpApiParamPostsOrderBy::Slug),
            parent: vec![PostId(44444), PostId(44445)],
            parent_exclude: vec![PostId(55555), PostId(55556)],
            search_columns: vec![
                WpApiParamPostsSearchColumn::PostContent,
                WpApiParamPostsSearchColumn::PostExcerpt,
            ],
            slug: vec!["sl_1".to_string(), "sl_2".to_string()],
            status: vec![MediaStatus::Inherit, MediaStatus::Private, MediaStatus::Trash],
            media_type: Some(MediaTypeParam::Image),
            mime_type: Some("image/jpeg".to_string()),
        },
        &expected_query_pairs_for_media_list_params_with_all_fields()
    )]
    #[trace]
    fn test_media_list_query_pairs(#[case] params: MediaListParams, #[case] expected_query: &str) {
        assert_expected_and_from_query_pairs(params, expected_query);
    }

    fn expected_query_pairs_for_media_list_params_with_all_fields() -> String {
        let after = unit_test_example_date_as_query_value("after");
        let modified_after = unit_test_example_date_as_query_value("modified_after");
        let before = unit_test_example_date_as_query_value("before");
        let modified_before = unit_test_example_date_as_query_value("modified_before");
        format!(
            "page=11&per_page=22&search=s_q&{after}&{modified_after}&author=111%2C112&author_exclude=211%2C212&{before}&{modified_before}&exclude=1111%2C1112&include=2111%2C2112&offset=11111&order=desc&orderby=slug&parent=44444%2C44445&parent_exclude=55555%2C55556&search_columns=post_content%2Cpost_excerpt&slug=sl_1%2Csl_2&status=inherit%2Cprivate%2Ctrash&media_type=image&mime_type=image%2Fjpeg"
        )
    }

    #[test]
    fn media_details_round_trip() {
        let original = r#"
        {
            "id": 11,
            "date": "2025-05-29T03:15:55",
            "date_gmt": "2025-05-29T03:15:55",
            "guid": { "rendered": "https://example.com/dummy.docx" },
            "modified": "2025-05-29T03:15:55",
            "modified_gmt": "2025-05-29T03:15:55",
            "slug": "dummy-slug",
            "status": "dummy-status",
            "type": "dummy-type",
            "link": "https://example.com/dummy-link/",
            "title": { "rendered": "Dummy Title" },
            "author": 1,
            "featured_media": 0,
            "comment_status": "dummy-comment-status",
            "ping_status": "dummy-ping-status",
            "template": "dummy-template",
            "meta": [],
            "class_list": [
                "dummy-class-1",
                "dummy-class-2",
                "dummy-class-3",
                "dummy-class-4",
                "dummy-class-5"
            ],
            "description": {
                "rendered": "<p class=\"dummy-class\"><a href='https://example.com/dummy.docx'>Dummy Link</a></p>\n"
            },
            "caption": {
                "rendered": "<p>Dummy Caption</p>\n"
            },
            "alt_text": "Dummy Alt Text",
            "media_type": "dummy-media-type",
            "mime_type": "dummy/mime-type",
            "media_details": {
                "filesize": 7378,
                "sizes": {}
            },
            "post": null,
            "source_url": "https://example.com/dummy.docx"
            }
        "#;

        let media = serde_json::from_str::<MediaWithViewContext>(original).unwrap();
        let serialized = serde_json::to_string(&media).unwrap();
        let json = serde_json::from_str::<serde_json::Value>(serialized.as_str()).unwrap();
        assert_eq!(json["media_details"]["filesize"], 7378);
        assert_eq!(
            json["media_details"]["sizes"],
            serde_json::Value::Object(serde_json::Map::new())
        );
    }

    #[rstest]
    #[case(SparseMediaFieldWithEditContext::Id, "id")]
    #[case(SparseMediaFieldWithEditContext::PostId, "post")]
    #[case(SparseMediaFieldWithEditContext::PostType, "type")]
    fn test_as_mapped_field_name_for_edit_context(
        #[case] field: SparseMediaFieldWithEditContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }

    #[rstest]
    #[case(SparseMediaFieldWithEmbedContext::Id, "id")]
    #[case(SparseMediaFieldWithEmbedContext::PostType, "type")]
    fn test_as_mapped_field_name_for_embed_context(
        #[case] field: SparseMediaFieldWithEmbedContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }

    #[rstest]
    #[case(SparseMediaFieldWithViewContext::Id, "id")]
    #[case(SparseMediaFieldWithViewContext::PostId, "post")]
    #[case(SparseMediaFieldWithViewContext::PostType, "type")]
    fn test_as_mapped_field_name_for_view_context(
        #[case] field: SparseMediaFieldWithViewContext,
        #[case] expected_mapped_field_name: &str,
    ) {
        assert_eq!(field.as_mapped_field_name(), expected_mapped_field_name);
    }
}
