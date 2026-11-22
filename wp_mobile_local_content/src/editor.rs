//! Converting between working copy fields and a post in the REST API's shape,
//! as a block editor holds it and sends it.
//!
//! The editor's post is a JSON object. Each key of it is one of three kinds:
//!
//! - A key with a working copy field, such as `title`, `status` or `date`,
//!   converts into that field. `meta` goes into `meta`.
//! - A key `wp_api` models but the working copy has no field for, such as
//!   `id`, `modified`, `link` or `template`, is left out. The server sets most
//!   of them, and the others stay as they are on the server.
//! - Any other key, such as a field a plugin adds, goes into
//!   `additional_fields` as it is, except keys starting with `_`, such as
//!   `_links`, which describe a response rather than the post.

use std::sync::Arc;

use serde_json::{Map, Value};
use wp_api::{
    WpAdditionalFields,
    date::WpGmtDateTime,
    media::MediaId,
    posts::{
        PostCommentStatus, PostCreateParams, PostFormat, PostMeta, PostPingStatus, PostStatus,
        PostUpdateParams,
    },
    terms::TermId,
    users::UserId,
};

use crate::{
    db_types::working_copy::{
        FeaturedImage, PostWorkingCopyFields, PublishDate, StatusSetting, WorkingCopyTag,
    },
    remote::{RemotePostConversionError, status_from_remote},
};

/// The keys `wp_api` models that have no working copy field.
const KEYS_WITHOUT_A_FIELD: [&str; 11] = [
    "id",
    "guid",
    "link",
    "modified",
    "modified_gmt",
    "type",
    "permalink_template",
    "generated_slug",
    "template",
    "parent",
    "menu_order",
];

/// Why an editor's post can't be converted into working copy fields.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum EditorPostConversionError {
    #[error("The editor's post must be a JSON object")]
    NotAnObject,
    #[error("The editor's {field} is invalid: {reason}")]
    InvalidField { field: String, reason: String },
    #[error("A post with status {status} can't be edited")]
    UneditableStatus { status: String },
    #[error(
        "The editor's date is in the site's timezone, and the site's offset from UTC isn't known"
    )]
    UnknownSiteTimezone,
}

impl PostWorkingCopyFields {
    /// Take the whole post as the editor has it.
    ///
    /// The fields whose keys `post` has are replaced, and the others stay as
    /// they are. `meta` and `additional_fields` are replaced as a whole.
    ///
    /// `utc_offset_seconds` is the site's offset from UTC. The editor's `date`
    /// is in the site's timezone, so without the offset it can only be read
    /// from `date_gmt`. An unpublished post whose `date` equals its `modified`
    /// gets `PublishDate::Immediately`, as in `from_remote`.
    pub fn apply_editor_post(
        &mut self,
        post: &Map<String, Value>,
        utc_offset_seconds: Option<i32>,
    ) -> Result<(), EditorPostConversionError> {
        self.apply_editor_keys(post, utc_offset_seconds, Merge::Replace)
    }

    /// Take the fields an editor's write sends: only the ones that changed.
    ///
    /// The same conversion as `apply_editor_post`, except that `meta` and
    /// `additional_fields` are merged key by key, so the keys `edits` doesn't
    /// have keep their values.
    pub fn apply_editor_edits(
        &mut self,
        edits: &Map<String, Value>,
        utc_offset_seconds: Option<i32>,
    ) -> Result<(), EditorPostConversionError> {
        self.apply_editor_keys(edits, utc_offset_seconds, Merge::KeyByKey)
    }

