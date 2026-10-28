use rusqlite::{Connection, OptionalExtension, Result as SqliteResult, ffi, params};

use crate::connection::{ConnectionError, enable_foreign_keys, prepare_connection};

/// Runs a database's migrations in order, recording each applied migration in
/// a `_migrations` table.
///
/// `migrations` holds one entry per migration, each a `;`-separated list of
/// SQL statements. A migration's ID is its 1-based position in the list, so
/// entries can only ever be appended.
///
/// Every pending migration runs in a single `BEGIN EXCLUSIVE` transaction, and
/// a `PRAGMA foreign_keys` in a migration has no effect inside it. Whether
/// foreign keys are enforced while they run depends on the method running them.
pub struct MigrationManager<'a> {
    connection: &'a Connection,
    migrations: &'static [&'static str],
}

impl<'a> MigrationManager<'a> {
    pub fn new(
        connection: &'a Connection,
        migrations: &'static [&'static str],
    ) -> Result<Self, ConnectionError> {
        // Cover raw-connection paths that bypass `DbConnection` (test fixtures
        // that hand a Connection straight to the repository layer).
        // `prepare_connection` is idempotent, so connections that were already
        // prepared are unaffected.
        prepare_connection(connection)?;
        Ok(Self {
            connection,
            migrations,
        })
    }

    /// Run the pending migrations with foreign key enforcement on, so foreign
    /// key actions run as the migrations change rows: dropping a table deletes
    /// the rows that reference it with `ON DELETE CASCADE`.
    pub fn perform_migrations(&mut self) -> SqliteResult<i64> {
        self.migrate_in_transaction(false)
    }

    /// Run the pending migrations the way SQLite documents for schema changes:
    /// with foreign key enforcement off, checking every foreign key before
    /// committing.
    ///
    /// Dropping a table leaves the rows that reference it alone, so a migration
    /// can rebuild a referenced table: create the new table, copy the rows
    /// into it, drop the old one, then rename the new one to the old name. If
    /// the migrations leave a row referencing one that doesn't exist, they're
    /// rolled back and a `FOREIGN KEY` constraint error is returned.
    /// Enforcement is turned back on whether or not they succeed.
    pub fn perform_migrations_with_foreign_keys_off(&mut self) -> Result<i64, ConnectionError> {
        // Set before the migrations' transaction opens: inside one it has no effect.
        self.connection.pragma_update(None, "foreign_keys", false)?;
        let result = self.migrate_in_transaction(true);
        enable_foreign_keys(self.connection)?;
        Ok(result?)
    }

    fn migrate_in_transaction(&mut self, check_foreign_keys: bool) -> SqliteResult<i64> {
        self.connection.execute("BEGIN EXCLUSIVE", ())?;

        let result = self.migrate().and_then(|count| {
            if check_foreign_keys {
                self.check_foreign_keys()?;
            }
            Ok(count)
        });
        match result {
            Ok(count) => {
                self.connection.execute("COMMIT", ())?;
                Ok(count)
            }
            Err(e) => {
                let _ = self.connection.execute("ROLLBACK", ());
                Err(e)
            }
        }
    }

    /// Fails with a `FOREIGN KEY` constraint error naming the first violation
    /// `PRAGMA foreign_key_check` reports.
    fn check_foreign_keys(&self) -> SqliteResult<()> {
        let violation = self
            .connection
            .query_row("PRAGMA foreign_key_check", [], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(2)?))
            })
            .optional()?;
        match violation {
            None => Ok(()),
            Some((table, parent)) => Err(rusqlite::Error::SqliteFailure(
                ffi::Error::new(ffi::SQLITE_CONSTRAINT_FOREIGNKEY),
                Some(format!(
                    "Migrations left a row in {table} referencing a missing row in {parent}"
                )),
            )),
        }
    }

    fn migrate(&mut self) -> SqliteResult<i64> {
        if !self.has_migrations_table()? {
            self.create_migrations_table()?;
        }

        let next_migration_id = self.get_next_migration_index()?;
        if next_migration_id > self.migrations.len() {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "next migration index ({next_migration_id}) exceeds available migrations ({})",
                self.migrations.len()
            )));
        }
        for (index, migration) in self.migrations[next_migration_id..].iter().enumerate() {
            for query in migration
                .split(";")
                .filter(|query| !query.trim().is_empty())
            {
                self.connection.execute(query, ())?;
            }

            // `.enumerate` will start the indexes from 0, so we need to add `next_migration_id`
            self.insert_migration((next_migration_id + index + 1) as i64)?;
        }

        Ok(self.migrations[next_migration_id..].len() as i64)
    }

    fn has_migrations_table(&self) -> SqliteResult<bool> {
        let mut statement: rusqlite::Statement<'_> = self.connection.prepare(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_migrations'",
        )?;

        let result = statement.query_row([], |row| row.get::<_, i32>(0))?;

        Ok(result > 0)
    }

    fn create_migrations_table(&self) -> SqliteResult<()> {
        self.connection.execute(
            "CREATE TABLE _migrations (migration_id INTEGER PRIMARY KEY)",
            (),
        )?;
        Ok(())
    }

    fn insert_migration(&mut self, migration_id: i64) -> SqliteResult<()> {
        let mut insert_migration_query = self
            .connection
            .prepare("INSERT INTO _migrations (migration_id) VALUES (?)")?;
        insert_migration_query.execute(params![migration_id])?;
        Ok(())
    }

    /// Returns the index of the next migration to run in `migrations`.
    /// Note that this is *not* the same as the migration ID.
    fn get_next_migration_index(&self) -> SqliteResult<usize> {
        let mut statement = self
            .connection
            .prepare("SELECT MAX(migration_id) FROM _migrations")?;
        let result = statement.query_row([], |row| row.get::<_, Option<usize>>(0))?;
        Ok(result.unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DbConnection;
    use crate::connection::tests::{connection_without_foreign_keys, foreign_keys_enabled};

    static TEST_MIGRATIONS: [&str; 3] = [
        "CREATE TABLE first (id INTEGER PRIMARY KEY);",
        "CREATE TABLE second (id INTEGER PRIMARY KEY); CREATE TABLE third (id INTEGER PRIMARY KEY);",
        "ALTER TABLE first ADD COLUMN name TEXT;",
    ];

    /// A `parent` table and a `child` table referencing it with `ON DELETE CASCADE`.
    const PARENT_AND_CHILD: &str = "CREATE TABLE parent (id INTEGER PRIMARY KEY);
         CREATE TABLE child (parent_id INTEGER NOT NULL REFERENCES parent(id) ON DELETE CASCADE);";

    /// Runs the first of `migrations`, adds a parent with two children, then
    /// runs the rest with foreign keys off.
    fn migrate_with_children(
        migrations: &'static [&'static str],
    ) -> (Connection, Result<i64, ConnectionError>) {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory database");
        MigrationManager::new(&conn, &migrations[..1])
            .expect("MigrationManager creation should succeed")
            .perform_migrations_with_foreign_keys_off()
            .expect("The first migration should succeed");
        conn.execute_batch(
            "INSERT INTO parent (id) VALUES (1); INSERT INTO child VALUES (1), (1);",
        )
        .expect("Inserting rows should succeed");
        let result = MigrationManager::new(&conn, migrations)
            .expect("MigrationManager creation should succeed")
            .perform_migrations_with_foreign_keys_off();
        (conn, result)
    }

    fn count_rows(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("Counting rows should succeed")
    }

    #[test]
    fn test_migration_manager_enables_foreign_keys_on_a_connection_that_had_them_off() {
        let conn = connection_without_foreign_keys();
        MigrationManager::new(&conn, &TEST_MIGRATIONS)
            .expect("MigrationManager creation should succeed");
        assert!(
            foreign_keys_enabled(&conn),
            "MigrationManager::new must enable foreign key enforcement so that raw \
             connections handed straight to the repository layer are also covered"
        );
    }

    #[test]
    fn test_migration_works() {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory database");
        let migrations_run = MigrationManager::new(&conn, &TEST_MIGRATIONS)
            .expect("MigrationManager creation should succeed")
            .perform_migrations()
            .expect("Migrations should succeed");
        assert_eq!(migrations_run, TEST_MIGRATIONS.len() as i64);
    }

    #[test]
    fn test_migration_numbering_should_be_sequential() {
        let connection = Connection::open_in_memory().unwrap();
        let mut migration_manager = MigrationManager::new(&connection, &TEST_MIGRATIONS).unwrap();

        // Create migrations table and run first migration manually
        migration_manager.create_migrations_table().unwrap();
        connection.execute(TEST_MIGRATIONS[0], ()).unwrap();
        migration_manager.insert_migration(1).unwrap();

        migration_manager
            .perform_migrations()
            .expect("Migrations should succeed");

        // Verify migration IDs are sequential
        let mut stmt = connection
            .prepare("SELECT migration_id FROM _migrations ORDER BY migration_id")
            .unwrap();
        let migration_ids: Vec<i64> = stmt
            .query_map([], |row| row.get::<_, i64>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        // Verify migration IDs are sequential and complete
        let expected_ids: Vec<i64> = (1..=TEST_MIGRATIONS.len() as i64).collect();
        assert_eq!(
            migration_ids,
            expected_ids,
            "Migration IDs should be sequential from 1 to {}",
            TEST_MIGRATIONS.len()
        );
    }

    #[test]
    fn test_migration_returns_error_when_index_exceeds_available_migrations() {
        let connection = Connection::open_in_memory().unwrap();
        let mut migration_manager = MigrationManager::new(&connection, &TEST_MIGRATIONS).unwrap();

        migration_manager.create_migrations_table().unwrap();

        // Insert a migration ID beyond the number of available migrations
        let beyond_max = TEST_MIGRATIONS.len() as i64 + 1;
        migration_manager.insert_migration(beyond_max).unwrap();

        let result = migration_manager.perform_migrations();
        assert!(
            result.is_err(),
            "Expected error when migration index exceeds available migrations"
        );
    }

    #[test]
    fn test_migrations_with_foreign_keys_off_can_rebuild_a_referenced_table() {
        static REBUILD_PARENT: [&str; 2] = [
            PARENT_AND_CHILD,
            "CREATE TABLE new_parent (id INTEGER PRIMARY KEY, name TEXT);
             INSERT INTO new_parent (id) SELECT id FROM parent;
             DROP TABLE parent;
             ALTER TABLE new_parent RENAME TO parent;",
        ];

        let (conn, result) = migrate_with_children(&REBUILD_PARENT);

        assert_eq!(result.expect("Rebuilding the parent should succeed"), 1);
        assert_eq!(
            count_rows(&conn, "child"),
            2,
            "Rebuilding the parent must keep the rows referencing it"
        );
        assert!(foreign_keys_enabled(&conn), "Enforcement must be back on");
    }

    #[test]
    fn test_migrations_with_foreign_keys_off_are_rolled_back_when_they_leave_a_reference_dangling()
    {
        static DELETE_PARENTS: [&str; 2] = [PARENT_AND_CHILD, "DELETE FROM parent;"];

        let (conn, result) = migrate_with_children(&DELETE_PARENTS);
        let applied: i64 = conn
            .query_row("SELECT MAX(migration_id) FROM _migrations", [], |row| {
                row.get(0)
            })
            .expect("Reading the applied migrations should succeed");

        assert!(
            matches!(
                &result,
                Err(ConnectionError::Sqlite(rusqlite::Error::SqliteFailure(err, _)))
                    if err.extended_code == ffi::SQLITE_CONSTRAINT_FOREIGNKEY
            ),
            "Expected a foreign key error, got {result:?}"
        );
        assert_eq!(applied, 1, "The migration should be rolled back");
        assert_eq!(count_rows(&conn, "parent"), 1, "The parent should be back");
        assert!(foreign_keys_enabled(&conn), "Enforcement must be back on");
    }

    #[test]
    fn test_concurrent_migrations_race_condition() {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = dir.path().join("race_test.db");
        let db_path = db_path.to_str().expect("Temp path should be UTF-8");

        let num_tasks = 10;
        let results: Vec<(usize, SqliteResult<i64>)> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..num_tasks)
                .map(|task_id| {
                    scope.spawn(move || {
                        let db = DbConnection::open(Some(db_path))
                            .expect("Opening the database should succeed");
                        let result = db.execute(|conn| {
                            MigrationManager::new(conn, &TEST_MIGRATIONS)
                                .expect("MigrationManager creation should succeed")
                                .perform_migrations()
                        });
                        (task_id, result)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("Migration thread panicked"))
                .collect()
        });

        let errors: Vec<_> = results
            .iter()
            .filter_map(|(task_id, result)| result.as_ref().err().map(|e| (task_id, e)))
            .collect();
        assert!(errors.is_empty(), "Race condition detected: {errors:?}");

        let migrations_run_counts: Vec<i64> = results
            .iter()
            .filter_map(|(_, result)| result.as_ref().ok())
            .copied()
            .collect();
        let full_migration_count = migrations_run_counts
            .iter()
            .filter(|&&count| count == TEST_MIGRATIONS.len() as i64)
            .count();
        let zero_migration_count = migrations_run_counts
            .iter()
            .filter(|&&count| count == 0)
            .count();

        assert_eq!(
            full_migration_count,
            1,
            "Expected exactly one task to run all {} migrations, but {} tasks did",
            TEST_MIGRATIONS.len(),
            full_migration_count
        );
        assert_eq!(
            zero_migration_count,
            migrations_run_counts.len() - 1,
            "Expected {} tasks to run zero migrations, but {} tasks did",
            migrations_run_counts.len() - 1,
            zero_migration_count
        );

        let final_db = DbConnection::open(Some(db_path)).expect("Reopening should succeed");
        let migration_count = final_db.execute(|conn| {
            conn.query_row("SELECT COUNT(*) FROM _migrations", [], |row| {
                row.get::<_, i64>(0)
            })
        });
        assert_eq!(
            migration_count.expect("Counting migrations should succeed"),
            TEST_MIGRATIONS.len() as i64
        );
    }
}
