use crate::{RowId, SqliteDbError, repository::QueryExecutor};
use rusqlite::OptionalExtension;
use wp_api::wp_com::me::WpComUserId;

/// Who Reader data in the cache belongs to: a WordPress.com account, or the reader who isn't signed
/// in. The Reader works without an account, so its posts, streams and followed tags need an owner
/// then too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum ReaderOwner {
    Account {
        user_id: WpComUserId,
    },
    /// The reader who isn't signed in to WordPress.com. A cache has one.
    SignedOut,
}

/// The `user_id` of the row that owns the signed-out reader's data. WordPress uses `0` for "no
/// user", and no account has it.
const SIGNED_OUT_USER_ID: WpComUserId = WpComUserId(0);

impl ReaderOwner {
    fn user_id(&self) -> WpComUserId {
        match self {
            Self::Account { user_id } => *user_id,
            Self::SignedOut => SIGNED_OUT_USER_ID,
        }
    }
}

/// WordPress.com accounts. Account-level data, such as the Reader, references an account's row and
/// is deleted with it. The signed-out reader has a row here too: see [`ReaderOwner`].
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

    /// The row that owns a reader's data, created if it doesn't exist yet.
    pub fn get_or_create_owner(
        executor: &impl QueryExecutor,
        owner: ReaderOwner,
    ) -> Result<RowId, SqliteDbError> {
        Self::get_or_create(executor, owner.user_id())
    }

    /// The row that owns a reader's data, if anything has been cached for them.
    pub fn select_owner(
        executor: &impl QueryExecutor,
        owner: ReaderOwner,
    ) -> Result<Option<RowId>, SqliteDbError> {
        Self::select(executor, owner.user_id())
    }

    /// Deletes everything cached for a reader. Returns whether there was anything.
    pub fn delete_owner(
        executor: &impl QueryExecutor,
        owner: ReaderOwner,
    ) -> Result<bool, SqliteDbError> {
        Self::delete(executor, owner.user_id())
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
    fn test_the_signed_out_reader_has_a_row_of_its_own() {
        let conn = conn();
        let account = ReaderOwner::Account {
            user_id: WpComUserId(7),
        };
        let signed_out =
            WordPressComAccountRepository::get_or_create_owner(&conn, ReaderOwner::SignedOut)
                .unwrap();
        let again =
            WordPressComAccountRepository::get_or_create_owner(&conn, ReaderOwner::SignedOut)
                .unwrap();
        let account_row =
            WordPressComAccountRepository::get_or_create_owner(&conn, account).unwrap();
        assert_eq!(signed_out, again);
        assert_ne!(signed_out, account_row);
        assert_eq!(
            WordPressComAccountRepository::get_or_create(&conn, WpComUserId(7)).unwrap(),
            account_row,
            "An account's owner row is the account's row"
        );
    }

    #[test]
    fn test_removing_an_account_leaves_the_signed_out_reader() {
        let conn = conn();
        let account = ReaderOwner::Account {
            user_id: WpComUserId(7),
        };
        let signed_out =
            WordPressComAccountRepository::get_or_create_owner(&conn, ReaderOwner::SignedOut)
                .unwrap();
        WordPressComAccountRepository::get_or_create_owner(&conn, account).unwrap();

        assert!(WordPressComAccountRepository::delete_owner(&conn, account).unwrap());
        assert_eq!(
            WordPressComAccountRepository::select_owner(&conn, ReaderOwner::SignedOut).unwrap(),
            Some(signed_out)
        );
        assert_eq!(
            WordPressComAccountRepository::select_owner(&conn, account).unwrap(),
            None
        );

        assert!(
            WordPressComAccountRepository::delete_owner(&conn, ReaderOwner::SignedOut).unwrap()
        );
        assert!(
            !WordPressComAccountRepository::delete_owner(&conn, ReaderOwner::SignedOut).unwrap()
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