    /// Lay these fields over `post`, a post in the REST API's edit shape, so
    /// it shows what the working copy holds.
    ///
    /// The keys of `post` with no working copy field keep their values, and so
    /// do `rendered` values, which can be out of date afterwards. The meta and
    /// additional fields are laid over key by key. Tags that aren't on the site
    /// yet are left out of `tags`, which holds IDs.
    pub fn lay_over_editor_post(
        &self,
        post: &mut Map<String, Value>,
        utc_offset_seconds: Option<i32>,
    ) {
        let common = &self.common;
        set_raw(post, "title", &common.title);
        set_raw(post, "content", &common.content);
        set_raw(post, "excerpt", &common.excerpt);
        if let Some(slug) = &common.slug {
            post.insert("slug".into(), slug.clone().into());
        }
        let (status, password) = common.status.to_columns();
        post.insert("status".into(), status.into());
        post.insert("password".into(), password.unwrap_or_default().into());
        match common.publish_date {
            PublishDate::Immediately => {
                post.insert("date".into(), Value::Null);
                post.insert("date_gmt".into(), Value::Null);
            }
            PublishDate::At { date } => {
                post.insert("date_gmt".into(), date_to_json(date, 0));
                if let Some(offset) = utc_offset_seconds {
                    post.insert("date".into(), date_to_json(date, offset));
                }
            }
        }
        if let Some(author) = common.author {
            post.insert("author".into(), author.0.into());
        }
        post.insert(
            "featured_media".into(),
            common.featured_image.to_column().unwrap_or(0).into(),
        );
        if let Some(comment_status) = &common.comment_status {
            post.insert("comment_status".into(), comment_status.to_string().into());
        }
        if let Some(ping_status) = &common.ping_status {
            post.insert("ping_status".into(), ping_status.to_string().into());
        }
        post.insert("format".into(), self.format.as_str().into());
        post.insert("sticky".into(), self.sticky.into());
        post.insert(
            "categories".into(),
            self.categories.iter().map(|id| id.0).collect(),
        );
        post.insert(
            "tags".into(),
            existing_tag_ids(&self.tags).map(|id| id.0).collect(),
        );

        let meta = post
            .entry("meta")
            .or_insert_with(|| Value::Object(Map::new()));
        if let (Some(meta), Some(working_copy_meta)) =
            (meta.as_object_mut(), meta_object(&self.meta))
        {
            meta.extend(working_copy_meta);
        }
        if let Some(additional_fields) = self.additional_fields.raw.as_object() {
            post.extend(additional_fields.clone());
        }
    }

    /// The parameters that update a remote post to every one of these fields.
    ///
    /// A `PublishDate::Immediately` sends no date, so the post keeps its date on
    /// the server. Tags that aren't on the site yet aren't sent. Template,
    /// parent and menu order aren't sent, since the working copy has no field
    /// for them.
    pub fn to_update_params(&self) -> PostUpdateParams {
        let common = &self.common;
        let (status, password) = status_and_password_params(&common.status);
        PostUpdateParams {
            date_gmt: publish_date_param(common.publish_date),
            slug: common.slug.clone(),
            status: Some(status),
            password,
            title: Some(common.title.clone()),
            content: Some(common.content.clone()),
            author: common.author,
            excerpt: Some(common.excerpt.clone()),
            featured_media: Some(MediaId(common.featured_image.to_column().unwrap_or(0))),
            comment_status: common.comment_status.clone(),
            ping_status: common.ping_status.clone(),
            format: Some(self.format.clone()),
            meta: Some(self.meta.clone()),
            sticky: Some(self.sticky),
            categories: self.categories.clone(),
            tags: existing_tag_ids(&self.tags).collect(),
            additional_fields: Some(self.additional_fields_with_empty_term_lists()),
            ..Default::default()
        }
    }

    /// The parameters that create a remote post from these fields. The same
    /// fields as `to_update_params`.
    pub fn to_create_params(&self) -> PostCreateParams {
        let PostUpdateParams {
            date,
            date_gmt,
            slug,
            status,
            password,
            title,
            content,
            author,
            excerpt,
            featured_media,
            comment_status,
            ping_status,
            format,
            meta,
            sticky,
            template,
            categories,
            tags,
            parent,
            menu_order,
            additional_fields,
        } = self.to_update_params();
        PostCreateParams {
            date,
            date_gmt,
            slug,
            status,
            password,
            title,
            content,
            author,
            excerpt,
            featured_media,
            comment_status,
            ping_status,
            format,
            meta,
            sticky,
            template,
            categories,
            tags,
            parent,
            menu_order,
            additional_fields,
        }
    }
}

