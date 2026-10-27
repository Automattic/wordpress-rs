//! Working copy types.
//!
//! Fields hold what the editor and its settings work with, shaped so that every
//! value the server stores in them fits and converting a remote post loses none
//! of them. See `Documentation/local-content-principles.md`.

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wp_api::{
    date::WpGmtDateTime,
    media::MediaId,
    posts::{PostCommentStatus, PostFormat, PostId, PostPingStatus},
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

/// The post's status, with its password where the status allows one.
///
/// A scheduled post is `Publish` with a future `PublishDate`, as WordPress
/// itself treats it. A private post has no password: WordPress clears it when
/// saving one. A `password` of `None` means the post has no password.
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
    /// A status registered by a plugin.
    Custom {
        status: String,
        password: Option<String>,
    },
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

/// A tag on a post.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum Tag {
    /// A tag that exists on the site.
    Existing {
        #[serde(with = "term_id")]
        id: TermId,
    },
    /// A tag the user entered that the site doesn't have yet, or that hasn't
    /// been matched to an existing tag.
    New { name: String },
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
    pub publish_date: PublishDate,
    /// `None` when the user hasn't chosen an author.
    pub author: Option<UserId>,
    pub featured_image: FeaturedImage,
    /// `None` when the user hasn't chosen whether comments are open.
    pub comment_status: Option<PostCommentStatus>,
    /// `None` when the user hasn't chosen whether pings are open.
    pub ping_status: Option<PostPingStatus>,
}

/// A post as the user edits it on the device.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PostWorkingCopy {
    pub common: WorkingCopyCommon,
    pub categories: Vec<TermId>,
    pub tags: Vec<Tag>,
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
    /// The `status` and `password` column values. The status is stored as
    /// WordPress names it.
    pub(crate) fn to_columns(&self) -> (&str, Option<&str>) {
        match self {
            Self::Draft { password } => ("draft", password.as_deref()),
            Self::PendingReview { password } => ("pending", password.as_deref()),
            Self::Publish { password } => ("publish", password.as_deref()),
            Self::Private => ("private", None),
            Self::Custom { status, password } => (status, password.as_deref()),
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

/// Serializes a `TermId` as its bare number in the stored tags JSON.
mod term_id {
    use serde::{Deserialize, Deserializer, Serializer};
    use wp_api::terms::TermId;

    pub fn serialize<S: Serializer>(id: &TermId, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i64(id.0)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<TermId, D::Error> {
        i64::deserialize(deserializer).map(TermId)
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
    Tags = 2,
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
