use crate::{
    DbTable, RowId, SqliteDbError,
    context::{EditContext, IsContext},
    db_types::{
        db_site::DbSite,
        post_types::{DbPostTypeDetailsWithEditContext, PostTypeEditContextColumn},
        row_ext::RowExt,
    },
    entity::{EntityId, FullEntity},
    repository::QueryExecutor,
};
use rusqlite::{OptionalExtension, Row};
use std::{marker::PhantomData, sync::Arc};
use wp_api::post_types::PostTypeDetailsWithEditContext;

/// Entity-specific context trait for PostTypes.
///
/// Associates a context with post-type-specific types and provides database row mapping.
pub trait PostTypeContext: IsContext {
    /// The context-specific post type entity type (e.g., PostTypeDetailsWithEditContext)
    type PostTypeDetails: serde::Serialize;

    /// The context-specific database wrapper type (e.g., DbPostTypeDetailsWithEditContext)
    type DbPostTypeDetails;

    /// Get the database table for this context
    fn table() -> DbTable;

    /// Construct DbPostTypeDetails from a database row
    fn from_row(row: &Row) -> Result<Self::DbPostTypeDetails, SqliteDbError>;

    /// Extract the rowid from DbPostTypeDetails (for EntityId creation)
    fn rowid(db_post_type: &Self::DbPostTypeDetails) -> RowId;
}

impl PostTypeContext for EditContext {
    type PostTypeDetails = PostTypeDetailsWithEditContext;
    type DbPostTypeDetails = DbPostTypeDetailsWithEditContext;

    fn table() -> DbTable {
        DbTable::PostTypesEditContext
    }

    fn from_row(row: &Row) -> Result<Self::DbPostTypeDetails, SqliteDbError> {
        use PostTypeEditContextColumn::*;

        let row_id: RowId = row.get_column(Rowid)?;
        let db_site_id: RowId = row.get_column(DbSiteId)?;
        let slug: String = row.get_column(Slug)?;
        let data_json: String = row.get_column(Data)?;
        let last_fetched_at: String = row.get_column(LastFetchedAt)?;

        // Deserialize the JSON data into PostTypeDetailsWithEditContext
        let post_type: PostTypeDetailsWithEditContext = serde_json::from_str(&data_json)
            .map_err(|e| SqliteDbError::SqliteError(format!("Failed to parse JSON: {}", e)))?;

        Ok(DbPostTypeDetailsWithEditContext {
            row_id,
            db_site_id,
            slug,
            post_type,
            last_fetched_at,
        })
    }

    fn rowid(db_post_type: &Self::DbPostTypeDetails) -> RowId {
        db_post_type.row_id
    }
}

/// Repository for managing post types in the database.
///
/// Generic over PostTypeContext trait to support different contexts (currently just edit).
///
/// # Type Parameters
/// * `C` - The context type (EditContext)
pub struct PostTypeRepository<C: PostTypeContext> {
    _phantom: PhantomData<C>,
}

impl<C: PostTypeContext> Default for PostTypeRepository<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: PostTypeContext> PostTypeRepository<C> {
    /// Create a new repository instance.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }

    /// Get the full table name for this context.
    pub fn table_name() -> &'static str {
        C::table().table_name()
    }

    /// Upsert a post type to the database.
    ///
    /// Inserts or updates the post type based on (db_site_id, slug) uniqueness constraint.
    ///
    /// # Returns
    /// EntityId of the upserted post type
    pub fn upsert(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        slug: &str,
        post_type: &C::PostTypeDetails,
    ) -> Result<EntityId, SqliteDbError> {
        // Serialize the post type to JSON
        let data_json = serde_json::to_string(post_type).map_err(|e| {
            SqliteDbError::SqliteError(format!("Failed to serialize to JSON: {}", e))
        })?;

        let sql = format!(
            "INSERT INTO {} (db_site_id, slug, data) VALUES (?, ?, ?)
             ON CONFLICT(db_site_id, slug) DO UPDATE SET
             data = excluded.data,
             last_fetched_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             RETURNING rowid",
            Self::table_name()
        );

        let mut stmt = executor.prepare(&sql)?;
        let row_id: RowId = stmt
            .query_row(rusqlite::params![site.row_id.0, slug, data_json], |row| {
                row.get(0)
            })?;

        Ok(EntityId::new(*site, C::table(), row_id))
    }

    /// Select a post type by its slug.
    ///
    /// Returns `Ok(None)` if no post type with the given slug exists.
    pub fn select_by_slug(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        slug: &str,
    ) -> Result<Option<FullEntity<C::DbPostTypeDetails>>, SqliteDbError> {
        let sql = format!(
            "SELECT * FROM {} WHERE db_site_id = ? AND slug = ?",
            Self::table_name()
        );

        let mut stmt = executor.prepare(&sql)?;
        let db_post_type = stmt
            .query_row(rusqlite::params![site.row_id.0, slug], |row| {
                C::from_row(row).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
            })
            .optional()
            .map_err(SqliteDbError::from)?;

        Ok(db_post_type.map(|db_post_type| {
            let row_id = C::rowid(&db_post_type);
            let entity_id = Arc::new(EntityId::new(*site, C::table(), row_id));
            FullEntity::new(entity_id, db_post_type)
        }))
    }

    /// Select all post types for a given site.
    ///
    /// Returns an empty vector if no post types exist for the site.
    pub fn select_all(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
    ) -> Result<Vec<FullEntity<C::DbPostTypeDetails>>, SqliteDbError> {
        let sql = format!(
            "SELECT * FROM {} WHERE db_site_id = ? ORDER BY slug",
            Self::table_name()
        );

        let mut stmt = executor.prepare(&sql)?;
        let rows = stmt.query_map([site.row_id.0], |row| {
            C::from_row(row).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        rows.map(|row_result| {
            let db_post_type = row_result?;
            let row_id = C::rowid(&db_post_type);
            let entity_id = Arc::new(EntityId::new(*site, C::table(), row_id));
            Ok(FullEntity::new(entity_id, db_post_type))
        })
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db_types::row_ext::ColumnIndex, test_fixtures::test_ctx};
    use rstest::*;

    /// Verify that PostTypeEditContextColumn enum values match the actual database schema.
    ///
    /// This test ensures the column enum stays synchronized with the SQL schema.
    /// If columns are added, removed, or reordered, this test will fail.
    #[rstest]
    fn test_post_type_edit_context_column_enum_matches_schema(
        test_ctx: crate::test_fixtures::TestContext,
    ) {
        use PostTypeEditContextColumn::*;

        let columns =
            crate::test_fixtures::get_table_column_names(&test_ctx.conn, "post_types_edit_context");

        // Verify each enum value maps to the correct column name
        assert_eq!(columns[Rowid.as_index()], "rowid");
        assert_eq!(columns[DbSiteId.as_index()], "db_site_id");
        assert_eq!(columns[Slug.as_index()], "slug");
        assert_eq!(columns[Data.as_index()], "data");
        assert_eq!(columns[LastFetchedAt.as_index()], "last_fetched_at");

        // Verify we have exactly the expected number of columns
        assert_eq!(
            columns.len(),
            5,
            "Expected 5 columns in post_types_edit_context table"
        );
    }
}