// Internal helpers

#[derive(Clone, Copy, PartialEq, Eq)]
enum Merge {
    Replace,
    KeyByKey,
}

impl PostWorkingCopyFields {
    fn apply_editor_keys(
        &mut self,
        post: &Map<String, Value>,
        utc_offset_seconds: Option<i32>,
        merge: Merge,
    ) -> Result<(), EditorPostConversionError> {
        let common = &mut self.common;
        if let Some(value) = post.get("title") {
            common.title = raw_string("title", value)?;
        }
        if let Some(value) = post.get("content") {
            common.content = raw_string("content", value)?;
        }
        if let Some(value) = post.get("excerpt") {
            common.excerpt = raw_string("excerpt", value)?;
        }
        if let Some(value) = post.get("slug") {
            common.slug = optional_string("slug", value)?.filter(|slug| !slug.is_empty());
        }
        if post.contains_key("status") || post.contains_key("password") {
            common.status = status_from_editor(&common.status, post)?;
        }
        if let Some(publish_date) = publish_date_from_editor(post, utc_offset_seconds, merge)? {
            common.publish_date = publish_date;
        }
        if let Some(value) = post.get("author") {
            common.author = optional_id("author", value)?.map(UserId);
        }
        if let Some(value) = post.get("featured_media") {
            common.featured_image =
                FeaturedImage::from_column(optional_id("featured_media", value)?);
        }
        if let Some(value) = post.get("comment_status") {
            common.comment_status = optional_string("comment_status", value)?
                .map(|status| status.parse().unwrap_or(PostCommentStatus::Custom(status)));
        }
        if let Some(value) = post.get("ping_status") {
            common.ping_status = optional_string("ping_status", value)?
                .map(|status| status.parse().unwrap_or(PostPingStatus::Custom(status)));
        }
        if let Some(value) = post.get("format") {
            let format = string("format", value)?;
            self.format = format.parse().unwrap_or(PostFormat::Custom(format));
        }
        if let Some(value) = post.get("sticky") {
            self.sticky = value
                .as_bool()
                .ok_or_else(|| invalid("sticky", "expected a boolean"))?;
        }
        if let Some(value) = post.get("categories") {
            self.categories = id_list("categories", value)?.map(TermId).collect();
        }
        if let Some(value) = post.get("tags") {
            self.tags = id_list("tags", value)?
                .map(|id| WorkingCopyTag::Existing { id: TermId(id) })
                .collect();
        }
        if let Some(value) = post.get("meta") {
            let edited = value
                .as_object()
                .ok_or_else(|| invalid("meta", "expected an object"))?;
            let mut meta = match merge {
                Merge::Replace => Map::new(),
                Merge::KeyByKey => meta_object(&self.meta).unwrap_or_default(),
            };
            meta.extend(edited.clone());
            self.meta = Arc::new(
                serde_json::from_value::<PostMeta>(Value::Object(meta))
                    .expect("Any JSON object is valid post meta"),
            );
        }

        let mut additional_fields = match merge {
            Merge::Replace => Map::new(),
            Merge::KeyByKey => self
                .additional_fields
                .raw
                .as_object()
                .cloned()
                .unwrap_or_default(),
        };
        additional_fields.extend(
            post.iter()
                .filter(|(key, _)| is_additional_field(key))
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        self.additional_fields = Arc::new(WpAdditionalFields {
            raw: Value::Object(additional_fields),
        });
        Ok(())
    }

    /// `PostUpdateParams` leaves out empty term lists, so a post whose last
    /// category or tag was removed would keep them on the server. The empty
    /// lists are sent with the additional fields instead.
    fn additional_fields_with_empty_term_lists(&self) -> Arc<WpAdditionalFields> {
        let mut fields = self
            .additional_fields
            .raw
            .as_object()
            .cloned()
            .unwrap_or_default();
        if self.categories.is_empty() {
            fields.insert("categories".into(), Value::Array(vec![]));
        }
        if existing_tag_ids(&self.tags).next().is_none() {
            fields.insert("tags".into(), Value::Array(vec![]));
        }
        Arc::new(WpAdditionalFields {
            raw: Value::Object(fields),
        })
    }
}

fn is_additional_field(key: &str) -> bool {
    const KEYS_WITH_A_FIELD: [&str; 17] = [
        "title",
        "content",
        "excerpt",
        "slug",
        "status",
        "password",
        "date",
        "date_gmt",
        "author",
        "featured_media",
        "comment_status",
        "ping_status",
        "format",
        "sticky",
        "categories",
        "tags",
        "meta",
    ];
    !key.starts_with('_')
        && !KEYS_WITH_A_FIELD.contains(&key)
        && !KEYS_WITHOUT_A_FIELD.contains(&key)
}

fn invalid(field: &str, reason: &str) -> EditorPostConversionError {
    EditorPostConversionError::InvalidField {
        field: field.to_string(),
        reason: reason.to_string(),
    }
}

fn string(field: &str, value: &Value) -> Result<String, EditorPostConversionError> {
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| invalid(field, "expected a string"))
}

