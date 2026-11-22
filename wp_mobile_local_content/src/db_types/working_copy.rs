//! The fields of a post or page as the user edits it, and the stored working
//! copies that hold them.
//!
//! Each field can hold any value WordPress returns for it, including statuses
//! and formats that plugins register. See
//! `Documentation/local-content-principles.md`.

use std::sync::Arc;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use uuid::Uuid;
use wp_api::{
    WpAdditionalFields,
    date::WpGmtDateTime,
    media::MediaId,
    posts::{PostCommentStatus, PostFormat, PostId, PostMeta, PostPingStatus, PostStatus},
    terms::TermId,
    users::UserId,
};
use wp_mobile_sqlite::ColumnIndex;

use crate::{LocalContentError, db_types::working_copy_remote_target::WorkingCopyRemoteTarget};

uniffi::custom_newtype!(WorkingCopyId, String);

/// The ID of a working copy, assigned when it's created: a lowercase,
/// hyphenated v4 UUID.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkingCopyId(pub String);

impl WorkingCopyId {
    pub(crate) fn generate() -> Self {
        Self(Uuid::new_v4().hyphenated().to_string())
    }
}

impl ToSql for WorkingCopyId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.as_str()))
    }
}

impl FromSql for WorkingCopyId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        String::column_result(value).map(WorkingCopyId)
    }
}

/// The post's status, and its password if it has one.
///
/// A `password` of `None` means the post has no password. A private post can't
/// have one, because WordPress removes the password when it saves a private
/// post.
///
/// There is no scheduled status. A scheduled post is `Publish` with a
/// `PublishDate` in the future. WordPress reports such a post as `future` until
/// its date passes, and as `publish` after.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum StatusSetting {
    Draft {
        password: Option<String>,
    },
    PendingReview {
        password: Option<String>,
    },
    Publish {
        password: Option<String>,
    },
    Private,
    /// A status the other variants don't cover, such as one a plugin registers.
    ///
    /// Saving a `Custom` status that another variant covers stores that
    /// variant, so it reads back as that variant. `draft` becomes `Draft`,
    /// `pending` becomes `PendingReview`, `publish` and `future` become
    /// `Publish`, and `private` becomes `Private` without its password. An
    /// empty status becomes `Draft`.
    Custom {
        status: String,
        password: Option<String>,
    },
}

/// When the post is published. With `Immediately`, WordPress sets the date
/// when the post is published.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum PublishDate {
    Immediately,
    At { date: WpGmtDateTime },
}

/// The image the user picked as the featured image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FeaturedImage {
    None,
    Remote { media_id: MediaId },
}

/// Fields shared by post and page working copies.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct WorkingCopyCommonFields {
    pub title: String,
    pub content: String,
    pub excerpt: String,
    /// `None` when the user hasn't set a slug.
    pub slug: Option<String>,
    pub status: StatusSetting,
    pub publish_date: PublishDate,
    /// `None` when the user hasn't chosen an author.
    pub author: Option<UserId>,
    pub featured_image: FeaturedImage,
    /// `None` when the user hasn't chosen whether comments are open.
    pub comment_status: Option<PostCommentStatus>,
    /// `None` when the user hasn't chosen whether pings are open.
    pub ping_status: Option<PostPingStatus>,
    /// The theme template the post uses, or `None` for the default one.
    pub template: Option<String>,
    /// The post's place in a custom order, or `None` if its post type has no
    /// such order, as posts don't.
    pub menu_order: Option<i64>,
}

/// A post as the user edits it on the device, holding what the editor and its
/// settings show.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PostWorkingCopyFields {
    pub common: WorkingCopyCommonFields,
    pub categories: Vec<TermId>,
    pub tags: Vec<TermId>,
    pub format: PostFormat,
    pub sticky: bool,
    /// Post meta, one value per meta key.
    pub meta: Arc<PostMeta>,
    /// The post's fields that have no typed field here, such as fields plugins
    /// add, one value per key.
    pub additional_fields: Arc<WpAdditionalFields>,
}

