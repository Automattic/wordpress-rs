use crate::RowId;
use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput};

/// Type of WordPress site stored in the database.
///
/// Uses integer representation in the database for performance.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[repr(i64)]
pub enum DbSiteType {
    SelfHosted = 0,
    WordPressCom = 1,
}

impl ToSql for DbSiteType {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(*self as i64))
    }
}

impl FromSql for DbSiteType {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> FromSqlResult<Self> {
        match i64::column_result(value)? {
            0 => Ok(DbSiteType::SelfHosted),
            1 => Ok(DbSiteType::WordPressCom),
            other => Err(rusqlite::types::FromSqlError::OutOfRange(other)),
        }
    }
}

/// Represents a cached WordPress site in the database.
///
/// # Design Rationale
///
/// This is intentionally a database-specific type (hence the `Db` prefix) rather than
/// a domain type representing a WordPress site. This design choice prevents confusion:
///
/// - **Not a WordPress.com site ID**: The `row_id` has no relationship to WordPress.com site IDs
/// - **Not a self-hosted site identifier**: Self-hosted sites don't have numeric IDs
/// - **Internal cache identifier only**: This ID exists only for our local database's multi-site support
///
/// # Site Type Mapping
///
/// The `site_type` field indicates which type-specific table contains additional data:
/// - `DbSiteType::SelfHosted` → `mapped_site_id` references `self_hosted_sites` table
/// - `DbSiteType::WordPressCom` → `mapped_site_id` references `wordpress_com_sites` table (future)
///
/// Note: `mapped_site_id` is a reference column, not a foreign key constraint, since it can
/// point to different tables based on `site_type`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Record,
)]
pub struct DbSite {
    pub row_id: RowId,
    pub site_type: DbSiteType,
    pub mapped_site_id: RowId,
}
