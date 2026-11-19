//! What the user turned on or off in the Reader that the server doesn't have yet, and the sites and
//! users they blocked.

use crate::{RowId, SqliteDbError, repository::QueryExecutor};
use rusqlite::{OptionalExtension, Row, params};
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::me::WpComUserId;

/// Something the user turned on or off that the server doesn't have yet. See the `reader_actions`
/// table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbReaderAction {
    pub row_id: RowId,
    /// Which action it is, as the Reader service numbers them.
    pub kind: i64,
    /// What it's about.
    pub target: String,
    /// What the user wants: on or off.
    pub wanted: bool,
    /// What the cache showed before the first tap that's still waiting, or `None` if it wasn't
    /// known.
    pub was: Option<bool>,
    /// The order of the last tap.
    pub seq: i64,
    /// What the request needs, as JSON.
    pub payload: String,
    /// How many times it's been sent without an answer.
    pub attempts: u32,
    pub last_error: Option<String>,
}

const ACTION_COLUMNS: &str = "rowid, kind, target, wanted, was, seq, payload, attempts, last_error";

fn action_from_row(row: &Row) -> rusqlite::Result<DbReaderAction> {
    Ok(DbReaderAction {
        row_id: row.get(0)?,
        kind: row.get(1)?,
        target: row.get(2)?,
        wanted: row.get(3)?,
        was: row.get(4)?,
        seq: row.get(5)?,
        payload: row.get(6)?,
        attempts: row.get(7)?,
        last_error: row.get(8)?,
    })
}

/// The actions waiting to be sent. A row is there from a tap until the server answers it.
pub struct ReaderActionRepository;

impl ReaderActionRepository {
    /// Records a tap: the user wants `target` on or off. A row already waiting for the same target
    /// takes the new answer and goes to the back of the line, and keeps what the cache showed
    /// before its first tap. `was` is what the cache shows now, for a new row. Returns the row.
    pub fn tap(
        executor: &impl QueryExecutor,
        account: RowId,
        kind: i64,
        target: &str,
        wanted: bool,
        was: Option<bool>,
        payload: &str,
    ) -> Result<DbReaderAction, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM reader_actions WHERE account_id = ?1",
        )?;
        let seq: i64 = statement.query_row([account], |row| row.get(0))?;
        executor.execute(
            "INSERT INTO reader_actions (account_id, kind, target, wanted, was, seq, payload)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(account_id, kind, target) DO UPDATE SET
                wanted = excluded.wanted,
                seq = excluded.seq,
                payload = excluded.payload,
                attempts = 0,
                last_error = NULL",
            params![account, kind, target, wanted, was, seq, payload],
        )?;
        Self::select(executor, account, kind, target)?.ok_or_else(|| {
            SqliteDbError::SqliteError(format!("Reader action {kind} {target} wasn't stored"))
        })
    }

    /// The row waiting for a target, if there is one.
    pub fn select(
        executor: &impl QueryExecutor,
        account: RowId,
        kind: i64,
        target: &str,
    ) -> Result<Option<DbReaderAction>, SqliteDbError> {
        let sql = format!(
            "SELECT {ACTION_COLUMNS} FROM reader_actions
            WHERE account_id = ?1 AND kind = ?2 AND target = ?3"
        );
        let mut statement = executor.prepare(&sql)?;
        Ok(statement
            .query_row(params![account, kind, target], action_from_row)
            .optional()?)
    }

    /// Every row waiting, oldest tap first.
    pub fn select_all(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<Vec<DbReaderAction>, SqliteDbError> {
        let sql = format!(
            "SELECT {ACTION_COLUMNS} FROM reader_actions WHERE account_id = ?1 ORDER BY seq"
        );
        let mut statement = executor.prepare(&sql)?;
        let rows = statement.query_map([account], action_from_row)?;
        rows.map(|row| row.map_err(SqliteDbError::from)).collect()
    }

    /// How many rows are waiting.
    pub fn count(executor: &impl QueryExecutor, account: RowId) -> Result<u64, SqliteDbError> {
        let mut statement =
            executor.prepare("SELECT COUNT(*) FROM reader_actions WHERE account_id = ?1")?;
        Ok(statement.query_row([account], |row| row.get(0))?)
    }

    /// Removes a row the server answered, unless the user tapped again since it was sent: `seq` is
    /// the row's when it was sent. Returns whether it removed it.
    pub fn finish(
        executor: &impl QueryExecutor,
        row: RowId,
        seq: i64,
    ) -> Result<bool, SqliteDbError> {
        let removed = executor.execute(
            "DELETE FROM reader_actions WHERE rowid = ?1 AND seq = ?2",
            params![row, seq],
        )?;
        Ok(removed > 0)
    }

    /// Records that the server has what a row asked for when it was sent, for a row the user
    /// tapped again since: what the cache showed before is now that.
    pub fn set_was(
        executor: &impl QueryExecutor,
        row: RowId,
        was: bool,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE reader_actions SET was = ?2 WHERE rowid = ?1",
            params![row, was],
        )?;
        Ok(())
    }

    /// Records that a row was sent and got no answer.
    pub fn record_attempt(
        executor: &impl QueryExecutor,
        row: RowId,
        error: &str,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE reader_actions SET attempts = attempts + 1, last_error = ?2 WHERE rowid = ?1",
            params![row, error],
        )?;
        Ok(())
    }
}

