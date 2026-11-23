//! An account's WordPress.com notifications (notes), kept as the API sent them, and where their
//! sync is up to.

use crate::{RowId, SqliteDbError, repository::QueryExecutor};
use rusqlite::{OptionalExtension, params, params_from_iter, types::Value};
use std::collections::{HashMap, HashSet};
use wp_api::wp_com::notifications::{NotificationId, NotificationType, NotificationWithJson};

/// A note with a time before this many seconds since 1970 has no date: the API sends a time in
/// the first day of 1970 for one.
pub const UNDATED_BEFORE: i64 = 86_400;

/// A stored note: its ID and its JSON, as the API sent it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbNotification {
    pub row_id: RowId,
    pub note_id: NotificationId,
    pub json: String,
}

/// What the cache knows of a note's hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbKnownHash {
    /// The note is stored whole, with the hash it came with.
    Whole(Option<String>),
    /// A list had the note, and it didn't come whole when asked for: the hash the list gave it.
    ListedOnly(Option<String>),
}

/// Which notes [`NotificationRepository::select`] returns.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DbNotificationFilter {
    /// Only notes the user hasn't read. A note that doesn't say counts as unread.
    pub unread_only: bool,
    /// Only notes of these types. Empty for every type.
    pub types: Vec<NotificationType>,
}

impl DbNotificationFilter {
    /// Whether the filter passes every note.
    pub fn is_everything(&self) -> bool {
        !self.unread_only && self.types.is_empty()
    }

    /// The filter as SQL conditions, each starting with ` AND`, with its values added to `values`.
    fn conditions(&self, values: &mut Vec<Value>) -> String {
        let mut sql = String::new();
        if self.unread_only {
            sql.push_str(" AND is_read IS NOT 1");
        }
        if !self.types.is_empty() {
            sql.push_str(&format!(
                " AND note_type IN ({})",
                vec!["?"; self.types.len()].join(",")
            ));
            values.extend(
                self.types
                    .iter()
                    .map(|note_type| Value::Text(note_type.as_str().to_string())),
            );
        }
        sql
    }
}

/// The notes a list of IDs covers, so the stored notes in it that the list didn't have can go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbNotificationWindow {
    /// Every note: a list of the newest notes that was shorter than asked for.
    Everything,
    /// The dated notes from `before` back: a list with a `before` that was shorter than asked
    /// for. A note with no date isn't covered: the API lists one among dated notes, not where its
    /// time puts it.
    AtOrBefore { before: i64 },
    /// The dated notes after `after` and up to `before`, or with no upper end when `before` is
    /// `None`: a full list, whose oldest dated note is at `after`. Notes sharing that second
    /// may not all fit in the list, so it isn't covered.
    DatedBetween { after: i64, before: Option<i64> },
}

/// When an account's notifications were last synced and looked at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DbNotificationSyncState {
    /// When the newest notes were last synced, in seconds since 1970.
    pub last_synced_at: Option<i64>,
    /// When the user last looked at their notifications, in seconds since 1970.
    pub last_seen_time: Option<i64>,
}

/// Where one tab of an account's notifications is up to in loading older notes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DbNotificationTabState {
    /// The `before` of the next older page, in seconds since 1970. `None` for the newest notes.
    pub older_before: Option<i64>,
    /// The `number` of the next older page. `None` until a sync sets it.
    pub older_number: Option<u32>,
    pub reached_end: bool,
    /// The time of the newest dated note the tab's last sync listed, in seconds since 1970.
    pub newest_time: Option<i64>,
}

fn hash_text(hash: Option<u64>) -> Option<String> {
    hash.map(|hash| hash.to_string())
}

/// The notifications of WordPress.com accounts. Each row belongs to an account's row in
/// `wordpress_com_accounts` and is deleted with it.
pub struct NotificationRepository;

impl NotificationRepository {
    /// Stores a note whole, replacing what was stored of it, and forgets that it was listed only.
    pub fn upsert(
        executor: &impl QueryExecutor,
        account: RowId,
        note: &NotificationWithJson,
    ) -> Result<(), SqliteDbError> {
        let fields = &note.note;
        executor.execute(
            "INSERT INTO notifications (account_id, note_id, note_type, is_read, timestamp, note_hash, json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(account_id, note_id) DO UPDATE SET
               note_type = excluded.note_type,
               is_read = excluded.is_read,
               timestamp = excluded.timestamp,
               note_hash = excluded.note_hash,
               json = excluded.json,
               fetched_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            params![
                account,
                fields.id.0,
                fields.note_type.as_ref().map(NotificationType::as_str),
                fields.read,
                fields.timestamp.map(|time| time.0.timestamp()),
                hash_text(fields.note_hash),
                note.json,
            ],
        )?;
        executor.execute(
            "DELETE FROM notification_listed_only WHERE account_id = ?1 AND note_id = ?2",
            params![account, fields.id.0],
        )?;
        Ok(())
    }

