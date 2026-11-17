use crate::{RowId, SqliteDbError, repository::QueryExecutor};
use rusqlite::OptionalExtension;
use wp_api::wp_com::me::WpComUserId;

/// WordPress.com accounts. Account-level data, such as the Reader, references an account's row and
/// is deleted with it.
pub struct WordPressComAccountRepository;

impl WordPressComAccountRepository {
    /// The account's row, created if it doesn't exist yet.
    pub fn get_or_create(
        executor: &impl QueryExecutor,
        user_id: WpComUserId,
    ) -> Result<RowId, SqliteDbError> {
        executor.execute(
            "INSERT INTO wordpress_com_accounts (user_id) VALUES (?1) ON CONFLICT(user_id) DO NOTHING",
            [user_id.0],
        )?;
        Self::select(executor, user_id)?.ok_or_else(|| {
            SqliteDbError::SqliteError(format!("Account {} wasn't stored", user_id.0))
        })
    }

    /// The account's row, if anything has been cached for it.
    pub fn select(
        executor: &impl QueryExecutor,
        user_id: WpComUserId,
    ) -> Result<Option<RowId>, SqliteDbError> {
        let mut statement =
            executor.prepare("SELECT rowid FROM wordpress_com_accounts WHERE user_id = ?1")?;
        Ok(statement
            .query_row([user_id.0], |row| row.get(0))
            .optional()?)
    }

    /// Deletes the account and, through `ON DELETE CASCADE`, everything cached for it. Returns
    /// whether it existed.
    pub fn delete(
        executor: &impl QueryExecutor,
        user_id: WpComUserId,
    ) -> Result<bool, SqliteDbError> {
        let deleted = executor.execute(
            "DELETE FROM wordpress_com_accounts WHERE user_id = ?1",
            [user_id.0],
        )?;
        Ok(deleted > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MigrationManager;
    use rusqlite::Connection;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        MigrationManager::new(&conn)
            .unwrap()
            .perform_migrations()
            .unwrap();
        conn
    }

    #[test]
    fn test_get_or_create_is_idempotent() {
        let conn = conn();
        let first = WordPressComAccountRepository::get_or_create(&conn, WpComUserId(7)).unwrap();
        let second = WordPressComAccountRepository::get_or_create(&conn, WpComUserId(7)).unwrap();
        let other = WordPressComAccountRepository::get_or_create(&conn, WpComUserId(8)).unwrap();
        assert_eq!(first, second);
        assert_ne!(first, other);
    }

    #[test]
    fn test_delete_reports_whether_the_account_existed() {
        let conn = conn();
        WordPressComAccountRepository::get_or_create(&conn, WpComUserId(7)).unwrap();
        assert!(WordPressComAccountRepository::delete(&conn, WpComUserId(7)).unwrap());
        assert!(!WordPressComAccountRepository::delete(&conn, WpComUserId(7)).unwrap());
        assert_eq!(
            WordPressComAccountRepository::select(&conn, WpComUserId(7)).unwrap(),
            None
        );
    }

    #[test]
    fn test_a_deleted_account_gets_a_new_row() {
        let conn = conn();
        let first = WordPressComAccountRepository::get_or_create(&conn, WpComUserId(7)).unwrap();
        WordPressComAccountRepository::delete(&conn, WpComUserId(7)).unwrap();
        let second = WordPressComAccountRepository::get_or_create(&conn, WpComUserId(7)).unwrap();
        assert_ne!(
            first, second,
            "A service still holding the old row must not write into the new account"
        );
    }
}
