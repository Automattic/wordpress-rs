use rusqlite::{Connection, OptionalExtension, Row};
use wp_mobile_sqlite::RowExt;

use crate::{
    LocalContentError,
    db_types::post_working_copy::{
        DbPostWorkingCopy, PostWorkingCopy, PostWorkingCopyColumn, PostWorkingCopyId,
    },
};

const TABLE_NAME: &str = "post_working_copies";

/// Repository for the post_working_copies table.
pub struct PostWorkingCopyRepository;

impl PostWorkingCopyRepository {
    /// Insert a new working copy and return its ID.
    pub fn insert(
        &self,
        conn: &Connection,
        working_copy: &PostWorkingCopy,
    ) -> Result<PostWorkingCopyId, LocalContentError> {
        let sql =
            format!("INSERT INTO {TABLE_NAME} (title, content) VALUES (?, ?) RETURNING rowid");
        let id = conn.query_row(
            &sql,
            rusqlite::params![working_copy.title, working_copy.content],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    /// Replace the fields of an existing working copy and refresh its `updated_at`.
    ///
    /// Returns `false` if no working copy has the given ID.
    pub fn update(
        &self,
        conn: &Connection,
        id: PostWorkingCopyId,
        working_copy: &PostWorkingCopy,
    ) -> Result<bool, LocalContentError> {
        let sql = format!(
            "UPDATE {TABLE_NAME} SET
             title = ?,
             content = ?,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE rowid = ?"
        );
        let updated = conn.execute(
            &sql,
            rusqlite::params![working_copy.title, working_copy.content, id],
        )?;
        Ok(updated > 0)
    }

    /// Select a working copy by its ID.
    ///
    /// Returns `Ok(None)` if no working copy has the given ID.
    pub fn select_by_id(
        &self,
        conn: &Connection,
        id: PostWorkingCopyId,
    ) -> Result<Option<DbPostWorkingCopy>, LocalContentError> {
        let sql = format!("SELECT * FROM {TABLE_NAME} WHERE rowid = ?");
        let db_working_copy = conn.query_row(&sql, [id], from_row).optional()?;
        Ok(db_working_copy)
    }
}

fn from_row(row: &Row) -> rusqlite::Result<DbPostWorkingCopy> {
    use PostWorkingCopyColumn::*;

    Ok(DbPostWorkingCopy {
        id: row.get_column(Rowid)?,
        working_copy: PostWorkingCopy {
            title: row.get_column(Title)?,
            content: row.get_column(Content)?,
        },
        created_at: row.get_column(CreatedAt)?,
        updated_at: row.get_column(UpdatedAt)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MIGRATIONS;
    use wp_mobile_sqlite::{ColumnIndex, MigrationManager};

    fn migrated_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory database");
        MigrationManager::new(&conn, &MIGRATIONS)
            .expect("Failed to create MigrationManager")
            .perform_migrations()
            .expect("Migrations should succeed");
        conn
    }

    fn working_copy(title: &str, content: &str) -> PostWorkingCopy {
        PostWorkingCopy {
            title: title.to_string(),
            content: content.to_string(),
        }
    }

    /// Verify that PostWorkingCopyColumn values match the actual database schema.
    #[test]
    fn test_post_working_copy_column_enum_matches_schema() {
        use PostWorkingCopyColumn::*;

        let conn = migrated_connection();
        let columns: Vec<String> = conn
            .prepare(&format!("PRAGMA table_info({TABLE_NAME})"))
            .expect("Failed to prepare PRAGMA query")
            .query_map([], |row| row.get::<_, String>(1))
            .expect("Failed to execute PRAGMA query")
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to collect column names");

        assert_eq!(columns[Rowid.as_index()], "rowid");
        assert_eq!(columns[Title.as_index()], "title");
        assert_eq!(columns[Content.as_index()], "content");
        assert_eq!(columns[CreatedAt.as_index()], "created_at");
        assert_eq!(columns[UpdatedAt.as_index()], "updated_at");
        assert_eq!(columns.len(), 5, "Expected 5 columns in {TABLE_NAME} table");
    }

    #[test]
    fn test_insert_then_select_returns_the_saved_fields() {
        let conn = migrated_connection();
        let repo = PostWorkingCopyRepository;
        let saved = working_copy(
            "Hello",
            "<!-- wp:paragraph --><p>World</p><!-- /wp:paragraph -->",
        );

        let id = repo.insert(&conn, &saved).expect("Insert should succeed");
        let selected = repo
            .select_by_id(&conn, id)
            .expect("Select should succeed")
            .expect("Inserted working copy should exist");

        assert_eq!(selected.id, id);
        assert_eq!(selected.working_copy, saved);
        assert_eq!(selected.created_at, selected.updated_at);
    }

    #[test]
    fn test_update_replaces_fields_and_keeps_created_at() {
        let conn = migrated_connection();
        let repo = PostWorkingCopyRepository;
        let id = repo
            .insert(&conn, &working_copy("First", "first content"))
            .expect("Insert should succeed");
        let before = repo
            .select_by_id(&conn, id)
            .expect("Select should succeed")
            .expect("Inserted working copy should exist");

        let edited = working_copy("Second", "second content");
        let updated = repo
            .update(&conn, id, &edited)
            .expect("Update should succeed");
        let after = repo
            .select_by_id(&conn, id)
            .expect("Select should succeed")
            .expect("Updated working copy should exist");

        assert!(updated, "Update should report the existing row");
        assert_eq!(after.working_copy, edited);
        assert_eq!(after.created_at, before.created_at);
        assert!(
            after.updated_at >= before.updated_at,
            "updated_at should not move backwards: before={}, after={}",
            before.updated_at,
            after.updated_at
        );
    }

    #[test]
    fn test_update_returns_false_for_unknown_id() {
        let conn = migrated_connection();
        let updated = PostWorkingCopyRepository
            .update(
                &conn,
                PostWorkingCopyId(42),
                &working_copy("Title", "content"),
            )
            .expect("Update should succeed");
        assert!(!updated);
    }

    #[test]
    fn test_select_returns_none_for_unknown_id() {
        let conn = migrated_connection();
        let selected = PostWorkingCopyRepository
            .select_by_id(&conn, PostWorkingCopyId(42))
            .expect("Select should succeed");
        assert_eq!(selected, None);
    }
}
