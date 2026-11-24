//! Marking notes read, unread and seen.
//!
//! Each change works the same way. It's written to the cache first, so every list shows it at
//! once and observers hear of it, and then everything waiting is sent, oldest first, holding the
//! service's lane, so a send never overlaps a sync.
//!
//! - When the server takes a change, the read state it now has is stored in the note, and the
//!   change goes.
//! - When the server refuses it for good (an answer in the 400s other than a rejected sign-in, a
//!   timeout or a rate limit, or a success that says it didn't make the change), the change goes,
//!   the cache shows the note as the server has it, and the call that made it returns the error.
//! - When no answer comes, such as offline, the change stays, and
//!   [`NotificationService::send_pending_changes`] or the next change sends it.
//!
//! A sync that stores the server's copy of a note while a change waits doesn't undo it: the
//! cached lists show the waiting change over the note's own read state.

use super::{NotificationService, NotificationServiceError, date};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use wp_api::date::WpGmtDateTime;
use wp_api::prelude::WpApiError;
use wp_api::wp_com::language::{WPComLanguage, WpComLanguageProvider};
use wp_api::wp_com::notifications::{
    NotificationId, NotificationReadChange, NotificationTab, NotificationsReadAllParams,
    NotificationsReadParams, NotificationsSeenParams,
};
use wp_mobile_cache::repository::notifications::{
    DbNotificationFilter, DbNotificationReadChange, NotificationReadChangeRepository,
    NotificationRepository,
};
use wp_mobile_cache::{RowId, SqliteDbError};

/// What the requests name the client as. The API uses it for its stats only.
const SOURCE: &str = "mobile";

/// How many notes one request marks.
const MARK_AT_A_TIME: usize = 100;

/// What became of a change to notes' read or seen state. It shows in the cache at once either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum NotificationChangeOutcome {
    /// The server has it.
    Sent,
    /// It's kept on this device, and sent by [`NotificationService::send_pending_changes`] or the
    /// next change: the device is offline, or no answer came.
    Waiting,
    /// There was nothing to change: the notes already show that way, there are no notes to mark,
    /// or the time isn't newer than the one the user already saw.
    Unchanged,
}

/// The service's language, for the client changes are sent with.
#[derive(Debug)]
pub(super) struct ServiceLanguage(pub(super) Arc<Mutex<Option<WPComLanguage>>>);

impl WpComLanguageProvider for ServiceLanguage {
    fn current_language(&self) -> Option<WPComLanguage> {
        Some(locale(&self.0))
    }
}

/// The language every request is sent in: the one set, or English.
pub(super) fn locale(language: &Mutex<Option<WPComLanguage>>) -> WPComLanguage {
    language
        .lock()
        .expect("language lock poisoned")
        .unwrap_or(WPComLanguage::English)
}

/// A change a call waits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Change {
    /// A note's read change, as the row was when the call recorded it.
    Note { note_id: NotificationId, seq: i64 },
    /// Every note at or before a time marked read.
    ReadAll { up_to: i64 },
    /// The time the user saw notes up to.
    Seen { time: i64 },
}

/// How the server answered a change.
#[derive(Debug)]
enum Answer {
    Taken,
    /// Refused, with the server's error when this change was the first of its request to be
    /// refused. A request refused as a whole refuses every change in it, and an error can only
    /// be returned once.
    Refused(Option<Box<WpApiError>>),
}

/// The changes calls wait on, and the server's answer to each once it comes. Whichever call holds
/// the lane sends every change waiting, so a change's answer can reach a call other than its own.
#[derive(Debug, Default)]
pub(super) struct Answers(Mutex<HashMap<Change, Option<Answer>>>);

impl Answers {
    fn records(&self, change: &Change) -> bool {
        self.0
            .lock()
            .expect("answers lock poisoned")
            .contains_key(change)
    }

    fn record(&self, change: Change, answer: Answer) {
        if let Some(slot) = self
            .0
            .lock()
            .expect("answers lock poisoned")
            .get_mut(&change)
        {
            *slot = Some(answer);
        }
    }
}

/// Marks changes as waited on while a call runs. Dropping it, including when the call is
/// cancelled, forgets them.
struct Waiting<'a> {
    answers: &'a Answers,
    changes: Vec<Change>,
}

