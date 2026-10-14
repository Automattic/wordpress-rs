//! Working copy types.
//!
//! Each field holds what the editor or its settings show the user, not what
//! the server stores; mapping to server fields is a separate step. See
//! `Documentation/local-content-principles.md`.

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use uuid::Uuid;
use wp_api::{
    date::WpGmtDateTime,
    media::MediaId,
    posts::{PostFormat, PostId},
    terms::TermId,
    users::UserId,
};
use wp_mobile_sqlite::ColumnIndex;

use crate::db_types::working_copy_target::WorkingCopyTarget;

uniffi::custom_newtype!(WorkingCopyId, String);

/// Identifies a working copy: a hyphenated, lowercase v4 UUID.
///
/// Unlike the table's `rowid`, it stays unique across databases.
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

/// What the user wants to happen to the post when it's pushed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum StatusSetting {
    Draft,
    PendingReview,
    Publish,
}

/// Who can see the post once it's published.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum Visibility {
    Public,
    Private,
    PasswordProtected { password: String },
}

/// When the post is published.
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
pub struct WorkingCopyCommon {
    pub title: String,
    pub content: String,
    pub excerpt: String,
    /// `None` when the user hasn't set a slug.
    pub slug: Option<String>,
    pub status: StatusSetting,
    pub visibility: Visibility,
    pub publish_date: PublishDate,
    /// `None` when the user hasn't chosen an author.
    pub author: Option<UserId>,
    pub featured_image: FeaturedImage,
}

/// A post as the user edits it on the device.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PostWorkingCopy {
    pub common: WorkingCopyCommon,
    pub categories: Vec<TermId>,
    /// Tag names as the user entered them, including tags that don't exist
    /// on the site yet.
    pub tags: Vec<String>,
    pub format: PostFormat,
    pub sticky: bool,
}

/// A page as the user edits it on the device.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PageWorkingCopy {
    pub common: WorkingCopyCommon,
    /// `None` for a top-level page.
    pub parent: Option<PostId>,
}

/// A post working copy as stored, with its ID, target and timestamps.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DbPostWorkingCopy {
    pub id: WorkingCopyId,
    pub working_copy: PostWorkingCopy,
    pub target: WorkingCopyTarget,
    /// ISO 8601 UTC timestamp of when the working copy was created.
    pub created_at: String,
    /// ISO 8601 UTC timestamp of the last save.
    pub updated_at: String,
}

/// A page working copy as stored, with its ID, target and timestamps.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DbPageWorkingCopy {
    pub id: WorkingCopyId,
    pub working_copy: PageWorkingCopy,
    pub target: WorkingCopyTarget,
    /// ISO 8601 UTC timestamp of when the working copy was created.
    pub created_at: String,
    /// ISO 8601 UTC timestamp of the last save.
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
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::PendingReview => "pending_review",
            Self::Publish => "publish",
        }
    }
}

impl ToSql for StatusSetting {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_str()))
    }
}

impl FromSql for StatusSetting {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str()? {
            "draft" => Ok(Self::Draft),
            "pending_review" => Ok(Self::PendingReview),
            "publish" => Ok(Self::Publish),
            other => Err(FromSqlError::Other(
                format!("Unknown status_setting: {other}").into(),
            )),
        }
    }
}

impl Visibility {
    /// The `visibility` and `password` column values.
    pub(crate) fn to_columns(&self) -> (&'static str, Option<&str>) {
        match self {
            Self::Public => ("public", None),
            Self::Private => ("private", None),
            Self::PasswordProtected { password } => ("password_protected", Some(password)),
        }
    }

    pub(crate) fn from_columns(
        visibility: &str,
        password: Option<String>,
    ) -> Result<Self, FromSqlError> {
        match (visibility, password) {
            ("public", None) => Ok(Self::Public),
            ("private", None) => Ok(Self::Private),
            ("password_protected", Some(password)) => Ok(Self::PasswordProtected { password }),
            (visibility, password) => Err(FromSqlError::Other(
                format!(
                    "Inconsistent visibility columns: visibility={visibility}, has_password={}",
                    password.is_some()
                )
                .into(),
            )),
        }
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
    StatusSetting = 7,
    Visibility = 8,
    Password = 9,
    PublishDate = 10,
    AuthorId = 11,
    FeaturedMediaId = 12,
    CreatedAt = 13,
    UpdatedAt = 14,
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
    TagNames = 2,
    Format = 3,
    Sticky = 4,
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
