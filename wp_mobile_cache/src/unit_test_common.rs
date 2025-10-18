#[cfg(test)]
use crate::MigrationManager;
#[cfg(test)]
use rusqlite::Connection;

/// Creates an in-memory test database with all migrations applied.
///
/// This runs ALL migrations to ensure tests don't break when later migrations
/// alter existing tables (e.g., ALTER TABLE statements).
///
/// Also inserts a test site with id = 1 to satisfy foreign key constraints.
#[cfg(test)]
pub fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    let mut migration_manager = MigrationManager::new(&conn).unwrap();

    migration_manager
        .perform_migrations()
        .expect("All migrations should succeed");

    // Insert a test site to satisfy foreign key constraints
    conn.execute("INSERT INTO sites (id) VALUES (1)", [])
        .expect("Failed to insert test site");

    conn
}