impl<'a> Waiting<'a> {
    fn new(answers: &'a Answers, changes: Vec<Change>) -> Self {
        let mut map = answers.0.lock().expect("answers lock poisoned");
        for change in &changes {
            map.insert(*change, None);
        }
        drop(map);
        Self { answers, changes }
    }

    fn take(&self, change: &Change) -> Option<Answer> {
        self.answers
            .0
            .lock()
            .expect("answers lock poisoned")
            .get_mut(change)
            .and_then(Option::take)
    }
}

impl Drop for Waiting<'_> {
    fn drop(&mut self) {
        let mut map = self.answers.0.lock().expect("answers lock poisoned");
        for change in &self.changes {
            map.remove(change);
        }
    }
}

/// Whether the server refused a request for good, rather than it failing on the way or for now.
/// A timeout and a rate limit pass, and a rejected sign-in is no answer to the change: it's sent
/// again once the user signs in again. Any other answer in the 400s won't change by asking again.
fn is_refused(error: &WpApiError) -> bool {
    /// Request timeout, too many requests.
    const TEMPORARY: [u32; 2] = [408, 429];
    if crate::service::is_sign_in_rejected(error) {
        return false;
    }
    match error {
        WpApiError::WpError { status_code, .. }
        | WpApiError::InvalidHttpStatusCode { status_code, .. }
        | WpApiError::UnknownError { status_code, .. } => {
            (400..500).contains(status_code) && !TEMPORARY.contains(status_code)
        }
        _ => false,
    }
}

/// What a call tells its caller, from each of its changes: sent (`Ok(true)`), still waiting
/// (`Ok(false)`), or refused, with the server's error when the call has it. A refusal with an
/// error wins over one without.
fn outcome(
    answers: Vec<Result<bool, Option<Box<WpApiError>>>>,
) -> Result<NotificationChangeOutcome, NotificationServiceError> {
    let mut waiting = false;
    let mut refused: Option<NotificationServiceError> = None;
    for answer in answers {
        match answer {
            Ok(true) => {}
            Ok(false) => waiting = true,
            Err(Some(error)) => refused = Some(NotificationServiceError::Api(*error)),
            Err(None) => {
                refused.get_or_insert(NotificationServiceError::ChangeNotMade);
            }
        }
    }
    match (refused, waiting) {
        (Some(error), _) => Err(error),
        (None, true) => Ok(NotificationChangeOutcome::Waiting),
        (None, false) => Ok(NotificationChangeOutcome::Sent),
    }
}

#[uniffi::export]
impl NotificationService {
    /// Marks notes read. Each shows as read at once, in every list and tab, and leaves the Unread
    /// tab's cached list; then it's sent with everything else waiting.
    ///
    /// Returns [`NotificationChangeOutcome::Waiting`] when it couldn't be sent yet, and
    /// [`NotificationChangeOutcome::Unchanged`] when every note already shows as read. Fails when
    /// the server refuses it, and the notes then show as the server has them.
    pub async fn mark_read(
        &self,
        note_ids: Vec<NotificationId>,
    ) -> Result<NotificationChangeOutcome, NotificationServiceError> {
        self.set_read(note_ids, true).await
    }

    /// Marks notes unread, as [`Self::mark_read`] marks them read. Each joins the Unread tab's
    /// cached list at once.
    pub async fn mark_unread(
        &self,
        note_ids: Vec<NotificationId>,
    ) -> Result<NotificationChangeOutcome, NotificationServiceError> {
        self.set_read(note_ids, false).await
    }