/// A site the user blocked, as the cache has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbReaderBlockedSite {
    pub site_id: WpComSiteId,
    pub name: Option<String>,
    pub url: Option<String>,
}

/// The sites and users a reader blocked. The Reader's streams leave out their posts.
pub struct ReaderBlockRepository;

impl ReaderBlockRepository {
    /// Blocks a site. A name or URL the caller doesn't have keeps what's stored.
    pub fn block_site(
        executor: &impl QueryExecutor,
        account: RowId,
        site: &DbReaderBlockedSite,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "INSERT INTO reader_blocked_sites (account_id, site_id, name, url)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(account_id, site_id) DO UPDATE SET
                name = COALESCE(excluded.name, name),
                url = COALESCE(excluded.url, url)",
            params![account, site.site_id.0, site.name, site.url],
        )?;
        Ok(())
    }

    pub fn unblock_site(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: WpComSiteId,
    ) -> Result<bool, SqliteDbError> {
        let removed = executor.execute(
            "DELETE FROM reader_blocked_sites WHERE account_id = ?1 AND site_id = ?2",
            params![account, site_id.0],
        )?;
        Ok(removed > 0)
    }

    pub fn is_site_blocked(
        executor: &impl QueryExecutor,
        account: RowId,
        site_id: WpComSiteId,
    ) -> Result<bool, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT EXISTS(
                SELECT 1 FROM reader_blocked_sites WHERE account_id = ?1 AND site_id = ?2)",
        )?;
        Ok(statement.query_row(params![account, site_id.0], |row| row.get(0))?)
    }

    /// The blocked sites, by name.
    pub fn select_sites(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<Vec<DbReaderBlockedSite>, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT site_id, name, url FROM reader_blocked_sites WHERE account_id = ?1
            ORDER BY name IS NULL, name COLLATE NOCASE, site_id",
        )?;
        let rows = statement.query_map([account], |row| {
            Ok(DbReaderBlockedSite {
                site_id: WpComSiteId(row.get(0)?),
                name: row.get(1)?,
                url: row.get(2)?,
            })
        })?;
        rows.map(|row| row.map_err(SqliteDbError::from)).collect()
    }

    /// Makes the blocked sites be exactly `sites`, which is what the server's list says, apart
    /// from `keep`: sites with a block or unblock still waiting to be sent, which stay as the user
    /// left them. Call it inside a transaction.
    pub fn replace_sites(
        executor: &impl QueryExecutor,
        account: RowId,
        sites: &[DbReaderBlockedSite],
        keep: &[WpComSiteId],
    ) -> Result<(), SqliteDbError> {
        let ids = |sites: &mut dyn Iterator<Item = u64>| {
            let ids: Vec<String> = sites.map(|id| id.to_string()).collect();
            format!("[{}]", ids.join(","))
        };
        let listed = ids(&mut sites.iter().map(|site| site.site_id.0));
        let kept = ids(&mut keep.iter().map(|site| site.0));
        executor.execute(
            "DELETE FROM reader_blocked_sites
            WHERE account_id = ?1
                AND site_id NOT IN (SELECT value FROM json_each(?2))
                AND site_id NOT IN (SELECT value FROM json_each(?3))",
            params![account, listed, kept],
        )?;
        for site in sites {
            if !keep.contains(&site.site_id) {
                Self::block_site(executor, account, site)?;
            }
        }
        Ok(())
    }

    /// Blocks or unblocks a user, on this device. Returns whether it changed anything.
    pub fn set_user_blocked(
        executor: &impl QueryExecutor,
        account: RowId,
        user_id: WpComUserId,
        blocked: bool,
    ) -> Result<bool, SqliteDbError> {
        let changed = if blocked {
            executor.execute(
                "INSERT INTO reader_blocked_users (account_id, user_id) VALUES (?1, ?2)
                ON CONFLICT(account_id, user_id) DO NOTHING",
                params![account, user_id.0],
            )?
        } else {
            executor.execute(
                "DELETE FROM reader_blocked_users WHERE account_id = ?1 AND user_id = ?2",
                params![account, user_id.0],
            )?
        };
        Ok(changed > 0)
    }

    /// The blocked users, in the order they were blocked.
    pub fn select_users(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<Vec<WpComUserId>, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT user_id FROM reader_blocked_users WHERE account_id = ?1 ORDER BY rowid",
        )?;
        let rows = statement.query_map([account], |row| Ok(WpComUserId(row.get(0)?)))?;
        rows.map(|row| row.map_err(SqliteDbError::from)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MigrationManager;
    use crate::repository::wordpress_com_accounts::WordPressComAccountRepository;
    use rusqlite::Connection;

    fn conn() -> (Connection, RowId) {
        let conn = Connection::open_in_memory().unwrap();
        MigrationManager::new(&conn)
            .unwrap()
            .perform_migrations()
            .unwrap();
        let account = WordPressComAccountRepository::get_or_create(&conn, WpComUserId(1)).unwrap();
        (conn, account)
    }

    fn site(id: u64, name: &str) -> DbReaderBlockedSite {
        DbReaderBlockedSite {
            site_id: WpComSiteId(id),
            name: Some(name.to_string()),
            url: None,
        }
    }

    #[test]
    fn test_a_second_tap_on_a_target_keeps_one_row_and_what_the_cache_showed_first() {
        let (conn, account) = conn();
        let first =
            ReaderActionRepository::tap(&conn, account, 1, "post:1:2", true, Some(false), "{}")
                .unwrap();
        ReaderActionRepository::tap(&conn, account, 1, "post:1:3", true, None, "{}").unwrap();
        // The cache shows the first tap by now, which isn't what the server had.
        let again =
            ReaderActionRepository::tap(&conn, account, 1, "post:1:2", false, Some(true), "{}")
                .unwrap();

        assert_eq!(again.row_id, first.row_id);
        assert!(!again.wanted);
        assert_eq!(again.was, Some(false));
        assert!(again.seq > first.seq);
        // The row tapped last is sent last.
        let targets: Vec<_> = ReaderActionRepository::select_all(&conn, account)
            .unwrap()
            .into_iter()
            .map(|row| row.target)
            .collect();
        assert_eq!(targets, ["post:1:3", "post:1:2"]);
        assert_eq!(ReaderActionRepository::count(&conn, account).unwrap(), 2);
    }

    #[test]
    fn test_the_same_target_of_another_kind_is_another_row() {
        let (conn, account) = conn();
        ReaderActionRepository::tap(&conn, account, 1, "post:1:2", true, None, "{}").unwrap();
        ReaderActionRepository::tap(&conn, account, 2, "post:1:2", true, None, "{}").unwrap();
        assert_eq!(ReaderActionRepository::count(&conn, account).unwrap(), 2);
    }

    #[test]
    fn test_finish_leaves_a_row_tapped_since_it_was_sent() {
        let (conn, account) = conn();
        let sent =
            ReaderActionRepository::tap(&conn, account, 1, "post:1:2", true, Some(false), "{}")
                .unwrap();
        let tapped =
            ReaderActionRepository::tap(&conn, account, 1, "post:1:2", false, Some(true), "{}")
                .unwrap();

        assert!(!ReaderActionRepository::finish(&conn, sent.row_id, sent.seq).unwrap());
        ReaderActionRepository::set_was(&conn, sent.row_id, true).unwrap();
        let row = ReaderActionRepository::select(&conn, account, 1, "post:1:2")
            .unwrap()
            .unwrap();
        assert_eq!(row.was, Some(true));

        assert!(ReaderActionRepository::finish(&conn, tapped.row_id, tapped.seq).unwrap());
        assert_eq!(ReaderActionRepository::count(&conn, account).unwrap(), 0);
    }

    #[test]
    fn test_an_attempt_is_counted_and_a_new_tap_starts_again() {
        let (conn, account) = conn();
        let row =
            ReaderActionRepository::tap(&conn, account, 1, "post:1:2", true, None, "{}").unwrap();
        ReaderActionRepository::record_attempt(&conn, row.row_id, "offline").unwrap();
        let row = ReaderActionRepository::select(&conn, account, 1, "post:1:2")
            .unwrap()
            .unwrap();
        assert_eq!(row.attempts, 1);
        assert_eq!(row.last_error.as_deref(), Some("offline"));

        let row =
            ReaderActionRepository::tap(&conn, account, 1, "post:1:2", true, None, "{}").unwrap();
        assert_eq!(row.attempts, 0);
        assert_eq!(row.last_error, None);
    }

    #[test]
    fn test_blocked_sites() {
        let (conn, account) = conn();
        ReaderBlockRepository::block_site(&conn, account, &site(2, "Beta")).unwrap();
        ReaderBlockRepository::block_site(&conn, account, &site(1, "alpha")).unwrap();
        // Blocking again without a name keeps the name.
        ReaderBlockRepository::block_site(
            &conn,
            account,
            &DbReaderBlockedSite {
                site_id: WpComSiteId(1),
                name: None,
                url: Some("https://example.com".to_string()),
            },
        )
        .unwrap();

        let sites = ReaderBlockRepository::select_sites(&conn, account).unwrap();
        assert_eq!(
            sites.iter().map(|site| site.site_id.0).collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(sites[0].name.as_deref(), Some("alpha"));
        assert_eq!(sites[0].url.as_deref(), Some("https://example.com"));
        assert!(ReaderBlockRepository::is_site_blocked(&conn, account, WpComSiteId(2)).unwrap());

        assert!(ReaderBlockRepository::unblock_site(&conn, account, WpComSiteId(2)).unwrap());
        assert!(!ReaderBlockRepository::unblock_site(&conn, account, WpComSiteId(2)).unwrap());
        assert!(!ReaderBlockRepository::is_site_blocked(&conn, account, WpComSiteId(2)).unwrap());
    }

    #[test]
    fn test_the_servers_list_replaces_the_blocked_sites_except_the_ones_to_keep() {
        let (conn, account) = conn();
        ReaderBlockRepository::block_site(&conn, account, &site(1, "One")).unwrap();
        ReaderBlockRepository::block_site(&conn, account, &site(2, "Two")).unwrap();

        // The server has 3 and 4. Site 2's block and site 4's unblock are still waiting to be sent.
        ReaderBlockRepository::replace_sites(
            &conn,
            account,
            &[site(3, "Three"), site(4, "Four")],
            &[WpComSiteId(2), WpComSiteId(4)],
        )
        .unwrap();

        let ids: Vec<_> = ReaderBlockRepository::select_sites(&conn, account)
            .unwrap()
            .iter()
            .map(|site| site.site_id.0)
            .collect();
        assert_eq!(ids, [3, 2]);
    }

    #[test]
    fn test_blocked_users() {
        let (conn, account) = conn();
        assert!(
            ReaderBlockRepository::set_user_blocked(&conn, account, WpComUserId(7), true).unwrap()
        );
        assert!(
            !ReaderBlockRepository::set_user_blocked(&conn, account, WpComUserId(7), true).unwrap()
        );
        ReaderBlockRepository::set_user_blocked(&conn, account, WpComUserId(8), true).unwrap();
        assert_eq!(
            ReaderBlockRepository::select_users(&conn, account).unwrap(),
            [WpComUserId(7), WpComUserId(8)]
        );
        assert!(
            ReaderBlockRepository::set_user_blocked(&conn, account, WpComUserId(7), false).unwrap()
        );
        assert_eq!(
            ReaderBlockRepository::select_users(&conn, account).unwrap(),
            [WpComUserId(8)]
        );
    }
}