    /// Records that a list had the note with `hash`, and that it didn't come whole, or came and
    /// couldn't be read (`error`). A stored copy of the note is deleted: the API no longer sends
    /// it.
    pub fn record_listed_only(
        executor: &impl QueryExecutor,
        account: RowId,
        note_id: NotificationId,
        hash: Option<u64>,
        error: Option<&str>,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "INSERT INTO notification_listed_only (account_id, note_id, note_hash, error)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(account_id, note_id) DO UPDATE SET
               note_hash = excluded.note_hash,
               error = excluded.error",
            params![account, note_id.0, hash_text(hash), error],
        )?;
        executor.execute(
            "DELETE FROM notifications WHERE account_id = ?1 AND note_id = ?2",
            params![account, note_id.0],
        )?;
        Ok(())
    }

    /// What the cache knows of each of `ids`' hashes. An ID it knows nothing of is left out.
    pub fn known_hashes(
        executor: &impl QueryExecutor,
        account: RowId,
        ids: &[NotificationId],
    ) -> Result<HashMap<NotificationId, DbKnownHash>, SqliteDbError> {
        let mut known = HashMap::new();
        for (table, whole) in [("notifications", true), ("notification_listed_only", false)] {
            for (id, hash) in
                Self::select_by_ids::<Option<String>>(executor, account, ids, table, "note_hash")?
            {
                // A note is in one table at a time.
                known.insert(
                    id,
                    if whole {
                        DbKnownHash::Whole(hash)
                    } else {
                        DbKnownHash::ListedOnly(hash)
                    },
                );
            }
        }
        Ok(known)
    }

    /// The time of each of `ids` that's stored whole, in seconds since 1970, or `None` when it
    /// has none.
    pub fn times(
        executor: &impl QueryExecutor,
        account: RowId,
        ids: &[NotificationId],
    ) -> Result<HashMap<NotificationId, Option<i64>>, SqliteDbError> {
        Ok(
            Self::select_by_ids(executor, account, ids, "notifications", "timestamp")?
                .into_iter()
                .collect(),
        )
    }

    /// Whether each of `ids` that's stored whole was read, or `None` when it didn't say.
    pub fn read_states(
        executor: &impl QueryExecutor,
        account: RowId,
        ids: &[NotificationId],
    ) -> Result<HashMap<NotificationId, Option<bool>>, SqliteDbError> {
        Ok(
            Self::select_by_ids(executor, account, ids, "notifications", "is_read")?
                .into_iter()
                .collect(),
        )
    }

    fn select_by_ids<T: rusqlite::types::FromSql>(
        executor: &impl QueryExecutor,
        account: RowId,
        ids: &[NotificationId],
        table: &str,
        column: &str,
    ) -> Result<Vec<(NotificationId, T)>, SqliteDbError> {
        let mut found = Vec::new();
        // SQLite limits how many values a statement takes.
        for chunk in ids.chunks(500) {
            let placeholders = vec!["?"; chunk.len()].join(",");
            let mut statement = executor.prepare(&format!(
                "SELECT note_id, {column} FROM {table} WHERE account_id = ? AND note_id IN ({placeholders})"
            ))?;
            let values = std::iter::once(Value::Integer(account.0))
                .chain(chunk.iter().map(|id| Value::Integer(id.0 as i64)));
            let rows = statement.query_map(params_from_iter(values), |row| {
                Ok((NotificationId(row.get(0)?), row.get(1)?))
            })?;
            for row in rows {
                found.push(row?);
            }
        }
        Ok(found)
    }

    /// The account's stored notes that pass `filter`, newest first. A note with no date is last.
    pub fn select(
        executor: &impl QueryExecutor,
        account: RowId,
        filter: &DbNotificationFilter,
    ) -> Result<Vec<DbNotification>, SqliteDbError> {
        let mut values = vec![Value::Integer(account.0)];
        let sql = format!(
            "SELECT rowid, note_id, json FROM notifications WHERE account_id = ?{} \
             ORDER BY timestamp IS NULL, timestamp DESC, note_id DESC",
            filter.conditions(&mut values)
        );
        let mut statement = executor.prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(values), |row| {
            Ok(DbNotification {
                row_id: row.get(0)?,
                note_id: NotificationId(row.get(1)?),
                json: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// How many notes the account has stored whole.
    pub fn count(executor: &impl QueryExecutor, account: RowId) -> Result<u64, SqliteDbError> {
        Self::count_in(executor, account, "notifications", "")
    }

    /// How many notes the account's lists had that never came whole, and how many of them came
    /// and couldn't be read.
    pub fn count_listed_only(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<(u64, u64), SqliteDbError> {
        Ok((
            Self::count_in(executor, account, "notification_listed_only", "")?,
            Self::count_in(
                executor,
                account,
                "notification_listed_only",
                " AND error IS NOT NULL",
            )?,
        ))
    }

    fn count_in(
        executor: &impl QueryExecutor,
        account: RowId,
        table: &str,
        condition: &str,
    ) -> Result<u64, SqliteDbError> {
        let mut statement = executor.prepare(&format!(
            "SELECT COUNT(*) FROM {table} WHERE account_id = ?1{condition}"
        ))?;
        Ok(statement.query_row([account], |row| row.get(0))?)
    }

    /// The stored notes in `window` that pass `filter` and aren't in `listed`, newest first.
    pub fn unlisted(
        executor: &impl QueryExecutor,
        account: RowId,
        window: DbNotificationWindow,
        filter: &DbNotificationFilter,
        listed: &HashSet<NotificationId>,
    ) -> Result<Vec<NotificationId>, SqliteDbError> {
        let (condition, bounds): (&str, Vec<i64>) = match window {
            DbNotificationWindow::Everything => ("", Vec::new()),
            DbNotificationWindow::AtOrBefore { before } => (
                " AND timestamp >= ? AND timestamp <= ?",
                vec![UNDATED_BEFORE, before],
            ),
            DbNotificationWindow::DatedBetween {
                after,
                before: None,
            } => (
                " AND timestamp >= ? AND timestamp > ?",
                vec![UNDATED_BEFORE, after],
            ),
            DbNotificationWindow::DatedBetween {
                after,
                before: Some(before),
            } => (
                " AND timestamp >= ? AND timestamp > ? AND timestamp <= ?",
                vec![UNDATED_BEFORE, after, before],
            ),
        };
        let mut values = vec![Value::Integer(account.0)];
        values.extend(bounds.into_iter().map(Value::Integer));
        let sql = format!(
            "SELECT note_id FROM notifications WHERE account_id = ?{condition}{} \
             ORDER BY timestamp IS NULL, timestamp DESC, note_id DESC",
            filter.conditions(&mut values)
        );
        let mut statement = executor.prepare(&sql)?;
        let ids = statement
            .query_map(params_from_iter(values), |row| {
                Ok(NotificationId(row.get(0)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ids.into_iter().filter(|id| !listed.contains(id)).collect())
    }

    /// Deletes the notes in `window` that pass `filter` and aren't in `listed`, and returns how
    /// many whole notes went. Notes a list had that never came whole only go with
    /// [`DbNotificationWindow::Everything`] and a filter that passes every note: without a time
    /// and a type, there's no telling whether they're in a narrower window.
    pub fn delete_unlisted(
        executor: &impl QueryExecutor,
        account: RowId,
        window: DbNotificationWindow,
        filter: &DbNotificationFilter,
        listed: &HashSet<NotificationId>,
    ) -> Result<u64, SqliteDbError> {
        let mut deleted = 0;
        for note_id in Self::unlisted(executor, account, window, filter, listed)? {
            deleted += executor.execute(
                "DELETE FROM notifications WHERE account_id = ?1 AND note_id = ?2",
                params![account, note_id.0],
            )? as u64;
        }
        if window == DbNotificationWindow::Everything && filter.is_everything() {
            let listed_only: Vec<NotificationId> = {
                let mut statement = executor.prepare(
                    "SELECT note_id FROM notification_listed_only WHERE account_id = ?1",
                )?;
                statement
                    .query_map([account], |row| Ok(NotificationId(row.get(0)?)))?
                    .collect::<Result<_, _>>()?
            };
            for note_id in listed_only {
                if !listed.contains(&note_id) {
                    executor.execute(
                        "DELETE FROM notification_listed_only WHERE account_id = ?1 AND note_id = ?2",
                        params![account, note_id.0],
                    )?;
                }
            }
        }
        Ok(deleted)
    }

    /// Deletes what's stored of a note, whole or listed only. Returns whether there was any.
    pub fn delete(
        executor: &impl QueryExecutor,
        account: RowId,
        note_id: NotificationId,
    ) -> Result<bool, SqliteDbError> {
        let mut deleted = 0;
        for table in ["notifications", "notification_listed_only"] {
            deleted += executor.execute(
                &format!("DELETE FROM {table} WHERE account_id = ?1 AND note_id = ?2"),
                params![account, note_id.0],
            )?;
        }
        Ok(deleted > 0)
    }

    /// When the account's notifications were last synced and looked at. The default before the
    /// first sync.
    pub fn sync_state(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<DbNotificationSyncState, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT last_synced_at, last_seen_time FROM notification_sync_state WHERE account_id = ?1",
        )?;
        Ok(statement
            .query_row([account], |row| {
                Ok(DbNotificationSyncState {
                    last_synced_at: row.get(0)?,
                    last_seen_time: row.get(1)?,
                })
            })
            .optional()?
            .unwrap_or_default())
    }

    /// Replaces when the account's notifications were last synced and looked at.
    pub fn set_sync_state(
        executor: &impl QueryExecutor,
        account: RowId,
        state: &DbNotificationSyncState,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "INSERT INTO notification_sync_state (account_id, last_synced_at, last_seen_time)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(account_id) DO UPDATE SET
               last_synced_at = excluded.last_synced_at,
               last_seen_time = excluded.last_seen_time",
            params![account, state.last_synced_at, state.last_seen_time],
        )?;
        Ok(())
    }

    /// Where the tab named `tab` is up to. The default before its first sync.
    pub fn tab_state(
        executor: &impl QueryExecutor,
        account: RowId,
        tab: &str,
    ) -> Result<DbNotificationTabState, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT older_before, older_number, reached_end, newest_time
             FROM notification_tab_state WHERE account_id = ?1 AND tab = ?2",
        )?;
        Ok(statement
            .query_row(params![account, tab], |row| {
                Ok(DbNotificationTabState {
                    older_before: row.get(0)?,
                    older_number: row.get(1)?,
                    reached_end: row.get(2)?,
                    newest_time: row.get(3)?,
                })
            })
            .optional()?
            .unwrap_or_default())
    }

    /// Replaces where the tab named `tab` is up to.
    pub fn set_tab_state(
        executor: &impl QueryExecutor,
        account: RowId,
        tab: &str,
        state: &DbNotificationTabState,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "INSERT INTO notification_tab_state
               (account_id, tab, older_before, older_number, reached_end, newest_time)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(account_id, tab) DO UPDATE SET
               older_before = excluded.older_before,
               older_number = excluded.older_number,
               reached_end = excluded.reached_end,
               newest_time = excluded.newest_time",
            params![
                account,
                tab,
                state.older_before,
                state.older_number,
                state.reached_end,
                state.newest_time,
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MigrationManager;
    use crate::repository::wordpress_com_accounts::WordPressComAccountRepository;
    use rusqlite::Connection;
    use wp_api::wp_com::me::WpComUserId;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        MigrationManager::new(&conn)
            .unwrap()
            .perform_migrations()
            .unwrap();
        conn
    }

    fn account(conn: &Connection, user: u64) -> RowId {
        WordPressComAccountRepository::get_or_create(conn, WpComUserId(user)).unwrap()
    }

    fn id(n: u64) -> NotificationId {
        NotificationId(10_000_000_000 + n)
    }

    /// A hand-built note at `time` seconds since 1970.
    fn note(n: u64, time: i64, hash: u64, read: bool, note_type: &str) -> NotificationWithJson {
        let json = format!(
            r#"{{"id":{},"type":"{note_type}","read":{},"timestamp":"{}","note_hash":{hash},"a_later_field":"kept"}}"#,
            id(n).0,
            u8::from(read),
            chrono::DateTime::from_timestamp(time, 0)
                .unwrap()
                .to_rfc3339(),
        );
        NotificationWithJson {
            note: serde_json::from_str(&json).unwrap(),
            json,
        }
    }

    fn stored(conn: &Connection, account: RowId, filter: &DbNotificationFilter) -> Vec<u64> {
        NotificationRepository::select(conn, account, filter)
            .unwrap()
            .iter()
            .map(|note| note.note_id.0 - 10_000_000_000)
            .collect()
    }

    fn everything(conn: &Connection, account: RowId) -> Vec<u64> {
        stored(conn, account, &DbNotificationFilter::default())
    }

    #[test]
    fn test_notes_are_kept_whole_newest_first_and_undated_last() {
        let conn = conn();
        let account = account(&conn, 1);
        for note in [
            note(1, 200_000, 11, true, "like"),
            note(2, 0, 12, true, "follow"),
            note(3, 300_000, 13, false, "comment"),
        ] {
            NotificationRepository::upsert(&conn, account, &note).unwrap();
        }
        assert_eq!(everything(&conn, account), [3, 1, 2]);
        let first =
            &NotificationRepository::select(&conn, account, &Default::default()).unwrap()[0];
        assert!(first.json.contains(r#""a_later_field":"kept""#));
        assert_eq!(NotificationRepository::count(&conn, account).unwrap(), 3);
    }

    #[test]
    fn test_a_filter_picks_unread_notes_and_types() {
        let conn = conn();
        let account = account(&conn, 1);
        for note in [
            note(1, 200_000, 11, true, "like"),
            note(2, 250_000, 12, false, "like"),
            note(3, 300_000, 13, false, "comment"),
        ] {
            NotificationRepository::upsert(&conn, account, &note).unwrap();
        }
        let unread = DbNotificationFilter {
            unread_only: true,
            ..Default::default()
        };
        assert_eq!(stored(&conn, account, &unread), [3, 2]);
        let likes = DbNotificationFilter {
            types: vec![NotificationType::Like],
            ..Default::default()
        };
        assert_eq!(stored(&conn, account, &likes), [2, 1]);
    }

    #[test]
    fn test_two_accounts_keep_their_own_notes_and_state() {
        let conn = conn();
        let first = account(&conn, 1);
        let second = account(&conn, 2);
        NotificationRepository::upsert(&conn, first, &note(1, 200_000, 11, true, "like")).unwrap();
        NotificationRepository::upsert(&conn, second, &note(1, 200_000, 99, true, "like")).unwrap();
        NotificationRepository::upsert(&conn, second, &note(2, 300_000, 12, true, "like")).unwrap();
        NotificationRepository::set_tab_state(
            &conn,
            first,
            "all",
            &DbNotificationTabState {
                reached_end: true,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(everything(&conn, first), [1]);
        assert_eq!(everything(&conn, second), [2, 1]);
        assert_eq!(
            NotificationRepository::known_hashes(&conn, first, &[id(1)]).unwrap()[&id(1)],
            DbKnownHash::Whole(Some("11".to_string()))
        );
        assert!(
            NotificationRepository::tab_state(&conn, first, "all")
                .unwrap()
                .reached_end
        );
        assert!(
            !NotificationRepository::tab_state(&conn, second, "all")
                .unwrap()
                .reached_end
        );

        NotificationRepository::delete_unlisted(
            &conn,
            first,
            DbNotificationWindow::Everything,
            &DbNotificationFilter::default(),
            &HashSet::new(),
        )
        .unwrap();
        assert!(everything(&conn, first).is_empty());
        assert_eq!(everything(&conn, second), [2, 1]);
    }

    #[test]
    fn test_removing_the_account_removes_its_notes() {
        let conn = conn();
        let first = account(&conn, 1);
        let second = account(&conn, 2);
        NotificationRepository::upsert(&conn, first, &note(1, 200_000, 11, true, "like")).unwrap();
        NotificationRepository::record_listed_only(&conn, first, id(2), Some(5), None).unwrap();
        NotificationRepository::set_sync_state(&conn, first, &Default::default()).unwrap();
        NotificationRepository::set_tab_state(&conn, first, "likes", &Default::default()).unwrap();
        NotificationRepository::upsert(&conn, second, &note(1, 200_000, 11, true, "like")).unwrap();

        WordPressComAccountRepository::delete(&conn, WpComUserId(1)).unwrap();

        for table in [
            "notifications",
            "notification_listed_only",
            "notification_sync_state",
            "notification_tab_state",
        ] {
            let rows: i64 = conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM {table} WHERE account_id = ?1"),
                    [first],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(rows, 0, "{table}");
        }
        assert_eq!(everything(&conn, second), [1]);
    }

    #[test]
    fn test_a_note_is_either_whole_or_listed_only() {
        let conn = conn();
        let account = account(&conn, 1);
        NotificationRepository::record_listed_only(&conn, account, id(1), Some(5), None).unwrap();
        assert_eq!(
            NotificationRepository::known_hashes(&conn, account, &[id(1), id(2)]).unwrap(),
            HashMap::from([(id(1), DbKnownHash::ListedOnly(Some("5".to_string())))])
        );
        NotificationRepository::upsert(&conn, account, &note(1, 200_000, 11, true, "like"))
            .unwrap();
        assert_eq!(
            NotificationRepository::known_hashes(&conn, account, &[id(1)]).unwrap()[&id(1)],
            DbKnownHash::Whole(Some("11".to_string()))
        );
        assert_eq!(
            NotificationRepository::count_listed_only(&conn, account).unwrap(),
            (0, 0)
        );
        NotificationRepository::record_listed_only(&conn, account, id(1), Some(6), Some("bad"))
            .unwrap();
        assert!(everything(&conn, account).is_empty());
        assert_eq!(
            NotificationRepository::count_listed_only(&conn, account).unwrap(),
            (1, 1)
        );
    }

    #[test]
    fn test_hashes_past_the_signed_range_are_kept_exactly() {
        let conn = conn();
        let account = account(&conn, 1);
        NotificationRepository::upsert(&conn, account, &note(1, 200_000, u64::MAX, true, "like"))
            .unwrap();
        assert_eq!(
            NotificationRepository::known_hashes(&conn, account, &[id(1)]).unwrap()[&id(1)],
            DbKnownHash::Whole(Some(u64::MAX.to_string()))
        );
    }

    #[test]
    fn test_a_window_removes_only_unlisted_notes_inside_it() {
        let conn = conn();
        let account = account(&conn, 1);
        let fill = || {
            for note in [
                note(1, 100_000, 1, true, "like"),
                note(2, 200_000, 2, true, "like"),
                note(3, 200_000, 3, true, "like"),
                note(4, 300_000, 4, true, "like"),
                note(5, 400_000, 5, true, "like"),
                note(6, 0, 6, true, "like"),
            ] {
                NotificationRepository::upsert(&conn, account, &note).unwrap();
            }
        };
        let listed = |ids: &[u64]| ids.iter().map(|n| id(*n)).collect::<HashSet<_>>();

        // A full list of the newest notes whose oldest is at 200,000: the notes sharing that
        // second, older notes and undated ones are outside it.
        fill();
        let removed = NotificationRepository::delete_unlisted(
            &conn,
            account,
            DbNotificationWindow::DatedBetween {
                after: 200_000,
                before: None,
            },
            &DbNotificationFilter::default(),
            &listed(&[5, 2]),
        )
        .unwrap();
        assert_eq!(removed, 1);
        assert_eq!(everything(&conn, account), [5, 3, 2, 1, 6]);

        // An older full page up to 300,000.
        fill();
        NotificationRepository::delete_unlisted(
            &conn,
            account,
            DbNotificationWindow::DatedBetween {
                after: 100_000,
                before: Some(300_000),
            },
            &DbNotificationFilter::default(),
            &listed(&[4, 1]),
        )
        .unwrap();
        assert_eq!(everything(&conn, account), [5, 4, 1, 6]);

        // A short older page lists every dated note at or before its `before`.
        fill();
        NotificationRepository::delete_unlisted(
            &conn,
            account,
            DbNotificationWindow::AtOrBefore { before: 200_000 },
            &DbNotificationFilter::default(),
            &listed(&[3]),
        )
        .unwrap();
        assert_eq!(everything(&conn, account), [5, 4, 3, 6]);

        // A short list of the newest notes lists them all.
        fill();
        NotificationRepository::record_listed_only(&conn, account, id(7), None, None).unwrap();
        NotificationRepository::record_listed_only(&conn, account, id(8), None, None).unwrap();
        NotificationRepository::delete_unlisted(
            &conn,
            account,
            DbNotificationWindow::Everything,
            &DbNotificationFilter::default(),
            &listed(&[4, 8]),
        )
        .unwrap();
        assert_eq!(everything(&conn, account), [4]);
        assert_eq!(
            NotificationRepository::count_listed_only(&conn, account).unwrap(),
            (1, 0)
        );
    }

    #[test]
    fn test_the_sync_state_round_trips() {
        let conn = conn();
        let account = account(&conn, 1);
        assert_eq!(
            NotificationRepository::sync_state(&conn, account).unwrap(),
            DbNotificationSyncState::default()
        );
        let state = DbNotificationSyncState {
            last_synced_at: Some(1_700_000_100),
            last_seen_time: Some(1_700_000_050),
        };
        NotificationRepository::set_sync_state(&conn, account, &state).unwrap();
        assert_eq!(
            NotificationRepository::sync_state(&conn, account).unwrap(),
            state
        );
    }

    #[test]
    fn test_each_tab_keeps_its_own_state() {
        let conn = conn();
        let account = account(&conn, 1);
        let likes = DbNotificationTabState {
            older_before: Some(1_700_000_000),
            older_number: Some(200),
            reached_end: true,
            newest_time: Some(1_700_000_200),
        };
        NotificationRepository::set_tab_state(&conn, account, "likes", &likes).unwrap();
        assert_eq!(
            NotificationRepository::tab_state(&conn, account, "likes").unwrap(),
            likes
        );
        assert_eq!(
            NotificationRepository::tab_state(&conn, account, "all").unwrap(),
            DbNotificationTabState::default()
        );
    }

    #[test]
    fn test_a_filter_narrows_what_a_window_removes() {
        let conn = conn();
        let account = account(&conn, 1);
        for note in [
            note(1, 100_000, 1, false, "like"),
            note(2, 200_000, 2, true, "comment"),
            note(3, 300_000, 3, false, "comment"),
        ] {
            NotificationRepository::upsert(&conn, account, &note).unwrap();
        }
        NotificationRepository::record_listed_only(&conn, account, id(4), None, None).unwrap();
        let comments = DbNotificationFilter {
            types: vec![NotificationType::Comment],
            ..Default::default()
        };
        let unread = DbNotificationFilter {
            unread_only: true,
            ..Default::default()
        };
        assert_eq!(
            NotificationRepository::unlisted(
                &conn,
                account,
                DbNotificationWindow::Everything,
                &unread,
                &[id(3)].into(),
            )
            .unwrap(),
            [id(1)]
        );
        let removed = NotificationRepository::delete_unlisted(
            &conn,
            account,
            DbNotificationWindow::Everything,
            &comments,
            &[id(3)].into(),
        )
        .unwrap();
        assert_eq!(removed, 1);
        assert_eq!(everything(&conn, account), [3, 1]);
        // A note listed only has no type, so a narrowed window leaves it.
        assert_eq!(
            NotificationRepository::count_listed_only(&conn, account).unwrap(),
            (1, 0)
        );
    }

    #[test]
    fn test_a_note_that_does_not_say_whether_it_was_read_is_unread() {
        let conn = conn();
        let account = account(&conn, 1);
        let json = format!(r#"{{"id":{},"type":"like"}}"#, id(1).0);
        let no_read = NotificationWithJson {
            note: serde_json::from_str(&json).unwrap(),
            json,
        };
        NotificationRepository::upsert(&conn, account, &no_read).unwrap();
        NotificationRepository::upsert(&conn, account, &note(2, 200_000, 2, true, "like")).unwrap();
        let unread = DbNotificationFilter {
            unread_only: true,
            ..Default::default()
        };
        assert_eq!(stored(&conn, account, &unread), [1]);
        assert_eq!(
            NotificationRepository::read_states(&conn, account, &[id(1), id(2)]).unwrap(),
            HashMap::from([(id(1), None), (id(2), Some(true))])
        );
    }

    #[test]
    fn test_deleting_a_note_removes_it_whole_or_listed_only() {
        let conn = conn();
        let account = account(&conn, 1);
        NotificationRepository::upsert(&conn, account, &note(1, 200_000, 1, true, "like")).unwrap();
        NotificationRepository::record_listed_only(&conn, account, id(2), None, None).unwrap();
        assert!(NotificationRepository::delete(&conn, account, id(1)).unwrap());
        assert!(NotificationRepository::delete(&conn, account, id(2)).unwrap());
        assert!(!NotificationRepository::delete(&conn, account, id(3)).unwrap());
        assert!(everything(&conn, account).is_empty());
        assert_eq!(
            NotificationRepository::count_listed_only(&conn, account).unwrap(),
            (0, 0)
        );
    }
}