    /// Marks every note of `tab` at or before `up_to` read, cached or not. Without `up_to`, it's
    /// the time of the tab's newest cached note, so a note that arrives after the user asked stays
    /// unread.
    ///
    /// For the All and Unread tabs it's one request for every note of the account, which needs
    /// no list of them. The other tabs mark the tab's cached unread notes, as [`Self::mark_read`]
    /// does. Returns [`NotificationChangeOutcome::Unchanged`] when there's nothing to mark.
    pub async fn mark_all_read(
        &self,
        tab: NotificationTab,
        up_to: Option<WpGmtDateTime>,
    ) -> Result<NotificationChangeOutcome, NotificationServiceError> {
        let filter: DbNotificationFilter = super::tab_filter(tab).into();
        let up_to = match up_to {
            Some(up_to) => Some(up_to.0.timestamp()),
            None => self.cache.execute(|connection| {
                NotificationRepository::newest_time(&*connection, self.account, &filter)
            })?,
        };
        let Some(up_to) = up_to else {
            return Ok(NotificationChangeOutcome::Unchanged);
        };
        if !matches!(tab, NotificationTab::All | NotificationTab::Unread) {
            let ids = self.cache.execute(|connection| {
                NotificationRepository::shown_unread(&*connection, self.account, &filter, up_to)
            })?;
            return self.set_read(ids, true).await;
        }

        let waiting_up_to = self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            NotificationRepository::set_read_all_up_to(&transaction, self.account, up_to)?;
            let state = NotificationRepository::sync_state(&transaction, self.account)?;
            transaction.commit()?;
            Ok::<_, SqliteDbError>(state.read_all_up_to)
        })?;
        // A newer time waiting covers this one.
        let change = Change::ReadAll {
            up_to: waiting_up_to.unwrap_or(up_to),
        };
        self.send_and_wait(vec![change]).await
    }

    /// Records that the user has seen the notes up to `time`, which clears the unseen flag:
    /// [`Self::has_unseen_notes`] is false at once, and the time is then sent. Without `time`,
    /// it's the time of the newest cached note: call it when the notifications list is shown.
    ///
    /// The time only moves forward. One that isn't after the newest the service knows, sent or
    /// not, changes nothing, sends nothing, and returns
    /// [`NotificationChangeOutcome::Unchanged`]. This can't be undone.
    pub async fn mark_seen(
        &self,
        time: Option<WpGmtDateTime>,
    ) -> Result<NotificationChangeOutcome, NotificationServiceError> {
        let time = match time {
            Some(time) => Some(time.0.timestamp()),
            None => self.cache.execute(|connection| {
                NotificationRepository::newest_time(
                    &*connection,
                    self.account,
                    &DbNotificationFilter::default(),
                )
            })?,
        };
        let Some(time) = time else {
            return Ok(NotificationChangeOutcome::Unchanged);
        };
        let changed = self.cache.execute(|connection| {
            NotificationRepository::set_seen_time_to_send(&*connection, self.account, time)
        })?;
        if !changed {
            return Ok(NotificationChangeOutcome::Unchanged);
        }
        self.send_and_wait(vec![Change::Seen { time }]).await
    }

    /// Whether the cache has a note newer than the newest the user has seen, sent or not: the
    /// flag that lights the notifications bell. Doesn't touch the network.
    ///
    /// It knows only the cached notes. [`Self::fetch_has_unseen_notes`] asks the server, which
    /// knows them all.
    pub fn has_unseen_notes(&self) -> Result<bool, NotificationServiceError> {
        Ok(self.cache.execute(|connection| {
            let seen = NotificationRepository::sync_state(&*connection, self.account)?.seen_time();
            let newest = NotificationRepository::newest_time(
                &*connection,
                self.account,
                &DbNotificationFilter::default(),
            )?;
            Ok::<_, SqliteDbError>(match (newest, seen) {
                (None, _) => false,
                (Some(newest), seen) => seen.is_none_or(|seen| newest > seen),
            })
        })?)
    }

    /// Asks the server whether the account has notes the user hasn't seen, as the WordPress iOS
    /// app lights its bell. Sends the changes waiting first. While a seen time still waits to be
    /// sent, the server can't know it, so the answer is [`Self::has_unseen_notes`]'s.
    pub async fn fetch_has_unseen_notes(&self) -> Result<bool, NotificationServiceError> {
        {
            let _lane = self.lane.lock().await;
            self.send_waiting().await?;
        }
        let state = self
            .cache
            .execute(|connection| NotificationRepository::sync_state(&*connection, self.account))?;
        if state.seen_time_to_send.is_some() {
            return self.has_unseen_notes();
        }
        Ok(self.client.me().unseen_notes().await?.data.has_unseen_notes)
    }

    /// Sends the changes waiting, such as notes marked read while offline, oldest first. Returns
    /// how many are still waiting. Call it at launch, when the app comes to the foreground, and
    /// when the device comes back online.
    ///
    /// A change the server refuses for good is dropped, and the notes show as the server has
    /// them. It's logged, and no call returns it.
    pub async fn send_pending_changes(&self) -> Result<u64, NotificationServiceError> {
        {
            let _lane = self.lane.lock().await;
            self.send_waiting().await?;
        }
        self.pending_change_count()
    }

    /// How many changes are waiting to be sent: one for each note marked read or unread, one
    /// for marking every note read, and one for a seen time. Doesn't touch the network.
    pub fn pending_change_count(&self) -> Result<u64, NotificationServiceError> {
        Ok(self.cache.execute(|connection| {
            let state = NotificationRepository::sync_state(&*connection, self.account)?;
            Ok::<_, SqliteDbError>(
                NotificationReadChangeRepository::count(&*connection, self.account)?
                    + u64::from(state.read_all_up_to.is_some())
                    + u64::from(state.seen_time_to_send.is_some()),
            )
        })?)
    }
}