/// A page as the user edits it on the device, holding what the editor and its
/// settings show.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PageWorkingCopyFields {
    pub common: WorkingCopyCommonFields,
    /// `None` for a top-level page.
    pub parent: Option<PostId>,
}

/// A stored post working copy: the user's version of a post, and where pushing
/// it goes.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PostWorkingCopy {
    pub id: WorkingCopyId,
    pub fields: PostWorkingCopyFields,
    pub remote_target: WorkingCopyRemoteTarget,
    /// When the working copy was created, as an ISO 8601 UTC timestamp.
    pub created_at: String,
    /// When the fields were last saved, as an ISO 8601 UTC timestamp. Until
    /// the first save, it's the same as `created_at`.
    pub updated_at: String,
}

/// A stored page working copy: the user's version of a page, and where pushing
/// it goes.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PageWorkingCopy {
    pub id: WorkingCopyId,
    pub fields: PageWorkingCopyFields,
    pub remote_target: WorkingCopyRemoteTarget,
    /// When the working copy was created, as an ISO 8601 UTC timestamp.
    pub created_at: String,
    /// When the fields were last saved, as an ISO 8601 UTC timestamp. Until
    /// the first save, it's the same as `created_at`.
    pub updated_at: String,
}

/// The kind of a working copy, stored in `working_copies.kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkingCopyKind {
    Post,
    Page,
}

impl WorkingCopyKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Post => "post",
            Self::Page => "page",
        }
    }
}

impl StatusSetting {
    /// The `status` and `password` column values. The status is stored as
    /// WordPress names it, and a `Custom` status another variant covers is
    /// stored as that variant, so it reads back as that variant.
    pub(crate) fn to_columns(&self) -> (&str, Option<&str>) {
        match self {
            Self::Draft { password } => ("draft", password.as_deref()),
            Self::PendingReview { password } => ("pending", password.as_deref()),
            Self::Publish { password } => ("publish", password.as_deref()),
            Self::Private => ("private", None),
            Self::Custom { status, password } => {
                let password = password.as_deref();
                match status.parse::<PostStatus>() {
                    Ok(PostStatus::Draft) => ("draft", password),
                    Ok(PostStatus::Pending) => ("pending", password),
                    Ok(PostStatus::Publish | PostStatus::Future) => ("publish", password),
                    Ok(PostStatus::Private) => ("private", None),
                    _ if status.is_empty() => ("draft", password),
                    _ => (status, password),
                }
            }
        }
    }

    pub(crate) fn from_columns(
        status: &str,
        password: Option<String>,
    ) -> Result<Self, FromSqlError> {
        match status {
            "draft" => Ok(Self::Draft { password }),
            "pending" => Ok(Self::PendingReview { password }),
            "publish" => Ok(Self::Publish { password }),
            "private" if password.is_none() => Ok(Self::Private),
            "private" => Err(FromSqlError::Other(
                "A private working copy has a password".into(),
            )),
            status => Ok(Self::Custom {
                status: status.to_string(),
                password,
            }),
        }
    }
}

/// The `category_ids` and `tag_ids` column values: a JSON array of term IDs.
pub(crate) fn term_ids_to_json(term_ids: &[TermId]) -> Result<String, LocalContentError> {
    let ids: Vec<i64> = term_ids.iter().map(|term_id| term_id.0).collect();
    serde_json::to_string(&ids).map_err(json_error)
}

pub(crate) fn term_ids_from_json(json: &str) -> Result<Vec<TermId>, FromSqlError> {
    let ids: Vec<i64> = serde_json::from_str(json).map_err(|e| FromSqlError::Other(e.into()))?;
    Ok(ids.into_iter().map(TermId).collect())
}

/// The `meta` column value: a JSON object.
pub(crate) fn meta_to_json(meta: &PostMeta) -> Result<String, LocalContentError> {
    serde_json::to_value(meta)
        .map_err(json_error)
        .and_then(|value| object_to_json("meta", &value))
}

