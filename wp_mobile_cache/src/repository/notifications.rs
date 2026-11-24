//! An account's WordPress.com notifications (notes), kept as the API sent them, where their sync
//! is up to, and the read and seen changes the server doesn't have yet.

use crate::{RowId, SqliteDbError, repository::QueryExecutor};
use rusqlite::{OptionalExtension, params, params_from_iter, types::Value};
use std::collections::{HashMap, HashSet};
use wp_api::wp_com::notifications::{NotificationId, NotificationType, NotificationWithJson};

/// A note with a time before this many seconds since 1970 has no date: the API sends a time in
/// the first day of 1970 for one.
pub const UNDATED_BEFORE: i64 = 86_400;

/// A stored note: its ID, its JSON, as the API sent it, and whether it shows as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbNotification {
    pub row_id: RowId,
    pub note_id: NotificationId,
    pub json: String,
    /// Whether the note shows as read: a change the server doesn't have yet, if there is one,
    /// and otherwise what the note says. `None` when neither says.
    pub read: Option<bool>,
}

/// Whether a stored note `n` shows as read, in SQL: a read change of `c` waiting to be sent,
/// then a mark of every note up to a time in `s`, then the note's own state.
const SHOWN_READ: &str = "CASE
    WHEN c.wanted IS NOT NULL THEN c.wanted
    WHEN s.read_all_up_to IS NOT NULL AND n.timestamp <= s.read_all_up_to THEN 1
    ELSE n.is_read END";

/// The tables [`SHOWN_READ`] reads, joined to `notifications n`.
const SHOWN_READ_JOINS: &str = "LEFT JOIN notification_read_changes c
    ON c.account_id = n.account_id AND c.note_id = n.note_id
    LEFT JOIN notification_sync_state s ON s.account_id = n.account_id";

