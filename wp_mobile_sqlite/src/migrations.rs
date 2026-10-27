use rusqlite::{Connection, Result as SqliteResult, params};

use crate::connection::{ConnectionError, prepare_connection};

/// Runs a database's migrations in order, recording each applied migration in
/// a `_migrations` table.
///
/// `migrations` holds one entry per migration, each a `;`-separated list of
/// SQL statements. A migration's ID is its 1-based position in the list, so
/// entries can only ever be appended.
///
/// Every pending migration runs in a single `BEGIN EXCLUSIVE` transaction with
/// foreign key enforcement on, and a `PRAGMA foreign_keys` in a migration has
/// no effect inside it.
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

    pub fn perform_migrations(&mut self) -> SqliteResult<i64> {
        self.connection.execute("BEGIN EXCLUSIVE", ())?;

        match self.migrate() {
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