pub(crate) fn meta_from_json(json: &str) -> Result<Arc<PostMeta>, FromSqlError> {
    serde_json::from_str(json)
        .map(Arc::new)
        .map_err(|e| FromSqlError::Other(e.into()))
}

/// The `additional_fields` column value: a JSON object.
pub(crate) fn additional_fields_to_json(
    additional_fields: &WpAdditionalFields,
) -> Result<String, LocalContentError> {
    object_to_json("additional_fields", &additional_fields.raw)
}

pub(crate) fn additional_fields_from_json(
    json: &str,
) -> Result<Arc<WpAdditionalFields>, FromSqlError> {
    serde_json::from_str(json)
        .map(|raw| Arc::new(WpAdditionalFields { raw }))
        .map_err(|e| FromSqlError::Other(e.into()))
}

fn object_to_json(field: &str, value: &serde_json::Value) -> Result<String, LocalContentError> {
    if value.is_object() {
        serde_json::to_string(value).map_err(json_error)
    } else {
        Err(LocalContentError::NotAJsonObject {
            field: field.to_string(),
        })
    }
}

fn json_error(err: serde_json::Error) -> LocalContentError {
    LocalContentError::Sqlite {
        err_message: format!("Failed to serialize to JSON: {err}"),
    }
}

impl PublishDate {
    /// The `publish_date` column value.
    pub(crate) fn to_column(self) -> Option<i64> {
        match self {
            Self::Immediately => None,
            Self::At { date } => Some(date.0.timestamp()),
        }
    }

    pub(crate) fn from_column(timestamp: Option<i64>) -> Result<Self, FromSqlError> {
        timestamp.map_or(Ok(Self::Immediately), |seconds| {
            date_from_timestamp(seconds).map(|date| Self::At { date })
        })
    }
}

impl FeaturedImage {
    /// The `featured_media_id` column value.
    pub(crate) fn to_column(self) -> Option<i64> {
        match self {
            Self::None => None,
            Self::Remote { media_id } => Some(media_id.0),
        }
    }

    pub(crate) fn from_column(media_id: Option<i64>) -> Self {
        media_id.map_or(Self::None, |id| Self::Remote {
            media_id: MediaId(id),
        })
    }
}

pub(crate) fn date_from_timestamp(seconds: i64) -> Result<WpGmtDateTime, FromSqlError> {
    chrono::DateTime::from_timestamp(seconds, 0)
        .map(WpGmtDateTime)
        .ok_or_else(|| FromSqlError::Other(format!("Timestamp out of range: {seconds}").into()))
}

/// Column indexes for the working_copies table.
/// These must match the order of columns in the CREATE TABLE statement.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum WorkingCopyColumn {
    Rowid = 0,
    Uuid = 1,
    // `kind` (index 2) is matched in the WHERE clause and never read.
    Title = 3,
    Content = 4,
    Excerpt = 5,
    Slug = 6,
    Status = 7,
    Password = 8,
    PublishDate = 9,
    AuthorId = 10,
    FeaturedMediaId = 11,
    CommentStatus = 12,
    PingStatus = 13,
    CreatedAt = 14,
    UpdatedAt = 15,
    Template = 16,
    MenuOrder = 17,
}

impl ColumnIndex for WorkingCopyColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}

/// Column indexes for the post_working_copy_fields table.
///
/// `working_copy_rowid` (index 0) is matched in the WHERE clause and never read.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum PostFieldsColumn {
    CategoryIds = 1,
    TagIds = 2,
    Format = 3,
    Sticky = 4,
    Meta = 5,
    AdditionalFields = 6,
}

impl ColumnIndex for PostFieldsColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}

/// Column indexes for the page_working_copy_fields table.
///
/// `working_copy_rowid` (index 0) is matched in the WHERE clause and never read.
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum PageFieldsColumn {
    ParentId = 1,
}

impl ColumnIndex for PageFieldsColumn {
    fn as_index(&self) -> usize {
        *self as usize
    }
}