/// A note the user marked read or unread that the server doesn't have yet. See the
/// `notification_read_changes` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbNotificationReadChange {
    pub row_id: RowId,
    pub note_id: NotificationId,
    /// Whether the user wants the note read.
    pub wanted: bool,
    /// The order of the last change.
    pub seq: i64,
    /// How many times it's been sent without an answer.
    pub attempts: u32,
    pub last_error: Option<String>,
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
    /// `read` is the SQL that says whether a note is read.
    fn conditions(&self, read: &str, values: &mut Vec<Value>) -> String {
        let mut sql = String::new();
        if self.unread_only {
            sql.push_str(&format!(" AND ({read}) IS NOT 1"));
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

/// When an account's notifications were last synced and looked at, and the changes to all of them
/// the server doesn't have yet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DbNotificationSyncState {
    /// When the newest notes were last synced, in seconds since 1970.
    pub last_synced_at: Option<i64>,
    /// When the user last looked at their notifications, as the API last said, in seconds since
    /// 1970.
    pub last_seen_time: Option<i64>,
    /// A newer time the user saw notes up to that isn't sent yet. Only
    /// [`NotificationRepository::set_seen_time_to_send`] and
    /// [`NotificationRepository::finish_seen`] change it.
    pub seen_time_to_send: Option<i64>,
    /// The user marked every note at or before this time read, and it isn't sent yet. Only
    /// [`NotificationRepository::set_read_all_up_to`] and
    /// [`NotificationRepository::finish_read_all`] change it.
    pub read_all_up_to: Option<i64>,
}

impl DbNotificationSyncState {
    /// The newest time the user saw notes up to, sent or not.
    pub fn seen_time(&self) -> Option<i64> {
        self.last_seen_time.max(self.seen_time_to_send)
    }
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
    /// A note shows as read, and passes an unread filter, as the user last marked it, sent or not.
    pub fn select(
        executor: &impl QueryExecutor,
        account: RowId,
        filter: &DbNotificationFilter,
    ) -> Result<Vec<DbNotification>, SqliteDbError> {
        let mut values = vec![Value::Integer(account.0)];
        let sql = format!(
            "SELECT n.rowid, n.note_id, n.json, {SHOWN_READ} FROM notifications n {SHOWN_READ_JOINS} \
             WHERE n.account_id = ?{} \
             ORDER BY n.timestamp IS NULL, n.timestamp DESC, n.note_id DESC",
            filter.conditions(SHOWN_READ, &mut values)
        );
        let mut statement = executor.prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(values), |row| {
            Ok(DbNotification {
                row_id: row.get(0)?,
                note_id: NotificationId(row.get(1)?),
                json: row.get(2)?,
                read: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// The stored note `note_id`, as [`Self::select`] returns it, or `None` when it isn't stored.
    pub fn select_one(
        executor: &impl QueryExecutor,
        account: RowId,
        note_id: NotificationId,
    ) -> Result<Option<DbNotification>, SqliteDbError> {
        let sql = format!(
            "SELECT n.rowid, n.note_id, n.json, {SHOWN_READ} FROM notifications n {SHOWN_READ_JOINS} \
             WHERE n.account_id = ?1 AND n.note_id = ?2"
        );
        let mut statement = executor.prepare(&sql)?;
        Ok(statement
            .query_row(params![account, note_id.0], |row| {
                Ok(DbNotification {
                    row_id: row.get(0)?,
                    note_id: NotificationId(row.get(1)?),
                    json: row.get(2)?,
                    read: row.get(3)?,
                })
            })
            .optional()?)
    }

    /// The ID and JSON of each of the account's stored notes whose JSON has `text` in it.
    pub fn select_json_containing(
        executor: &impl QueryExecutor,
        account: RowId,
        text: &str,
    ) -> Result<Vec<(NotificationId, String)>, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT note_id, json FROM notifications WHERE account_id = ?1 AND instr(json, ?2) > 0",
        )?;
        let rows = statement.query_map(params![account, text], |row| {
            Ok((NotificationId(row.get(0)?), row.get(1)?))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Replaces a stored note's JSON, keeping its hash and the columns a list sorts and filters
    /// by. For a change the server took that its stored copy doesn't have yet.
    pub fn set_json(
        executor: &impl QueryExecutor,
        account: RowId,
        note_id: NotificationId,
        json: &str,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE notifications SET json = ?3 WHERE account_id = ?1 AND note_id = ?2",
            params![account, note_id.0, json],
        )?;
        Ok(())
    }

    /// The time of the newest dated note that passes `filter`, as [`Self::select`] filters, in
    /// seconds since 1970.
    pub fn newest_time(
        executor: &impl QueryExecutor,
        account: RowId,
        filter: &DbNotificationFilter,
    ) -> Result<Option<i64>, SqliteDbError> {
        let mut values = vec![Value::Integer(account.0), Value::Integer(UNDATED_BEFORE)];
        let sql = format!(
            "SELECT MAX(n.timestamp) FROM notifications n {SHOWN_READ_JOINS} \
             WHERE n.account_id = ? AND n.timestamp >= ?{}",
            filter.conditions(SHOWN_READ, &mut values)
        );
        let mut statement = executor.prepare(&sql)?;
        Ok(statement.query_row(params_from_iter(values), |row| row.get(0))?)
    }

    /// Whether each of `ids` that's stored whole shows as read, as [`Self::select`] shows it, or
    /// `None` when nothing says.
    pub fn shown_read_states(
        executor: &impl QueryExecutor,
        account: RowId,
        ids: &[NotificationId],
    ) -> Result<HashMap<NotificationId, Option<bool>>, SqliteDbError> {
        let mut found = HashMap::new();
        for chunk in ids.chunks(500) {
            let placeholders = vec!["?"; chunk.len()].join(",");
            let mut statement = executor.prepare(&format!(
                "SELECT n.note_id, {SHOWN_READ} FROM notifications n {SHOWN_READ_JOINS}
                 WHERE n.account_id = ? AND n.note_id IN ({placeholders})"
            ))?;
            let values = std::iter::once(Value::Integer(account.0))
                .chain(chunk.iter().map(|id| Value::Integer(id.0 as i64)));
            let rows = statement.query_map(params_from_iter(values), |row| {
                Ok((NotificationId(row.get(0)?), row.get(1)?))
            })?;
            for row in rows {
                let (id, read) = row?;
                found.insert(id, read);
            }
        }
        Ok(found)
    }

    /// The notes that pass `filter` and show as unread, as [`Self::select`] shows them, with a
    /// time at or before `up_to`, newest first. Notes dated in the first day of 1970, which have
    /// no date, are among them.
    pub fn shown_unread(
        executor: &impl QueryExecutor,
        account: RowId,
        filter: &DbNotificationFilter,
        up_to: i64,
    ) -> Result<Vec<NotificationId>, SqliteDbError> {
        let unread = DbNotificationFilter {
            unread_only: true,
            ..filter.clone()
        };
        let mut values = vec![Value::Integer(account.0), Value::Integer(up_to)];
        let sql = format!(
            "SELECT n.note_id FROM notifications n {SHOWN_READ_JOINS} \
             WHERE n.account_id = ? AND n.timestamp <= ?{} \
             ORDER BY n.timestamp DESC, n.note_id DESC",
            unread.conditions(SHOWN_READ, &mut values)
        );
        let mut statement = executor.prepare(&sql)?;
        let ids = statement
            .query_map(params_from_iter(values), |row| {
                Ok(NotificationId(row.get(0)?))
            })?
            .collect::<Result<_, _>>()?;
        Ok(ids)
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
        // What the server last said of the notes: a change it doesn't have yet isn't in its lists.
        let sql = format!(
            "SELECT note_id FROM notifications WHERE account_id = ?{condition}{} \
             ORDER BY timestamp IS NULL, timestamp DESC, note_id DESC",
            filter.conditions("is_read", &mut values)
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
            "SELECT last_synced_at, last_seen_time, seen_time_to_send, read_all_up_to
             FROM notification_sync_state WHERE account_id = ?1",
        )?;
        Ok(statement
            .query_row([account], |row| {
                Ok(DbNotificationSyncState {
                    last_synced_at: row.get(0)?,
                    last_seen_time: row.get(1)?,
                    seen_time_to_send: row.get(2)?,
                    read_all_up_to: row.get(3)?,
                })
            })
            .optional()?
            .unwrap_or_default())
    }

    /// Replaces when the account's notifications were last synced, and moves when the user last
    /// looked at them forward to `state`'s, never back. Leaves the changes waiting to be sent.
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
               last_seen_time = MAX(
                 COALESCE(last_seen_time, excluded.last_seen_time),
                 COALESCE(excluded.last_seen_time, last_seen_time))",
            params![account, state.last_synced_at, state.last_seen_time],
        )?;
        Ok(())
    }

    fn ensure_sync_state(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "INSERT INTO notification_sync_state (account_id) VALUES (?1)
             ON CONFLICT(account_id) DO NOTHING",
            [account],
        )?;
        Ok(())
    }

    /// Records a time the user saw notes up to, to send. A time not after the one already
    /// waiting, or the last seen time, changes nothing. Returns whether it changed anything.
    pub fn set_seen_time_to_send(
        executor: &impl QueryExecutor,
        account: RowId,
        time: i64,
    ) -> Result<bool, SqliteDbError> {
        Self::ensure_sync_state(executor, account)?;
        let changed = executor.execute(
            "UPDATE notification_sync_state SET seen_time_to_send = ?2
             WHERE account_id = ?1
               AND (last_seen_time IS NULL OR last_seen_time < ?2)
               AND (seen_time_to_send IS NULL OR seen_time_to_send < ?2)",
            params![account, time],
        )?;
        Ok(changed > 0)
    }

    /// Records that the server has `sent` as the time the user saw notes up to, or refused it
    /// (`taken` is false). A newer time waiting stays. A time taken moves the last seen time
    /// forward to it, and to `answered`, the time the server answered with.
    pub fn finish_seen(
        executor: &impl QueryExecutor,
        account: RowId,
        sent: i64,
        taken: bool,
        answered: Option<i64>,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE notification_sync_state SET seen_time_to_send = NULL
             WHERE account_id = ?1 AND seen_time_to_send = ?2",
            params![account, sent],
        )?;
        if taken {
            let newest = answered.map_or(sent, |answered| answered.max(sent));
            executor.execute(
                "UPDATE notification_sync_state
                 SET last_seen_time = MAX(COALESCE(last_seen_time, ?2), ?2)
                 WHERE account_id = ?1",
                params![account, newest],
            )?;
        }
        Ok(())
    }

    /// Records that the user marked every note at or before `up_to` read, to send. A time not
    /// after one already waiting changes nothing. The read changes waiting for notes at or before
    /// it go: marking every note read covers them. Returns whether it changed anything.
    pub fn set_read_all_up_to(
        executor: &impl QueryExecutor,
        account: RowId,
        up_to: i64,
    ) -> Result<bool, SqliteDbError> {
        Self::ensure_sync_state(executor, account)?;
        let changed = executor.execute(
            "UPDATE notification_sync_state SET read_all_up_to = ?2
             WHERE account_id = ?1 AND (read_all_up_to IS NULL OR read_all_up_to < ?2)",
            params![account, up_to],
        )?;
        executor.execute(
            "DELETE FROM notification_read_changes WHERE account_id = ?1 AND note_id IN (
               SELECT note_id FROM notifications WHERE account_id = ?1 AND timestamp <= ?2)",
            params![account, up_to],
        )?;
        Ok(changed > 0)
    }

    /// Records that the server took marking every note at or before `sent` read, or refused it.
    /// A newer time waiting stays. Taken, the stored notes at or before it are read.
    pub fn finish_read_all(
        executor: &impl QueryExecutor,
        account: RowId,
        sent: i64,
        taken: bool,
    ) -> Result<(), SqliteDbError> {
        if taken {
            Self::store_read_up_to(executor, account, sent)?;
        }
        executor.execute(
            "UPDATE notification_sync_state SET read_all_up_to = NULL
             WHERE account_id = ?1 AND read_all_up_to = ?2",
            params![account, sent],
        )?;
        Ok(())
    }

    /// Stores the stored notes `ids` as read or unread, in their read state and their JSON, as
    /// the server now has them.
    pub fn store_read(
        executor: &impl QueryExecutor,
        account: RowId,
        ids: &[NotificationId],
        read: bool,
    ) -> Result<(), SqliteDbError> {
        for id in ids {
            executor.execute(
                "UPDATE notifications SET is_read = ?3, json = json_set(json, '$.read', ?3)
                 WHERE account_id = ?1 AND note_id = ?2",
                params![account, id.0, i64::from(read)],
            )?;
        }
        Ok(())
    }

    /// Stores every stored note at or before `up_to` as read, as [`Self::store_read`] does.
    fn store_read_up_to(
        executor: &impl QueryExecutor,
        account: RowId,
        up_to: i64,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE notifications SET is_read = 1, json = json_set(json, '$.read', 1)
             WHERE account_id = ?1 AND timestamp <= ?2 AND is_read IS NOT 1",
            params![account, up_to],
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

const READ_CHANGE_COLUMNS: &str = "rowid, note_id, wanted, seq, attempts, last_error";

fn read_change_from_row(row: &rusqlite::Row) -> rusqlite::Result<DbNotificationReadChange> {
    Ok(DbNotificationReadChange {
        row_id: row.get(0)?,
        note_id: NotificationId(row.get(1)?),
        wanted: row.get(2)?,
        seq: row.get(3)?,
        attempts: row.get(4)?,
        last_error: row.get(5)?,
    })
}

/// The notes the user marked read or unread that the server doesn't have yet. A row is there from
/// the change until the server answers it.
pub struct NotificationReadChangeRepository;

impl NotificationReadChangeRepository {
    /// Records that the user wants a note read or unread. A row already waiting for the note
    /// takes the new answer and goes to the back of the line. Returns the row.
    pub fn record(
        executor: &impl QueryExecutor,
        account: RowId,
        note_id: NotificationId,
        wanted: bool,
    ) -> Result<DbNotificationReadChange, SqliteDbError> {
        let mut statement = executor.prepare(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM notification_read_changes WHERE account_id = ?1",
        )?;
        let seq: i64 = statement.query_row([account], |row| row.get(0))?;
        executor.execute(
            "INSERT INTO notification_read_changes (account_id, note_id, wanted, seq)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(account_id, note_id) DO UPDATE SET
               wanted = excluded.wanted,
               seq = excluded.seq,
               attempts = 0,
               last_error = NULL",
            params![account, note_id.0, wanted, seq],
        )?;
        Self::select(executor, account, note_id)?.ok_or_else(|| {
            SqliteDbError::SqliteError(format!(
                "The read change of note {} wasn't stored",
                note_id.0
            ))
        })
    }

    /// The row waiting for a note, if there is one.
    pub fn select(
        executor: &impl QueryExecutor,
        account: RowId,
        note_id: NotificationId,
    ) -> Result<Option<DbNotificationReadChange>, SqliteDbError> {
        let mut statement = executor.prepare(&format!(
            "SELECT {READ_CHANGE_COLUMNS} FROM notification_read_changes
             WHERE account_id = ?1 AND note_id = ?2"
        ))?;
        Ok(statement
            .query_row(params![account, note_id.0], read_change_from_row)
            .optional()?)
    }

    /// Every row waiting, oldest change first.
    pub fn select_all(
        executor: &impl QueryExecutor,
        account: RowId,
    ) -> Result<Vec<DbNotificationReadChange>, SqliteDbError> {
        let mut statement = executor.prepare(&format!(
            "SELECT {READ_CHANGE_COLUMNS} FROM notification_read_changes
             WHERE account_id = ?1 ORDER BY seq"
        ))?;
        let rows = statement.query_map([account], read_change_from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// How many rows are waiting.
    pub fn count(executor: &impl QueryExecutor, account: RowId) -> Result<u64, SqliteDbError> {
        let mut statement = executor
            .prepare("SELECT COUNT(*) FROM notification_read_changes WHERE account_id = ?1")?;
        Ok(statement.query_row([account], |row| row.get(0))?)
    }

    /// Removes a row the server answered, unless the user changed the note again since it was
    /// sent: `seq` is the row's when it was sent. Returns whether it removed it.
    pub fn finish(
        executor: &impl QueryExecutor,
        row: RowId,
        seq: i64,
    ) -> Result<bool, SqliteDbError> {
        let removed = executor.execute(
            "DELETE FROM notification_read_changes WHERE rowid = ?1 AND seq = ?2",
            params![row, seq],
        )?;
        Ok(removed > 0)
    }

    /// Records that a row was sent and got no answer.
    pub fn record_attempt(
        executor: &impl QueryExecutor,
        row: RowId,
        error: &str,
    ) -> Result<(), SqliteDbError> {
        executor.execute(
            "UPDATE notification_read_changes SET attempts = attempts + 1, last_error = ?2
             WHERE rowid = ?1",
            params![row, error],
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
        NotificationReadChangeRepository::record(&conn, first, id(1), false).unwrap();
        NotificationRepository::upsert(&conn, second, &note(1, 200_000, 11, true, "like")).unwrap();

        WordPressComAccountRepository::delete(&conn, WpComUserId(1)).unwrap();

        for table in [
            "notifications",
            "notification_listed_only",
            "notification_sync_state",
            "notification_tab_state",
            "notification_read_changes",
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
            ..Default::default()
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

    /// Whether each stored note shows as read, newest first, by `n`.
    fn shown_read(conn: &Connection, account: RowId) -> Vec<(u64, Option<bool>)> {
        NotificationRepository::select(conn, account, &DbNotificationFilter::default())
            .unwrap()
            .iter()
            .map(|note| (note.note_id.0 - 10_000_000_000, note.read))
            .collect()
    }

    fn unread() -> DbNotificationFilter {
        DbNotificationFilter {
            unread_only: true,
            ..Default::default()
        }
    }

    #[test]
    fn test_a_read_change_waiting_shows_over_the_server_copy() {
        let conn = conn();
        let account = account(&conn, 1);
        NotificationRepository::upsert(&conn, account, &note(1, 200_000, 1, false, "like"))
            .unwrap();
        NotificationRepository::upsert(&conn, account, &note(2, 300_000, 2, true, "like")).unwrap();
        NotificationReadChangeRepository::record(&conn, account, id(1), true).unwrap();
        NotificationReadChangeRepository::record(&conn, account, id(2), false).unwrap();
        assert_eq!(
            shown_read(&conn, account),
            [(2, Some(false)), (1, Some(true))]
        );
        assert_eq!(stored(&conn, account, &unread()), [2]);

        // A sync stores the server's copies, which are older than the changes.
        NotificationRepository::upsert(&conn, account, &note(1, 200_000, 3, false, "like"))
            .unwrap();
        NotificationRepository::upsert(&conn, account, &note(2, 300_000, 4, true, "like")).unwrap();
        assert_eq!(
            shown_read(&conn, account),
            [(2, Some(false)), (1, Some(true))]
        );

        // What the server last said decides which notes its lists should have.
        let unlisted = NotificationRepository::unlisted(
            &conn,
            account,
            DbNotificationWindow::Everything,
            &unread(),
            &HashSet::new(),
        )
        .unwrap();
        assert_eq!(unlisted, [id(1)]);
    }

    #[test]
    fn test_a_read_state_the_server_took_is_stored_in_the_note() {
        let conn = conn();
        let account = account(&conn, 1);
        NotificationRepository::upsert(&conn, account, &note(1, 200_000, 1, false, "like"))
            .unwrap();
        NotificationRepository::store_read(&conn, account, &[id(1), id(9)], true).unwrap();
        let stored =
            &NotificationRepository::select(&conn, account, &Default::default()).unwrap()[0];
        assert_eq!(stored.read, Some(true));
        let json: serde_json::Value = serde_json::from_str(&stored.json).unwrap();
        assert_eq!(json["read"], 1);
        assert_eq!(json["a_later_field"], "kept");
        assert_eq!(
            NotificationRepository::read_states(&conn, account, &[id(1)]).unwrap()[&id(1)],
            Some(true)
        );
    }

    #[test]
    fn test_a_read_change_is_one_row_per_note_sent_in_order() {
        let conn = conn();
        let account = account(&conn, 1);
        let first = NotificationReadChangeRepository::record(&conn, account, id(1), true).unwrap();
        NotificationReadChangeRepository::record(&conn, account, id(2), true).unwrap();
        NotificationReadChangeRepository::record_attempt(&conn, first.row_id, "offline").unwrap();
        let again = NotificationReadChangeRepository::record(&conn, account, id(1), false).unwrap();
        assert_eq!(again.row_id, first.row_id);
        assert!(!again.wanted && again.seq > first.seq);
        assert_eq!((again.attempts, again.last_error), (0, None));
        let order: Vec<_> = NotificationReadChangeRepository::select_all(&conn, account)
            .unwrap()
            .iter()
            .map(|row| row.note_id)
            .collect();
        assert_eq!(order, [id(2), id(1)]);

        // An answer to the row as it was sent before the second change leaves it.
        assert!(!NotificationReadChangeRepository::finish(&conn, first.row_id, first.seq).unwrap());
        assert!(NotificationReadChangeRepository::finish(&conn, again.row_id, again.seq).unwrap());
        assert_eq!(
            NotificationReadChangeRepository::count(&conn, account).unwrap(),
            1
        );
    }

    #[test]
    fn test_marking_every_note_read_up_to_a_time() {
        let conn = conn();
        let account = account(&conn, 1);
        for note in [
            note(1, 0, 1, false, "like"),
            note(2, 200_000, 2, false, "like"),
            note(3, 300_000, 3, false, "comment"),
        ] {
            NotificationRepository::upsert(&conn, account, &note).unwrap();
        }
        NotificationReadChangeRepository::record(&conn, account, id(2), false).unwrap();
        NotificationReadChangeRepository::record(&conn, account, id(3), false).unwrap();
        assert_eq!(
            NotificationRepository::shown_unread(&conn, account, &Default::default(), 200_000)
                .unwrap(),
            [id(2), id(1)]
        );

        assert!(NotificationRepository::set_read_all_up_to(&conn, account, 200_000).unwrap());
        // Marking every note read covers the change waiting for an older note.
        assert_eq!(
            NotificationReadChangeRepository::select_all(&conn, account)
                .unwrap()
                .iter()
                .map(|row| row.note_id)
                .collect::<Vec<_>>(),
            [id(3)]
        );
        assert_eq!(
            shown_read(&conn, account),
            [(3, Some(false)), (2, Some(true)), (1, Some(true))]
        );
        assert_eq!(stored(&conn, account, &unread()), [3]);
        // An older time waits behind the newer one.
        assert!(!NotificationRepository::set_read_all_up_to(&conn, account, 100_000).unwrap());
        assert_eq!(
            NotificationRepository::sync_state(&conn, account)
                .unwrap()
                .read_all_up_to,
            Some(200_000)
        );

        NotificationRepository::finish_read_all(&conn, account, 200_000, true).unwrap();
        assert_eq!(
            NotificationRepository::sync_state(&conn, account)
                .unwrap()
                .read_all_up_to,
            None
        );
        assert_eq!(
            NotificationRepository::read_states(&conn, account, &[id(1), id(2), id(3)]).unwrap(),
            HashMap::from([
                (id(1), Some(true)),
                (id(2), Some(true)),
                (id(3), Some(false))
            ])
        );
        assert_eq!(
            NotificationRepository::newest_time(&conn, account, &unread()).unwrap(),
            Some(300_000)
        );
    }

    #[test]
    fn test_a_refused_read_all_shows_the_notes_as_they_were() {
        let conn = conn();
        let account = account(&conn, 1);
        NotificationRepository::upsert(&conn, account, &note(1, 200_000, 1, false, "like"))
            .unwrap();
        NotificationRepository::set_read_all_up_to(&conn, account, 200_000).unwrap();
        NotificationRepository::finish_read_all(&conn, account, 200_000, false).unwrap();
        assert_eq!(shown_read(&conn, account), [(1, Some(false))]);
    }

    #[test]
    fn test_the_seen_time_only_moves_forward() {
        let conn = conn();
        let account = account(&conn, 1);
        let state = |conn: &Connection| NotificationRepository::sync_state(conn, account).unwrap();
        NotificationRepository::set_sync_state(
            &conn,
            account,
            &DbNotificationSyncState {
                last_seen_time: Some(1_000),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!NotificationRepository::set_seen_time_to_send(&conn, account, 1_000).unwrap());
        assert!(NotificationRepository::set_seen_time_to_send(&conn, account, 2_000).unwrap());
        assert!(!NotificationRepository::set_seen_time_to_send(&conn, account, 1_500).unwrap());
        assert_eq!(state(&conn).seen_time(), Some(2_000));

        // A sync that brings an older time, or none, moves nothing back, and leaves what waits.
        for last_seen_time in [Some(500), None] {
            NotificationRepository::set_sync_state(
                &conn,
                account,
                &DbNotificationSyncState {
                    last_seen_time,
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(state(&conn).last_seen_time, Some(1_000));
            assert_eq!(state(&conn).seen_time_to_send, Some(2_000));
        }

        // A newer time recorded while 2,000 was being sent stays to be sent.
        NotificationRepository::set_seen_time_to_send(&conn, account, 3_000).unwrap();
        NotificationRepository::finish_seen(&conn, account, 2_000, true, Some(1_900)).unwrap();
        assert_eq!(state(&conn).last_seen_time, Some(2_000));
        assert_eq!(state(&conn).seen_time_to_send, Some(3_000));
        // Refused, it goes, and the time stays where the server had it.
        NotificationRepository::finish_seen(&conn, account, 3_000, false, None).unwrap();
        assert_eq!(state(&conn).seen_time(), Some(2_000));
    }
}
