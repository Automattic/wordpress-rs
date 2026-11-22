//! Converting between working copy fields and a post in the REST API's shape,
//! as a block editor holds it and sends it.
//!
//! The editor's post is a JSON object. Each key of it is one of three kinds:
//!
//! - A key with a working copy field, such as `title`, `status` or `date`,
//!   converts into that field. `meta` goes into `meta`.
//! - A key the server sets, such as `id`, `modified`, `link` or `date_gmt`, is
//!   left out. The user can't change it, so it isn't part of what the user
//!   wrote.
//! - Any other key, such as a field a plugin adds, goes into
//!   `additional_fields` as it is, except keys starting with `_`, such as
//!   `_links`, which describe a response rather than the post.

use std::sync::Arc;

use serde_json::{Map, Value};
use wp_api::{
    WpAdditionalFields,
    date::WpDateString,
    posts::{PostCommentStatus, PostFormat, PostMeta, PostPingStatus, PostStatus},
    terms::TermId,
    users::UserId,
};

use crate::{
    db_types::working_copy::{FeaturedImage, PostWorkingCopyFields, PublishDate, StatusSetting},
    remote::{RemotePostConversionError, status_from_remote},
};

/// The keys the server sets, which the user can't change.
const SERVER_KEYS: [&str; 9] = [
    "id",
    "guid",
    "link",
    "modified",
    "modified_gmt",
    "date_gmt",
    "type",
    "permalink_template",
    "generated_slug",
];

/// The keys with a working copy field.
const KEYS_WITH_A_FIELD: [&str; 18] = [
    "title",
    "content",
    "excerpt",
    "slug",
    "status",
    "password",
    "date",
    "author",
    "featured_media",
    "comment_status",
    "ping_status",
    "template",
    "menu_order",
    "format",
    "sticky",
    "categories",
    "tags",
    "meta",
];

/// The format of the REST API's `date`, in the site's timezone.
const DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

/// Why an editor's post can't be converted into working copy fields.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum EditorPostConversionError {
    #[error("The editor's post must be a JSON object")]
    NotAnObject,
    #[error("The editor's {field} is invalid: {reason}")]
    InvalidField { field: String, reason: String },
    #[error("A post with status {status} can't be edited")]
    UneditableStatus { status: String },
}

impl PostWorkingCopyFields {
    /// Take the whole post as the editor has it.
    ///
    /// The fields whose keys `post` has are replaced, and the others stay as
    /// they are. `meta` and `additional_fields` are replaced as a whole. An
    /// unpublished post whose `date` equals its `modified` gets
    /// `PublishDate::Immediately`, as in `from_remote`.
    pub fn apply_editor_post(
        &mut self,
        post: &Map<String, Value>,
    ) -> Result<(), EditorPostConversionError> {
        self.apply_editor_keys(post, Merge::Replace)
    }

    /// Take the fields an editor's write sends: only the ones that changed.
    ///
    /// The same conversion as `apply_editor_post`, except that `meta` and
    /// `additional_fields` are merged key by key, so the keys `edits` doesn't
    /// have keep their values.
    pub fn apply_editor_edits(
        &mut self,
        edits: &Map<String, Value>,
    ) -> Result<(), EditorPostConversionError> {
        self.apply_editor_keys(edits, Merge::KeyByKey)
    }

    /// Lay these fields over `post`, a post in the REST API's edit shape, so
    /// it shows what the working copy holds.
    ///
    /// The keys of `post` with no working copy field keep their values, and so
    /// do `rendered` values, which can be out of date afterwards. The meta and
    /// additional fields are laid over key by key.
    pub fn lay_over_editor_post(&self, post: &mut Map<String, Value>) {
        let mut fields = self.to_rest_fields();
        for key in ["title", "content", "excerpt"] {
            if let Some(raw) = fields.remove(key) {
                match post.get_mut(key).and_then(Value::as_object_mut) {
                    Some(object) => {
                        object.insert("raw".into(), raw);
                    }
                    None => {
                        post.insert(key.into(), serde_json::json!({ "raw": raw }));
                    }
                }
            }
        }
        if let Some(Value::Object(working_copy_meta)) = fields.remove("meta") {
            match post.get_mut("meta").and_then(Value::as_object_mut) {
                Some(meta) => meta.extend(working_copy_meta),
                None => {
                    post.insert("meta".into(), Value::Object(working_copy_meta));
                }
            }
        }
        post.extend(fields);
    }