fn optional_string(
    field: &str,
    value: &Value,
) -> Result<Option<String>, EditorPostConversionError> {
    match value {
        Value::Null => Ok(None),
        value => string(field, value).map(Some),
    }
}

/// A string, or an object holding it as `raw`, as the edit context returns
/// `title`, `content` and `excerpt`.
fn raw_string(field: &str, value: &Value) -> Result<String, EditorPostConversionError> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Object(object) => object
            .get("raw")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| invalid(field, "expected an object with a string `raw`")),
        _ => Err(invalid(field, "expected a string or an object with `raw`")),
    }
}

/// An ID where WordPress uses 0 or `null` for "none".
fn optional_id(field: &str, value: &Value) -> Result<Option<i64>, EditorPostConversionError> {
    match value {
        Value::Null => Ok(None),
        value => value
            .as_i64()
            .map(|id| Some(id).filter(|id| *id != 0))
            .ok_or_else(|| invalid(field, "expected an integer")),
    }
}

fn id_list<'a>(
    field: &str,
    value: &'a Value,
) -> Result<impl Iterator<Item = i64> + 'a, EditorPostConversionError> {
    let ids = value
        .as_array()
        .ok_or_else(|| invalid(field, "expected an array of integers"))?;
    if ids.iter().all(Value::is_i64) {
        Ok(ids.iter().filter_map(Value::as_i64))
    } else {
        Err(invalid(field, "expected an array of integers"))
    }
}

/// The status and password from the editor, each falling back to `current`
/// when the editor doesn't send it. An `auto-draft`, the status WordPress
/// gives a post nobody has saved yet, becomes a draft.
fn status_from_editor(
    current: &StatusSetting,
    post: &Map<String, Value>,
) -> Result<StatusSetting, EditorPostConversionError> {
    let (current_status, current_password) = current.to_columns();
    let status = match post.get("status") {
        Some(value) => string("status", value)?,
        None => current_status.to_string(),
    };
    let password = match post.get("password") {
        Some(value) => optional_string("password", value)?,
        None => current_password.map(str::to_string),
    };
    let status = match status.as_str() {
        "auto-draft" => PostStatus::Draft,
        _ => status.parse().unwrap_or(PostStatus::Custom(status)),
    };
    status_from_remote(&status, password.as_deref()).map_err(|err| match err {
        RemotePostConversionError::UneditableStatus { status } => {
            EditorPostConversionError::UneditableStatus { status }
        }
        err => invalid("status", &err.to_string()),
    })
}