impl NotificationService {
    /// Records the notes as the user wants them, then sends them with everything waiting.
    async fn set_read(
        &self,
        note_ids: Vec<NotificationId>,
        read: bool,
    ) -> Result<NotificationChangeOutcome, NotificationServiceError> {
        let mut seen = HashSet::new();
        let note_ids: Vec<NotificationId> =
            note_ids.into_iter().filter(|id| seen.insert(*id)).collect();
        let changes = self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            let shown =
                NotificationRepository::shown_read_states(&transaction, self.account, &note_ids)?;
            let mut changes = Vec::new();
            for note_id in &note_ids {
                // A note the cache shows that way already needs nothing. One it doesn't have, or
                // that doesn't say, is sent.
                if shown.get(note_id) == Some(&Some(read)) {
                    continue;
                }
                let row = NotificationReadChangeRepository::record(
                    &transaction,
                    self.account,
                    *note_id,
                    read,
                )?;
                changes.push(Change::Note {
                    note_id: *note_id,
                    seq: row.seq,
                });
            }
            transaction.commit()?;
            Ok::<_, SqliteDbError>(changes)
        })?;
        if changes.is_empty() {
            return Ok(NotificationChangeOutcome::Unchanged);
        }
        self.send_and_wait(changes).await
    }

    /// Waits on `changes`, sends everything waiting, and says how the server answered them.
    async fn send_and_wait(
        &self,
        changes: Vec<Change>,
    ) -> Result<NotificationChangeOutcome, NotificationServiceError> {
        let waiting = Waiting::new(&self.answers, changes.clone());
        {
            let _lane = self.lane.lock().await;
            self.send_waiting().await?;
        }
        let mut answers = Vec::new();
        for change in &changes {
            answers.push(match waiting.take(change) {
                Some(Answer::Taken) => Ok(true),
                Some(Answer::Refused(error)) => Err(error),
                // No answer reached this call: the change is still waiting, or a newer change to
                // the same note replaced it, and that one's call has its answer.
                None => Ok(!self.still_waiting(change)?),
            });
        }
        outcome(answers)
    }

    /// Whether `change` is still waiting to be sent.
    fn still_waiting(&self, change: &Change) -> Result<bool, SqliteDbError> {
        self.cache.execute(|connection| match change {
            Change::Note { note_id, seq } => {
                Ok(
                    NotificationReadChangeRepository::select(&*connection, self.account, *note_id)?
                        .is_some_and(|row| row.seq == *seq),
                )
            }
            Change::ReadAll { up_to } => {
                Ok(
                    NotificationRepository::sync_state(&*connection, self.account)?.read_all_up_to
                        == Some(*up_to),
                )
            }
            Change::Seen { time } => Ok(NotificationRepository::sync_state(
                &*connection,
                self.account,
            )?
            .seen_time_to_send
                == Some(*time)),
        })
    }

    /// Sends everything waiting: marking every note read, then the notes' read changes oldest
    /// first, then the seen time. Stops at the first request that gets no answer, since the next
    /// would fare no better. Call it holding the lane.
    async fn send_waiting(&self) -> Result<(), SqliteDbError> {
        if !self.send_read_all().await? {
            return Ok(());
        }
        if !self.send_read_changes().await? {
            return Ok(());
        }
        self.send_seen().await?;
        Ok(())
    }

    /// Sends marking every note read, if it waits. Returns whether to go on.
    async fn send_read_all(&self) -> Result<bool, SqliteDbError> {
        let state = self
            .cache
            .execute(|connection| NotificationRepository::sync_state(&*connection, self.account))?;
        let Some(up_to) = state.read_all_up_to else {
            return Ok(true);
        };
        let change = Change::ReadAll { up_to };
        // Sent with no time, it would mark every note read.
        let Some(time) = date(up_to) else {
            log::error!(
                "Dropping marking every notification read up to a time that isn't a date: {up_to}"
            );
            self.cache.execute(|connection| {
                NotificationRepository::finish_read_all(&*connection, self.account, up_to, false)
            })?;
            self.answered(change, Answer::Refused(None));
            return Ok(true);
        };
        let answer = self
            .write_client
            .notifications()
            .mark_all_read(&NotificationsReadAllParams {
                time: Some(time),
                source: Some(SOURCE.to_string()),
            })
            .await;
        let (taken, answer) = match answer {
            Ok(response) if response.data.success => (true, Answer::Taken),
            Ok(_) => (false, Answer::Refused(None)),
            Err(error) if is_refused(&error) => (false, Answer::Refused(Some(Box::new(error)))),
            Err(error) => {
                log::info!("Marking every notification read is waiting: {error}");
                return Ok(false);
            }
        };
        self.cache.execute(|connection| {
            NotificationRepository::finish_read_all(&*connection, self.account, up_to, taken)
        })?;
        self.answered(change, answer);
        Ok(true)
    }

    /// Sends the notes' read changes, a request at a time, oldest first. Returns whether to go
    /// on.
    async fn send_read_changes(&self) -> Result<bool, SqliteDbError> {
        // Rows sent this round, as they were when sent. A row changed since is sent again.
        let mut sent: HashSet<(RowId, i64)> = HashSet::new();
        loop {
            let rows: Vec<DbNotificationReadChange> = self
                .cache
                .execute(|connection| {
                    NotificationReadChangeRepository::select_all(&*connection, self.account)
                })?
                .into_iter()
                .filter(|row| !sent.contains(&(row.row_id, row.seq)))
                .take(MARK_AT_A_TIME)
                .collect();
            if rows.is_empty() {
                return Ok(true);
            }
            sent.extend(rows.iter().map(|row| (row.row_id, row.seq)));
            let answer = self
                .write_client
                .notifications()
                .mark_read(&NotificationsReadParams {
                    changes: rows
                        .iter()
                        .map(|row| NotificationReadChange {
                            note_id: row.note_id,
                            read: row.wanted,
                        })
                        .collect(),
                    source: Some(SOURCE.to_string()),
                })
                .await;
            match answer {
                Ok(response) if response.data.success => {
                    self.read_changes_taken(&rows, response.data.updated.as_deref())?
                }
                Ok(_) => self.read_changes_refused(&rows, None)?,
                Err(error) if is_refused(&error) => {
                    self.read_changes_refused(&rows, Some(error))?
                }
                Err(error) => {
                    let message = error.to_string();
                    log::info!(
                        "{} notification read changes are waiting: {message}",
                        rows.len()
                    );
                    self.cache.execute(|connection| {
                        for row in &rows {
                            NotificationReadChangeRepository::record_attempt(
                                &*connection,
                                row.row_id,
                                &message,
                            )?;
                        }
                        Ok::<_, SqliteDbError>(())
                    })?;
                    return Ok(false);
                }
            }
        }
    }

    /// Records that the server took `rows`. `updated`, when the answer has it, lists the notes
    /// that are unread afterwards, which is what the server has, whatever was asked.
    fn read_changes_taken(
        &self,
        rows: &[DbNotificationReadChange],
        updated: Option<&[NotificationId]>,
    ) -> Result<(), SqliteDbError> {
        let mut answers = Vec::new();
        self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            for row in rows {
                let read = updated.map_or(row.wanted, |unread| !unread.contains(&row.note_id));
                // The server has `read` now, whether or not the user changed the note since. A
                // newer change still waiting shows over it.
                NotificationRepository::store_read(
                    &transaction,
                    self.account,
                    &[row.note_id],
                    read,
                )?;
                NotificationReadChangeRepository::finish(&transaction, row.row_id, row.seq)?;
                answers.push((
                    Change::Note {
                        note_id: row.note_id,
                        seq: row.seq,
                    },
                    if read == row.wanted {
                        Answer::Taken
                    } else {
                        Answer::Refused(None)
                    },
                ));
            }
            transaction.commit()?;
            Ok::<_, SqliteDbError>(())
        })?;
        for (change, answer) in answers {
            self.answered(change, answer);
        }
        Ok(())
    }

    /// Records that the server refused `rows`: each goes, unless the user changed the note again
    /// since, and the note shows as the cache had it from the server.
    fn read_changes_refused(
        &self,
        rows: &[DbNotificationReadChange],
        error: Option<WpApiError>,
    ) -> Result<(), SqliteDbError> {
        let mut error = error.map(Box::new);
        self.cache.execute(|connection| {
            let transaction = connection.transaction()?;
            for row in rows {
                NotificationReadChangeRepository::finish(&transaction, row.row_id, row.seq)?;
            }
            transaction.commit()?;
            Ok::<_, SqliteDbError>(())
        })?;
        for row in rows {
            let change = Change::Note {
                note_id: row.note_id,
                seq: row.seq,
            };
            // The error goes to the first change a call waits on.
            let error = if self.answers.records(&change) {
                error.take()
            } else {
                None
            };
            self.answered(change, Answer::Refused(error));
        }
        Ok(())
    }

    /// Sends the seen time, if one waits. A time that's no longer newer than the server's, which
    /// a sync can bring after it was recorded, is dropped without being sent.
    async fn send_seen(&self) -> Result<(), SqliteDbError> {
        let state = self
            .cache
            .execute(|connection| NotificationRepository::sync_state(&*connection, self.account))?;
        let Some(time) = state.seen_time_to_send else {
            return Ok(());
        };
        let change = Change::Seen { time };
        if state.last_seen_time.is_some_and(|last| last >= time) {
            self.cache.execute(|connection| {
                NotificationRepository::finish_seen(&*connection, self.account, time, true, None)
            })?;
            self.answered(change, Answer::Taken);
            return Ok(());
        }
        let Some(seen_time) = date(time) else {
            log::error!("Dropping a notification seen time that isn't a date: {time}");
            self.cache.execute(|connection| {
                NotificationRepository::finish_seen(&*connection, self.account, time, false, None)
            })?;
            self.answered(change, Answer::Refused(None));
            return Ok(());
        };
        let answer = self
            .write_client
            .notifications()
            .mark_seen(&NotificationsSeenParams {
                time: seen_time,
                source: Some(SOURCE.to_string()),
            })
            .await;
        let (taken, answered, answer) = match answer {
            Ok(response) if response.data.success => (
                true,
                response.data.last_seen_time.map(|time| time.0.timestamp()),
                Answer::Taken,
            ),
            Ok(_) => (false, None, Answer::Refused(None)),
            Err(error) if is_refused(&error) => {
                (false, None, Answer::Refused(Some(Box::new(error))))
            }
            Err(error) => {
                log::info!("A notification seen time is waiting: {error}");
                return Ok(());
            }
        };
        self.cache.execute(|connection| {
            NotificationRepository::finish_seen(&*connection, self.account, time, taken, answered)
        })?;
        self.answered(change, answer);
        Ok(())
    }

    /// Gives a call waiting on `change` the server's answer, or logs a refusal no call waits on.
    fn answered(&self, change: Change, answer: Answer) {
        if !self.answers.records(&change) {
            if let Answer::Refused(error) = &answer {
                log::warn!(
                    "The server refused a notification change sent later: {}",
                    error
                        .as_ref()
                        .map_or("it answered without making it".to_string(), |e| e
                            .to_string())
                );
            }
            return;
        }
        self.answers.record(change, answer);
    }
}