    /// Every field, as the REST API takes it when creating or updating a post.
    ///
    /// `Immediately` is a `date` of `null`. A field the user hasn't set, such
    /// as a slug or an author, is left out, and so is a private post's
    /// password, which WordPress clears. The additional fields are top-level
    /// keys, as the server returns them.
    pub fn to_rest_fields(&self) -> Map<String, Value> {
        let common = &self.common;
        let mut fields = Map::new();
        fields.insert("title".into(), common.title.clone().into());
        fields.insert("content".into(), common.content.clone().into());
        fields.insert("excerpt".into(), common.excerpt.clone().into());
        if let Some(slug) = &common.slug {
            fields.insert("slug".into(), slug.clone().into());
        }
        let (status, password) = common.status.to_columns();
        fields.insert("status".into(), status.into());
        if status != "private" {
            // An empty password removes the one the post has on the server.
            fields.insert("password".into(), password.unwrap_or_default().into());
        }
        fields.insert(
            "date".into(),
            match &common.publish_date {
                PublishDate::Immediately => Value::Null,
                PublishDate::At { date } => date.value.clone().into(),
            },
        );
        if let Some(author) = common.author {
            fields.insert("author".into(), author.0.into());
        }
        fields.insert(
            "featured_media".into(),
            common.featured_image.to_column().unwrap_or(0).into(),
        );
        if let Some(comment_status) = &common.comment_status {
            fields.insert("comment_status".into(), comment_status.to_string().into());
        }
        if let Some(ping_status) = &common.ping_status {
            fields.insert("ping_status".into(), ping_status.to_string().into());
        }
        fields.insert(
            "template".into(),
            common.template.clone().unwrap_or_default().into(),
        );
        if let Some(menu_order) = common.menu_order {
            fields.insert("menu_order".into(), menu_order.into());
        }
        fields.insert("format".into(), self.format.as_str().into());
        fields.insert("sticky".into(), self.sticky.into());
        fields.insert(
            "categories".into(),
            self.categories.iter().map(|id| id.0).collect(),
        );
        fields.insert("tags".into(), self.tags.iter().map(|id| id.0).collect());
        fields.insert(
            "meta".into(),
            Value::Object(meta_object(&self.meta).unwrap_or_default()),
        );
        if let Some(additional_fields) = self.additional_fields.raw.as_object() {
            fields.extend(
                additional_fields
                    .iter()
                    .filter(|(key, _)| is_additional_field(key))
                    .map(|(key, value)| (key.clone(), value.clone())),
            );
        }
        fields
    }