/// The publish date from the editor's `date` and `date_gmt`, or `None` if it
/// has neither.
///
/// `date` wins over `date_gmt`, since `date` is what the editor edits. A
/// whole post can hold a `date_gmt` from before the user changed `date`.
fn publish_date_from_editor(
    post: &Map<String, Value>,
    utc_offset_seconds: Option<i32>,
    merge: Merge,
) -> Result<Option<PublishDate>, EditorPostConversionError> {
    let date = post
        .get("date")
        .map(|value| optional_string("date", value))
        .transpose()?;
    let date_gmt = post
        .get("date_gmt")
        .map(|value| optional_string("date_gmt", value))
        .transpose()?;

    // The block editor's `isEditedPostDateFloating`, which `from_remote` follows too.
    let floats = merge == Merge::Replace
        && matches!(
            post.get("status").and_then(Value::as_str),
            Some("draft" | "pending" | "auto-draft")
        )
        && date.as_ref().and_then(Option::as_deref) == post.get("modified").and_then(Value::as_str);

    match (date, date_gmt) {
        (None, None) => Ok(None),
        (Some(None), _) | (None, Some(None)) => Ok(Some(PublishDate::Immediately)),
        (Some(Some(_)), _) if floats => Ok(Some(PublishDate::Immediately)),
        (Some(Some(date)), date_gmt) => match (utc_offset_seconds, date_gmt.flatten()) {
            (Some(offset), _) => {
                let local = parse_date("date", &date)?;
                Ok(Some(PublishDate::At {
                    date: WpGmtDateTime(local.0 - chrono::Duration::seconds(offset.into())),
                }))
            }
            (None, Some(date_gmt)) => Ok(Some(PublishDate::At {
                date: parse_date("date_gmt", &date_gmt)?,
            })),
            (None, None) => Err(EditorPostConversionError::UnknownSiteTimezone),
        },
        (None, Some(Some(date_gmt))) => Ok(Some(PublishDate::At {
            date: parse_date("date_gmt", &date_gmt)?,
        })),
    }
}

/// Parses a date in WordPress's format. A date without a timezone reads as
/// UTC.
fn parse_date(field: &str, value: &str) -> Result<WpGmtDateTime, EditorPostConversionError> {
    value
        .parse()
        .map_err(|_| invalid(field, "expected a date such as 2024-01-15T10:30:00"))
}

/// A date in WordPress's format, without a timezone, at `offset_seconds` from
/// UTC.
fn date_to_json(date: WpGmtDateTime, offset_seconds: i32) -> Value {
    (date.0 + chrono::Duration::seconds(offset_seconds.into()))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
        .into()
}

fn set_raw(post: &mut Map<String, Value>, key: &str, raw: &str) {
    match post.get_mut(key).and_then(Value::as_object_mut) {
        Some(object) => {
            object.insert("raw".into(), raw.into());
        }
        None => {
            post.insert(key.into(), serde_json::json!({ "raw": raw }));
        }
    }
}

fn meta_object(meta: &PostMeta) -> Option<Map<String, Value>> {
    match serde_json::to_value(meta) {
        Ok(Value::Object(object)) => Some(object),
        _ => None,
    }
}

fn existing_tag_ids(tags: &[WorkingCopyTag]) -> impl Iterator<Item = TermId> + '_ {
    tags.iter().filter_map(|tag| match tag {
        WorkingCopyTag::Existing { id } => Some(*id),
        WorkingCopyTag::New { .. } => None,
    })
}

fn status_and_password_params(status: &StatusSetting) -> (PostStatus, Option<String>) {
    let (status, password) = status.to_columns();
    let status = status
        .parse()
        .unwrap_or_else(|_| PostStatus::Custom(status.to_string()));
    match status {
        // WordPress clears a private post's password itself.
        PostStatus::Private => (status, None),
        // An empty password removes the one the post has on the server.
        status => (status, Some(password.unwrap_or_default().to_string())),
    }
}

fn publish_date_param(publish_date: PublishDate) -> Option<WpGmtDateTime> {
    match publish_date {
        PublishDate::Immediately => None,
        PublishDate::At { date } => Some(date),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_types::working_copy::WorkingCopyCommonFields;
    use serde_json::json;
    use wp_api::JsonValue;

    fn gmt(value: &str) -> WpGmtDateTime {
        value.parse().expect("Date should parse")
    }

    fn object(value: Value) -> Map<String, Value> {
        match value {
            Value::Object(object) => object,
            _ => panic!("Expected a JSON object"),
        }
    }

    fn empty_fields() -> PostWorkingCopyFields {
        PostWorkingCopyFields {
            common: WorkingCopyCommonFields {
                title: String::new(),
                content: String::new(),
                excerpt: String::new(),
                slug: None,
                status: StatusSetting::Draft { password: None },
                publish_date: PublishDate::Immediately,
                author: None,
                featured_image: FeaturedImage::None,
                comment_status: None,
                ping_status: None,
            },
            categories: vec![],
            tags: vec![],
            format: PostFormat::Standard,
            sticky: false,
            meta: PostMeta::new(),
            additional_fields: WpAdditionalFields::new(),
        }
    }

    fn meta(key: &str, value: &str) -> Arc<PostMeta> {
        PostMeta::new().with_value(key.to_string(), JsonValue::String(value.to_string()))
    }

    fn additional_fields(value: Value) -> Arc<WpAdditionalFields> {
        Arc::new(WpAdditionalFields { raw: value })
    }

    /// A post as the block editor holds it: the edit context's shape, with the
    /// user's changes in it.
    fn editor_post() -> Map<String, Value> {
        object(json!({
            "id": 42,
            "date": "2024-01-15T12:30:00",
            "date_gmt": "2024-01-15T10:30:00",
            "modified": "2024-01-16T14:20:00",
            "link": "https://example.com/a-post",
            "type": "post",
            "slug": "a-post",
            "status": "publish",
            "password": "",
            "title": { "raw": "A post", "rendered": "A post" },
            "content": "<!-- wp:paragraph --><p>Edited</p><!-- /wp:paragraph -->",
            "excerpt": { "raw": "", "rendered": "" },
            "author": 10,
            "featured_media": 0,
            "comment_status": "open",
            "ping_status": "closed",
            "format": "aside",
            "sticky": true,
            "template": "",
            "categories": [1],
            "tags": [7, 8],
            "meta": { "footnotes": "" },
            "plugin_field": { "nested": [1, 2] },
            "_links": { "self": [] },
        }))
    }

    #[test]
    fn test_editor_post_converts_typed_fields_and_keeps_the_others() {
        let mut fields = empty_fields();

        fields
            .apply_editor_post(&editor_post(), Some(2 * 3600))
            .expect("Conversion should succeed");

        assert_eq!(
            fields,
            PostWorkingCopyFields {
                common: WorkingCopyCommonFields {
                    title: "A post".to_string(),
                    content: "<!-- wp:paragraph --><p>Edited</p><!-- /wp:paragraph -->".to_string(),
                    excerpt: String::new(),
                    slug: Some("a-post".to_string()),
                    status: StatusSetting::Publish { password: None },
                    publish_date: PublishDate::At {
                        date: gmt("2024-01-15T10:30:00Z"),
                    },
                    author: Some(UserId(10)),
                    featured_image: FeaturedImage::None,
                    comment_status: Some(PostCommentStatus::Open),
                    ping_status: Some(PostPingStatus::Closed),
                },
                categories: vec![TermId(1)],
                tags: vec![
                    WorkingCopyTag::Existing { id: TermId(7) },
                    WorkingCopyTag::Existing { id: TermId(8) },
                ],
                format: PostFormat::Aside,
                sticky: true,
                meta: meta("footnotes", ""),
                // `id`, `link`, `template` and the rest have no field, and
                // `_links` describes a response.
                additional_fields: additional_fields(
                    json!({ "plugin_field": { "nested": [1, 2] } })
                ),
            }
        );
    }

    #[test]
    fn test_editor_post_replaces_meta_and_additional_fields() {
        let mut fields = PostWorkingCopyFields {
            meta: meta("old_meta", "old"),
            additional_fields: additional_fields(json!({ "old_field": 1 })),
            ..empty_fields()
        };

        fields
            .apply_editor_post(&editor_post(), Some(0))
            .expect("Conversion should succeed");

        assert_eq!(fields.meta, meta("footnotes", ""));
        assert_eq!(
            fields.additional_fields,
            additional_fields(json!({ "plugin_field": { "nested": [1, 2] } }))
        );
    }

    #[test]
    fn test_editor_edits_change_only_what_they_send() {
        let mut fields = PostWorkingCopyFields {
            meta: meta("kept_meta", "kept"),
            additional_fields: additional_fields(json!({ "kept_field": 1, "edited_field": 1 })),
            ..empty_fields()
        };
        fields
            .apply_editor_post(&editor_post(), Some(0))
            .expect("Conversion should succeed");
        let before = PostWorkingCopyFields {
            meta: meta("kept_meta", "kept"),
            additional_fields: additional_fields(json!({ "kept_field": 1, "edited_field": 1 })),
            ..fields.clone()
        };
        let mut fields = before.clone();

        fields
            .apply_editor_edits(
                &object(json!({
                    "id": 42,
                    "title": "Edited title",
                    "meta": { "edited_meta": "new" },
                    "edited_field": 2,
                })),
                None,
            )
            .expect("Conversion should succeed");

        assert_eq!(
            fields,
            PostWorkingCopyFields {
                common: WorkingCopyCommonFields {
                    title: "Edited title".to_string(),
                    ..before.common.clone()
                },
                meta: meta("kept_meta", "kept").with_value(
                    "edited_meta".to_string(),
                    JsonValue::String("new".to_string())
                ),
                additional_fields: additional_fields(json!({ "kept_field": 1, "edited_field": 2 })),
                ..before
            }
        );
    }

    #[test]
    fn test_editor_date_converts_with_the_site_offset() {
        let date_only = object(json!({ "date": "2024-01-15T12:30:00" }));
        let with_date_gmt = object(json!({
            "date": "2024-01-15T12:30:00",
            "date_gmt": "2024-01-15T10:30:00",
        }));
        for (edits, offset, expected) in [
            // `date` is in the site's timezone.
            (
                &date_only,
                Some(2 * 3600),
                Ok(PublishDate::At {
                    date: gmt("2024-01-15T10:30:00Z"),
                }),
            ),
            // Without the offset, only `date_gmt` says when it is.
            (
                &with_date_gmt,
                None,
                Ok(PublishDate::At {
                    date: gmt("2024-01-15T10:30:00Z"),
                }),
            ),
            (
                &date_only,
                None,
                Err(EditorPostConversionError::UnknownSiteTimezone),
            ),
            // `null` is how the block editor resets a date to "Immediately".
            (
                &object(json!({ "date": null })),
                None,
                Ok(PublishDate::Immediately),
            ),
        ] {
            let mut fields = PostWorkingCopyFields {
                common: WorkingCopyCommonFields {
                    publish_date: PublishDate::At {
                        date: gmt("2000-01-01T00:00:00Z"),
                    },
                    ..empty_fields().common
                },
                ..empty_fields()
            };

            let result = fields
                .apply_editor_edits(edits, offset)
                .map(|()| fields.common.publish_date);

            assert_eq!(result, expected, "edits {edits:?}, offset {offset:?}");
        }
    }

    #[test]
    fn test_unpublished_editor_post_whose_date_is_its_modified_date_floats() {
        let mut post = editor_post();
        post.insert("status".into(), "draft".into());
        post.insert("date".into(), "2024-01-16T14:20:00".into());
        let mut fields = empty_fields();

        fields
            .apply_editor_post(&post, Some(0))
            .expect("Conversion should succeed");

        assert_eq!(fields.common.publish_date, PublishDate::Immediately);
    }

    #[test]
    fn test_editor_status_and_password_each_keep_the_other() {
        let protected_draft = StatusSetting::Draft {
            password: Some("secret".to_string()),
        };
        for (edits, expected) in [
            (
                json!({ "status": "publish" }),
                Ok(StatusSetting::Publish {
                    password: Some("secret".to_string()),
                }),
            ),
            (
                json!({ "password": "" }),
                Ok(StatusSetting::Draft { password: None }),
            ),
            (
                json!({ "status": "auto-draft" }),
                Ok(StatusSetting::Draft {
                    password: Some("secret".to_string()),
                }),
            ),
            (json!({ "status": "private" }), Ok(StatusSetting::Private)),
            (
                json!({ "status": "trash" }),
                Err(EditorPostConversionError::UneditableStatus {
                    status: "trash".to_string(),
                }),
            ),
        ] {
            let mut fields = PostWorkingCopyFields {
                common: WorkingCopyCommonFields {
                    status: protected_draft.clone(),
                    ..empty_fields().common
                },
                ..empty_fields()
            };

            let result = fields
                .apply_editor_edits(&object(edits.clone()), None)
                .map(|()| fields.common.status);

            assert_eq!(result, expected, "edits {edits}");
        }
    }

    #[test]
    fn test_editor_post_with_a_value_of_the_wrong_type_is_refused() {
        let mut fields = empty_fields();

        let result = fields.apply_editor_edits(&object(json!({ "categories": ["1"] })), None);

        assert_eq!(
            result,
            Err(EditorPostConversionError::InvalidField {
                field: "categories".to_string(),
                reason: "expected an array of integers".to_string(),
            })
        );
    }

    #[test]
    fn test_update_params_send_every_field() {
        let fields = PostWorkingCopyFields {
            common: WorkingCopyCommonFields {
                title: "A post".to_string(),
                status: StatusSetting::Publish { password: None },
                publish_date: PublishDate::At {
                    date: gmt("2024-01-15T10:30:00Z"),
                },
                ..empty_fields().common
            },
            meta: meta("footnotes", ""),
            additional_fields: additional_fields(json!({ "plugin_field": 1 })),
            ..empty_fields()
        };

        let body =
            serde_json::to_value(fields.to_update_params()).expect("Params should serialize");

        assert_eq!(
            body,
            json!({
                "date_gmt": "2024-01-15T10:30:00",
                "status": "publish",
                // An empty password removes the post's password on the server.
                "password": "",
                "title": "A post",
                "content": "",
                "excerpt": "",
                "featured_media": 0,
                "format": "standard",
                "meta": { "footnotes": "" },
                "sticky": false,
                // Empty term lists are sent, so removing the last one is pushed too.
                "categories": [],
                "tags": [],
                "plugin_field": 1,
            })
        );
    }

    #[test]
    fn test_editor_post_round_trips_through_update_params() {
        let mut fields = empty_fields();
        fields
            .apply_editor_post(&editor_post(), Some(2 * 3600))
            .expect("Conversion should succeed");
        let body = object(
            serde_json::to_value(fields.to_update_params()).expect("Params should serialize"),
        );

        let mut round_tripped = empty_fields();
        round_tripped
            .apply_editor_post(&body, None)
            .expect("Conversion should succeed");

        assert_eq!(round_tripped, fields);
    }

    #[test]
    fn test_laying_over_an_editor_post_keeps_what_the_working_copy_lacks() {
        let mut fields = empty_fields();
        fields
            .apply_editor_post(&editor_post(), Some(2 * 3600))
            .expect("Conversion should succeed");
        fields.common.title = "Working copy title".to_string();
        fields.meta = meta("footnotes", "").with_value(
            "working_copy_meta".to_string(),
            JsonValue::String("value".to_string()),
        );
        let mut post = editor_post();

        fields.lay_over_editor_post(&mut post, Some(2 * 3600));

        assert_eq!(
            post.get("title"),
            Some(&json!({ "raw": "Working copy title", "rendered": "A post" }))
        );
        assert_eq!(
            post.get("meta"),
            Some(&json!({ "footnotes": "", "working_copy_meta": "value" }))
        );
        assert_eq!(post.get("id"), Some(&json!(42)));
        assert_eq!(post.get("date"), Some(&json!("2024-01-15T12:30:00")));
        let mut converted_back = empty_fields();
        converted_back
            .apply_editor_post(&post, Some(2 * 3600))
            .expect("Conversion should succeed");
        assert_eq!(converted_back, fields);
    }
}