    /// The fields of `to_rest_fields` whose values differ from `server`'s,
    /// such as a working copy's from the post on the server. The meta keys are
    /// compared one by one, and `meta` holds only those that differ.
    pub fn rest_fields_changed_from(&self, server: &PostWorkingCopyFields) -> Map<String, Value> {
        let server = server.to_rest_fields();
        self.to_rest_fields()
            .into_iter()
            .filter_map(|(key, value)| match (key.as_str(), value) {
                ("meta", Value::Object(meta)) => {
                    let server_meta = server.get("meta").and_then(Value::as_object);
                    let changed: Map<String, Value> = meta
                        .into_iter()
                        .filter(|(meta_key, meta_value)| {
                            server_meta.and_then(|server_meta| server_meta.get(meta_key))
                                != Some(meta_value)
                        })
                        .collect();
                    (!changed.is_empty()).then_some((key, Value::Object(changed)))
                }
                (_, value) => (server.get(&key) != Some(&value)).then_some((key, value)),
            })
            .collect()
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
        if let Some(publish_date) = publish_date_from_editor(post, merge)? {
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
        if let Some(value) = post.get("template") {
            common.template =
                optional_string("template", value)?.filter(|template| !template.is_empty());
        }
        if let Some(value) = post.get("menu_order") {
            common.menu_order = match value {
                Value::Null => None,
                value => Some(
                    value
                        .as_i64()
                        .ok_or_else(|| invalid("menu_order", "expected an integer"))?,
                ),
            };
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
            self.tags = id_list("tags", value)?.map(TermId).collect();
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
}

fn is_additional_field(key: &str) -> bool {
    !key.starts_with('_') && !KEYS_WITH_A_FIELD.contains(&key) && !SERVER_KEYS.contains(&key)
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

/// The publish date from the editor's `date`, which is in the site's
/// timezone, or `None` if the editor didn't send one. `null` is how the block
/// editor resets a date to "Immediately".
fn publish_date_from_editor(
    post: &Map<String, Value>,
    merge: Merge,
) -> Result<Option<PublishDate>, EditorPostConversionError> {
    let Some(date) = post.get("date") else {
        return Ok(None);
    };
    let Some(date) = optional_string("date", date)? else {
        return Ok(Some(PublishDate::Immediately));
    };
    // The block editor's `isEditedPostDateFloating`, which `from_remote`
    // follows too.
    let floats = merge == Merge::Replace
        && matches!(
            post.get("status").and_then(Value::as_str),
            Some("draft" | "pending" | "auto-draft")
        )
        && post.get("modified").and_then(Value::as_str) == Some(date.as_str());
    if floats {
        return Ok(Some(PublishDate::Immediately));
    }
    chrono::NaiveDateTime::parse_from_str(&date, DATE_FORMAT)
        .map_err(|_| invalid("date", "expected a date such as 2024-01-15T10:30:00"))?;
    Ok(Some(PublishDate::At {
        date: WpDateString::new(date),
    }))
}

fn meta_object(meta: &PostMeta) -> Option<Map<String, Value>> {
    match serde_json::to_value(meta) {
        Ok(Value::Object(object)) => Some(object),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_types::working_copy::WorkingCopyCommonFields;
    use serde_json::json;
    use wp_api::JsonValue;

    fn date(value: &str) -> PublishDate {
        PublishDate::At {
            date: WpDateString::new(value),
        }
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
                template: None,
                menu_order: None,
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
            "template": "wide.php",
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
            .apply_editor_post(&editor_post())
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
                    // The site's date, not `date_gmt`.
                    publish_date: date("2024-01-15T12:30:00"),
                    author: Some(UserId(10)),
                    featured_image: FeaturedImage::None,
                    comment_status: Some(PostCommentStatus::Open),
                    ping_status: Some(PostPingStatus::Closed),
                    template: Some("wide.php".to_string()),
                    menu_order: None,
                },
                categories: vec![TermId(1)],
                tags: vec![TermId(7), TermId(8)],
                format: PostFormat::Aside,
                sticky: true,
                meta: meta("footnotes", ""),
                // `id`, `link` and the rest are the server's, and `_links`
                // describes a response.
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
            .apply_editor_post(&editor_post())
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
            .apply_editor_post(&editor_post())
            .expect("Conversion should succeed");
        let before = PostWorkingCopyFields {
            meta: meta("kept_meta", "kept"),
            additional_fields: additional_fields(json!({ "kept_field": 1, "edited_field": 1 })),
            ..fields.clone()
        };
        let mut fields = before.clone();

        fields
            .apply_editor_edits(&object(json!({
                "id": 42,
                "title": "Edited title",
                "meta": { "edited_meta": "new" },
                "edited_field": 2,
            })))
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
    fn test_editor_date_is_kept_in_the_site_timezone() {
        for (edits, expected) in [
            (
                json!({ "date": "2024-01-15T12:30:00", "date_gmt": "2024-01-15T10:30:00" }),
                Ok(date("2024-01-15T12:30:00")),
            ),
            // `null` is how the block editor resets a date to "Immediately".
            (json!({ "date": null }), Ok(PublishDate::Immediately)),
            // `date_gmt` is the server's, worked out from `date`.
            (
                json!({ "date_gmt": "2024-01-15T10:30:00" }),
                Ok(date("2000-01-01T00:00:00")),
            ),
            (
                json!({ "date": "15 January 2024" }),
                Err(EditorPostConversionError::InvalidField {
                    field: "date".to_string(),
                    reason: "expected a date such as 2024-01-15T10:30:00".to_string(),
                }),
            ),
        ] {
            let mut fields = PostWorkingCopyFields {
                common: WorkingCopyCommonFields {
                    publish_date: date("2000-01-01T00:00:00"),
                    ..empty_fields().common
                },
                ..empty_fields()
            };

            let result = fields
                .apply_editor_edits(&object(edits.clone()))
                .map(|()| fields.common.publish_date.clone());

            assert_eq!(result, expected, "edits {edits}");
        }
    }

    #[test]
    fn test_unpublished_editor_post_whose_date_is_its_modified_date_floats() {
        let mut post = editor_post();
        post.insert("status".into(), "draft".into());
        post.insert("date".into(), "2024-01-16T14:20:00".into());
        let mut fields = empty_fields();

        fields
            .apply_editor_post(&post)
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
                .apply_editor_edits(&object(edits.clone()))
                .map(|()| fields.common.status);

            assert_eq!(result, expected, "edits {edits}");
        }
    }

    #[test]
    fn test_editor_post_with_a_value_of_the_wrong_type_is_refused() {
        let mut fields = empty_fields();

        let result = fields.apply_editor_edits(&object(json!({ "categories": ["1"] })));

        assert_eq!(
            result,
            Err(EditorPostConversionError::InvalidField {
                field: "categories".to_string(),
                reason: "expected an array of integers".to_string(),
            })
        );
    }

    #[test]
    fn test_rest_fields_hold_every_field() {
        let fields = PostWorkingCopyFields {
            common: WorkingCopyCommonFields {
                title: "A post".to_string(),
                status: StatusSetting::Publish { password: None },
                ..empty_fields().common
            },
            meta: meta("footnotes", ""),
            additional_fields: additional_fields(json!({ "plugin_field": 1 })),
            ..empty_fields()
        };

        assert_eq!(
            Value::Object(fields.to_rest_fields()),
            json!({
                "title": "A post",
                "content": "",
                "excerpt": "",
                "status": "publish",
                // An empty password removes the post's password on the server.
                "password": "",
                // `Immediately`.
                "date": null,
                "featured_media": 0,
                // An empty template is the default one.
                "template": "",
                "format": "standard",
                "sticky": false,
                // Empty term lists are sent, so removing the last one is pushed too.
                "categories": [],
                "tags": [],
                "meta": { "footnotes": "" },
                "plugin_field": 1,
            })
        );
    }

    #[test]
    fn test_editor_post_round_trips_through_rest_fields() {
        let mut fields = empty_fields();
        fields
            .apply_editor_post(&editor_post())
            .expect("Conversion should succeed");

        let mut round_tripped = empty_fields();
        round_tripped
            .apply_editor_post(&fields.to_rest_fields())
            .expect("Conversion should succeed");

        assert_eq!(round_tripped, fields);
    }

    #[test]
    fn test_changed_rest_fields_hold_only_what_differs_from_the_server() {
        let mut server = empty_fields();
        server
            .apply_editor_post(&editor_post())
            .expect("Conversion should succeed");
        let mut fields = server.clone();
        fields
            .apply_editor_edits(&object(json!({
                "title": "Edited",
                "tags": [],
                "meta": { "plugin_meta": "1" },
            })))
            .expect("Conversion should succeed");

        assert_eq!(
            Value::Object(fields.rest_fields_changed_from(&server)),
            json!({
                "title": "Edited",
                "tags": [],
                // Only the meta key that changed.
                "meta": { "plugin_meta": "1" },
            })
        );
    }

    #[test]
    fn test_laying_over_an_editor_post_keeps_what_the_working_copy_lacks() {
        let mut fields = empty_fields();
        fields
            .apply_editor_post(&editor_post())
            .expect("Conversion should succeed");
        fields.common.title = "Working copy title".to_string();
        fields.meta = meta("footnotes", "").with_value(
            "working_copy_meta".to_string(),
            JsonValue::String("value".to_string()),
        );
        let mut post = editor_post();

        fields.lay_over_editor_post(&mut post);

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
            .apply_editor_post(&post)
            .expect("Conversion should succeed");
        assert_eq!(converted_back, fields);
    }
}
