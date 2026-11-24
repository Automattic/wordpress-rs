//! `NotificationService`'s changes against the live API: marking notes read, unread and seen.
//!
//! **Every trial here changes the account of `WP_COM_API_KEY`, and each is ignored**, so
//! `cargo run -p wp_com_e2e` never runs one, with or without a filter. Read a trial's doc comment
//! first: it says what it changes and how it puts it back. Then run it alone, by its exact name:
//!
//! ```bash
//! WP_COM_API_KEY=… cargo run -q -p wp_com_e2e --bin wp_com_e2e -- \
//!     --ignored --exact notification_writes::read_and_unread_survive_a_sync
//! ```
//!
//! Each trial uses caches of its own, in memory, and prints counts and type names only.

use libtest_mimic::{Failed, Trial};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use wp_api::date::WpGmtDateTime;
use wp_api::prelude::*;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::language::WPComLanguage;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::notifications::{
    NotificationId, NotificationReadChange, NotificationTab, NotificationType, NotificationsQuery,
    NotificationsReadParams, WpComNotification,
};
use wp_mobile::service::notifications::{
    NotificationChangeOutcome, NotificationService, NotificationsFilter,
};
use wp_mobile_cache::WpApiCache;

use crate::context::{TestContext, changing, making_a_change, read_back};
use crate::reader_service_tests::{
    account, first_post_of, like_and_check, new_service, server_says_post_liked,
};

/// The receiving account's token, for the trial of a deleted note. Only GET requests are sent
/// with it.
const RECEIVER_TOKEN: &str = "WP_COM_E2E_NOTE_RECEIVER_API_KEY";
/// A site the receiving account owns, whose newest post the account of `WP_COM_API_KEY` likes.
const RECEIVER_SITE: &str = "WP_COM_E2E_NOTE_RECEIVER_SITE_ID";

/// The most notes the mark-all trial may change.
const MARK_ALL_LIMIT: usize = 3;

/// The most older pages of the Unread tab the mark-all trial loads: more than a list of 200
/// unread notes needs.
const UNREAD_PAGES: usize = 5;

/// How long a note may take to arrive or go, and how often the receiver syncs meanwhile.
const NOTE_WAIT: Duration = Duration::from_secs(60);
const NOTE_POLL: Duration = Duration::from_secs(5);

/// A note before this many seconds since 1970 has no date.
const UNDATED_BEFORE: i64 = 86_400;

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    vec![
        // Changes one note's read state and puts it back: see `read_and_unread_survive_a_sync`.
        Trial::test("notification_writes::read_and_unread_survive_a_sync", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _changes = making_a_change(&ctx.locks);
                let _hashes = changing(&ctx.locks.notification_hashes);
                ctx.runtime.block_on(read_and_unread_survive_a_sync(&ctx))
            }
        })
        .with_ignored_flag(true),
        // Moves the account's last seen time forward, which can't be undone: see
        // `marking_seen_clears_the_unseen_flag`.
        Trial::test(
            "notification_writes::marking_seen_clears_the_unseen_flag",
            {
                let ctx = Arc::clone(&ctx);
                move || {
                    let _changes = making_a_change(&ctx.locks);
                    let _hashes = changing(&ctx.locks.notification_hashes);
                    ctx.runtime
                        .block_on(marking_seen_clears_the_unseen_flag(&ctx))
                }
            },
        )
        .with_ignored_flag(true),
        // Marks at most three of the oldest unread notes read and puts them back: see
        // `mark_all_read_up_to_a_time`.
        Trial::test("notification_writes::mark_all_read_up_to_a_time", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _changes = making_a_change(&ctx.locks);
                let _hashes = changing(&ctx.locks.notification_hashes);
                ctx.runtime.block_on(mark_all_read_up_to_a_time(&ctx))
            }
        })
        .with_ignored_flag(true),
        // Likes and unlikes a post of another account's site: see
        // `a_deleted_note_leaves_the_cache`.
        Trial::test("notification_writes::a_deleted_note_leaves_the_cache", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _changes = making_a_change(&ctx.locks);
                let _hashes = changing(&ctx.locks.notification_hashes);
                ctx.runtime.block_on(a_deleted_note_leaves_the_cache(&ctx))
            }
        })
        .with_ignored_flag(true),
    ]
}

fn service_with(delegate: WpApiClientDelegate) -> Result<NotificationService, Failed> {
    let cache = WpApiCache::new(None).map_err(|e| e.to_string())?;
    cache.perform_migrations().map_err(|e| e.to_string())?;
    // The cache is the trial's own, so any user ID keys it.
    Ok(
        NotificationService::new(delegate, Arc::new(cache), WpComUserId(1))
            .map_err(|e| e.to_string())?,
    )
}

fn service(ctx: &TestContext) -> Result<NotificationService, Failed> {
    service_with(ctx.delegate.clone())
}

fn type_name(note: &WpComNotification) -> &str {
    note.note_type
        .as_ref()
        .map_or("(no type)", NotificationType::as_str)
}

fn seconds(note: &WpComNotification) -> Option<i64> {
    note.timestamp.map(|time| time.0.timestamp())
}

fn cached(
    service: &NotificationService,
    note_id: NotificationId,
) -> Result<Option<WpComNotification>, Failed> {
    Ok(service
        .load_notes(NotificationsFilter::default())
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|note| note.id == note_id))
}

fn in_unread_tab(service: &NotificationService, note_id: NotificationId) -> Result<bool, Failed> {
    Ok(service
        .load_tab_notes(NotificationTab::Unread)
        .map_err(|e| e.to_string())?
        .iter()
        .any(|note| note.id == note_id))
}

/// The account's unread notes, as the server lists their IDs, newest first, in the language the
/// service sends. `None` when there are more than one list holds.
async fn unread_ids(ctx: &TestContext) -> Result<Option<Vec<NotificationId>>, Failed> {
    let ids = ctx
        .client
        .notifications()
        .list_ids(&NotificationsQuery {
            number: Some(200),
            unread: true,
            locale: Some(WPComLanguage::English),
            ..Default::default()
        })
        .await
        .map_err(|e| e.to_string())?
        .data
        .ids;
    Ok((ids.len() < 200).then_some(ids))
}

/// Marks notes read or unread with the endpoint itself, for putting things back when the service
/// couldn't.
async fn mark_directly(
    ctx: &TestContext,
    note_ids: &[NotificationId],
    read: bool,
) -> Result<(), Failed> {
    let answer = ctx
        .client
        .notifications()
        .mark_read(&NotificationsReadParams {
            changes: note_ids
                .iter()
                .map(|note_id| NotificationReadChange {
                    note_id: *note_id,
                    read,
                })
                .collect(),
            source: Some("mobile".to_string()),
        })
        .await
        .map_err(|e| format!("Putting {} note(s) back was refused: {e}", note_ids.len()))?
        .data;
    if !answer.success {
        return Err(format!("Putting {} note(s) back wasn't done", note_ids.len()).into());
    }
    Ok(())
}

/// **Changes one note of the account of `WP_COM_API_KEY`**: the newest note that says whether
/// it was read. It's marked the other way with the service, then back. Whatever happens in
/// between, also when a check fails, the note is marked back to what it was, first with the
/// service and, if that fails, with the endpoint itself.
///
/// It checks, after each change, that:
/// - the service's cache shows the change at once, and its Unread tab has the note only while
///   it's unread;
/// - a second cache, which synced the note before the change, as another device would, syncs
///   the server's new state: the note leaves its Unread tab once read, and comes back once
///   unread.
///
/// Set `WP_COM_API_KEY`. An account with no note passes and says so.
async fn read_and_unread_survive_a_sync(ctx: &TestContext) -> Result<(), Failed> {
    let here = service(ctx)?;
    let elsewhere = service(ctx)?;
    here.sync().await.map_err(|e| e.to_string())?;
    let notes = here
        .load_notes(NotificationsFilter::default())
        .map_err(|e| e.to_string())?;
    let Some(note) = notes.iter().find(|note| note.read.is_some()) else {
        println!("No note says whether it was read: nothing to do");
        return Ok(());
    };
    let (note_id, was_read) = (note.id, note.read == Some(true));
    println!(
        "One {} note, read: {was_read}; {} notes cached",
        type_name(note),
        notes.len()
    );
    elsewhere.sync().await.map_err(|e| e.to_string())?;
    elsewhere
        .sync_tab(NotificationTab::Unread)
        .await
        .map_err(|e| e.to_string())?;

    let flipped = mark_and_check(&here, &elsewhere, note_id, !was_read).await;
    // Whatever happened, the note is put back.
    let restored = mark_and_check(&here, &elsewhere, note_id, was_read).await;
    if restored.is_err() {
        mark_directly(ctx, &[note_id], was_read).await?;
        println!("The service didn't put the note back; the endpoint did");
    }
    flipped?;
    restored
}

/// Marks the note through `here`, and checks both caches show `read`.
async fn mark_and_check(
    here: &NotificationService,
    elsewhere: &NotificationService,
    note_id: NotificationId,
    read: bool,
) -> Result<(), Failed> {
    let outcome = if read {
        here.mark_read(vec![note_id]).await
    } else {
        here.mark_unread(vec![note_id]).await
    }
    .map_err(|e| format!("Marking the note read: {read} was refused: {e}"))?;
    if !matches!(
        outcome,
        NotificationChangeOutcome::Sent | NotificationChangeOutcome::Unchanged
    ) {
        return Err(format!("Marking the note read: {read} wasn't sent: {outcome:?}").into());
    }
    let shown = cached(here, note_id)?.and_then(|note| note.read);
    if shown != Some(read) {
        return Err(format!("This cache shows read: {shown:?}, not {read}").into());
    }
    if in_unread_tab(here, note_id)? == read {
        return Err(format!("This cache's Unread tab is wrong for read: {read}").into());
    }

    // The other cache learns of it by syncing, as another device does.
    let (synced, in_tab) = read_back(
        async || {
            elsewhere.sync().await.map_err(|e| e.to_string())?;
            elsewhere
                .sync_tab(NotificationTab::Unread)
                .await
                .map_err(|e| e.to_string())?;
            Ok((
                cached(elsewhere, note_id)?.and_then(|note| note.read),
                in_unread_tab(elsewhere, note_id)?,
            ))
        },
        |(synced, in_tab)| *synced == Some(read) && *in_tab != read,
    )
    .await?;
    if synced != Some(read) {
        return Err(format!("After a sync, the server says read: {synced:?}, not {read}").into());
    }
    if in_tab == read {
        return Err(format!("After a sync, the other Unread tab is wrong for read: {read}").into());
    }
    println!("Marked read: {read}; both caches agree after a sync");
    Ok(())
}

/// **Moves the last seen time of the account of `WP_COM_API_KEY` forward, to its newest note.
/// This can't be undone**: no request moves the time back. It clears the account's unseen flag,
/// which lights the notifications bell in the apps and on the web.
///
/// It sends the newest note's time only when that's newer than the account's current last seen
/// time, which a sync just read, so it never sends an older one. Then it checks that the
/// server's flag is clear, and that a second cache syncs the new time.
///
/// When nothing is unseen, the flag has nothing to clear. The trial then sends the current
/// time, which is newer than every note, and checks that the server takes it and that a second
/// cache syncs it.
///
/// Set `WP_COM_API_KEY`. An account with no dated note sends nothing and passes.
async fn marking_seen_clears_the_unseen_flag(ctx: &TestContext) -> Result<(), Failed> {
    let service = service(ctx)?;
    service.sync().await.map_err(|e| e.to_string())?;
    let info = service.info().map_err(|e| e.to_string())?;
    let flag = service
        .fetch_has_unseen_notes()
        .await
        .map_err(|e| e.to_string())?;
    let newest = service
        .load_notes(NotificationsFilter::default())
        .map_err(|e| e.to_string())?
        .iter()
        .filter_map(seconds)
        .filter(|time| *time >= UNDATED_BEFORE)
        .max();
    let seen = info.last_seen_time.map(|time| time.0.timestamp());
    println!(
        "Unseen flag: {flag}; newest note is newer than the last seen time: {:?}",
        newest.map(|newest| seen.is_none_or(|seen| newest > seen))
    );
    let Some(newest) = newest else {
        println!("No dated note: nothing to do");
        return Ok(());
    };
    if seen.is_some_and(|seen| newest <= seen) {
        let outcome = service.mark_seen(None).await.map_err(|e| e.to_string())?;
        if outcome != NotificationChangeOutcome::Unchanged {
            return Err(format!("A time that isn't newer was sent: {outcome:?}").into());
        }
        // The flag is already clear, so only the request itself is left to show.
        let now = chrono::Utc::now().timestamp();
        let outcome = service
            .mark_seen(chrono::DateTime::from_timestamp(now, 0).map(WpGmtDateTime))
            .await
            .map_err(|e| format!("Marking seen was refused: {e}"))?;
        if outcome != NotificationChangeOutcome::Sent {
            return Err(format!("Marking seen wasn't sent: {outcome:?}").into());
        }
        let other = self::service(ctx)?;
        other.sync().await.map_err(|e| e.to_string())?;
        let synced = other
            .info()
            .map_err(|e| e.to_string())?
            .last_seen_time
            .map(|time| time.0.timestamp());
        if synced.is_none_or(|synced| synced < now) {
            return Err("A second cache doesn't sync the new last seen time".into());
        }
        println!(
            "Nothing was unseen, so the flag had nothing to clear; the server took the current \
             time, and a second cache has it"
        );
        return Ok(());
    }

    let outcome = service
        .mark_seen(None)
        .await
        .map_err(|e| format!("Marking seen was refused: {e}"))?;
    if outcome != NotificationChangeOutcome::Sent {
        return Err(format!("Marking seen wasn't sent: {outcome:?}").into());
    }
    if service.has_unseen_notes().map_err(|e| e.to_string())? {
        return Err("The cache still says there are unseen notes".into());
    }
    let flag = read_back(
        async || {
            service
                .fetch_has_unseen_notes()
                .await
                .map_err(|e| e.to_string().into())
        },
        |flag| !*flag,
    )
    .await?;
    if flag {
        return Err("The server's unseen flag is still set".into());
    }
    let other = self::service(ctx)?;
    other.sync().await.map_err(|e| e.to_string())?;
    let synced = other
        .info()
        .map_err(|e| e.to_string())?
        .last_seen_time
        .map(|time| time.0.timestamp());
    if synced.is_none_or(|synced| synced < newest) {
        return Err("A second cache doesn't sync the new last seen time".into());
    }
    println!("The unseen flag is clear, and a second cache has the new time");
    Ok(())
}

/// **Marks a few notes of the account of `WP_COM_API_KEY` read, then unread again.** They're the
/// oldest unread notes, chosen by a time: every unread note at or before it, which the trial
/// makes sure is three at most. Marking every note read takes a time and nothing else, so the
/// trial first lists every unread note and picks a time with no other unread note at or before
/// it. An unread note whose time it can't know, because the API doesn't send it whole or it has
/// no date, may be marked too: there can be three of those at most. Each note the server
/// changed is then marked unread, by the service and, if that fails, by the endpoint itself,
/// whatever happens in between. A note the server marked that the trial didn't allow for is put
/// back too, and fails the trial.
///
/// It sends nothing, and passes, when the account has no unread note, more unread notes than a
/// list of 200 holds, more than three unread notes whose time it can't know, or no time that
/// picks three notes or fewer.
///
/// Set `WP_COM_API_KEY`.
async fn mark_all_read_up_to_a_time(ctx: &TestContext) -> Result<(), Failed> {
    let Some(before) = unread_ids(ctx).await? else {
        println!("More unread notes than a list of 200 holds: nothing to do");
        return Ok(());
    };
    if before.is_empty() {
        println!("No unread note: nothing to do");
        return Ok(());
    }
    // Every unread note, whole, for its time: the Unread tab, paged to its end.
    let service = service(ctx)?;
    let mut page = service
        .sync_tab(NotificationTab::Unread)
        .await
        .map_err(|e| e.to_string())?;
    for _ in 0..UNREAD_PAGES {
        if page.reached_end {
            break;
        }
        page = service
            .load_older_in_tab(NotificationTab::Unread)
            .await
            .map_err(|e| e.to_string())?;
    }
    if !page.reached_end {
        println!("The Unread tab has no end within {UNREAD_PAGES} pages: nothing to do");
        return Ok(());
    }
    let times: HashMap<NotificationId, Option<i64>> = service
        .load_tab_notes(NotificationTab::Unread)
        .map_err(|e| e.to_string())?
        .iter()
        .map(|note| (note.id, seconds(note)))
        .collect();
    let mut dated: Vec<(i64, NotificationId)> = Vec::new();
    // Unread notes whose time isn't known: any of them may be at or before the time.
    let mut timeless: HashSet<NotificationId> = HashSet::new();
    for note_id in &before {
        match times.get(note_id).copied().flatten() {
            Some(time) if time >= UNDATED_BEFORE => dated.push((time, *note_id)),
            _ => {
                timeless.insert(*note_id);
            }
        }
    }
    if timeless.len() > MARK_ALL_LIMIT {
        println!(
            "{} unread note(s); {} can't be fetched whole or have no date: nothing to do",
            before.len(),
            timeless.len()
        );
        return Ok(());
    }
    dated.sort_by_key(|(time, note_id)| (*time, note_id.0));
    // The most notes, up to the limit, whose time no other unread note shares.
    let Some(count) = (1..=MARK_ALL_LIMIT.min(dated.len())).rev().find(|count| {
        dated
            .get(*count)
            .is_none_or(|next| next.0 > dated[count - 1].0)
    }) else {
        println!("No time picks {MARK_ALL_LIMIT} unread notes or fewer: nothing to do");
        return Ok(());
    };
    // Without a time, every note would be marked.
    let up_to = chrono::DateTime::from_timestamp(dated[count - 1].0, 0)
        .map(WpGmtDateTime)
        .ok_or("The time that picks the notes isn't a date")?;
    let expected: HashSet<NotificationId> = dated[..count].iter().map(|(_, id)| *id).collect();
    println!(
        "{} unread note(s); marking every note up to a time, which picks {count}, and may pick \
         {} more whose time isn't known",
        before.len(),
        timeless.len()
    );

    let marked = service
        .mark_all_read(NotificationTab::All, Some(up_to))
        .await;
    // Whatever the answer, find what the server changed, and put it back.
    let after = read_back(
        async || {
            Ok(unread_ids(ctx)
                .await?
                .ok_or("More unread notes than a list of 200 holds")?)
        },
        |after| expected.iter().all(|id| !after.contains(id)),
    )
    .await;
    let changed: Vec<NotificationId> = match &after {
        Ok(after) => before
            .iter()
            .filter(|id| !after.contains(id))
            .copied()
            .collect(),
        // Unknown: put back every note it may have changed.
        Err(_) => before.clone(),
    };
    let put_back = put_back_unread(ctx, &service, &changed).await;

    let outcome = marked.map_err(|e| format!("Marking every note read was refused: {e}"))?;
    if outcome != NotificationChangeOutcome::Sent {
        return Err(format!("Marking every note read wasn't sent: {outcome:?}").into());
    }
    after?;
    let changed_set: HashSet<NotificationId> = changed.iter().copied().collect();
    put_back?;
    if !changed_set.is_superset(&expected) {
        return Err(format!(
            "{} of the {count} notes at or before the time stayed unread",
            expected.difference(&changed_set).count()
        )
        .into());
    }
    let unexpected = changed_set
        .iter()
        .filter(|id| !expected.contains(id) && !timeless.contains(id))
        .count();
    if unexpected > 0 {
        return Err(format!(
            "{unexpected} note(s) after the time were marked read too; all were put back"
        )
        .into());
    }
    println!(
        "{count} note(s) were marked read and put back, with {} whose time isn't known",
        changed_set.len() - count
    );
    Ok(())
}

/// Marks `note_ids` unread with the service, or with the endpoint itself if that fails, and
/// checks the server lists them as unread.
async fn put_back_unread(
    ctx: &TestContext,
    service: &NotificationService,
    note_ids: &[NotificationId],
) -> Result<(), Failed> {
    if note_ids.is_empty() {
        return Ok(());
    }
    let by_service = service.mark_unread(note_ids.to_vec()).await;
    if !matches!(by_service, Ok(NotificationChangeOutcome::Sent)) {
        println!("The service didn't put the notes back; the endpoint does");
        for chunk in note_ids.chunks(100) {
            mark_directly(ctx, chunk, false).await?;
        }
    }
    let unread_now = async || {
        read_back(
            async || {
                Ok(unread_ids(ctx)
                    .await?
                    .ok_or("More unread notes than a list of 200 holds")?)
            },
            |after| note_ids.iter().all(|id| after.contains(id)),
        )
        .await
    };
    let mut after = unread_now().await?;
    // The service sends only the notes its cache shows as read. The endpoint marks any other.
    let still_read: Vec<NotificationId> = note_ids
        .iter()
        .filter(|id| !after.contains(id))
        .copied()
        .collect();
    if !still_read.is_empty() {
        println!(
            "{} note(s) the service didn't put back; the endpoint does",
            still_read.len()
        );
        for chunk in still_read.chunks(100) {
            mark_directly(ctx, chunk, false).await?;
        }
        after = unread_now().await?;
    }
    let missing = note_ids.iter().filter(|id| !after.contains(id)).count();
    if missing > 0 {
        return Err(format!("{missing} note(s) aren't unread again").into());
    }
    Ok(())
}

/// **Likes the newest post of a site, then unlikes it, from the account of `WP_COM_API_KEY`.**
/// The like notifies the site's owner, the receiving account, with a `like` note; the unlike
/// deletes it. The unlike is sent whatever happens after the like. Nothing is changed on the
/// receiving account: its token is only used for GET requests.
///
/// The receiving account's cache syncs before the like, then until the note arrives (a minute
/// at most, every five seconds), then, after the unlike, until a sync removes it.
///
/// Set `WP_COM_API_KEY`, `WP_COM_E2E_NOTE_RECEIVER_API_KEY` to the receiving account's token,
/// and `WP_COM_E2E_NOTE_RECEIVER_SITE_ID` to a site it owns. Without either of the last two, or
/// when the account already likes the post, or the receiver already has a like note for it
/// (an unlike wouldn't delete a note that other likes share), it sends nothing and passes.
async fn a_deleted_note_leaves_the_cache(ctx: &TestContext) -> Result<(), Failed> {
    let (Ok(token), Some(site_id)) = (
        std::env::var(RECEIVER_TOKEN),
        std::env::var(RECEIVER_SITE)
            .ok()
            .and_then(|site| site.trim().parse().ok())
            .map(WpComSiteId),
    ) else {
        println!("Set {RECEIVER_TOKEN} and {RECEIVER_SITE}: nothing to do");
        return Ok(());
    };
    let receiver = service_with(WpApiClientDelegate {
        auth_provider: WpAuthenticationProvider::static_with_auth(WpAuthentication::Bearer {
            token,
        })
        .into(),
        ..ctx.delegate.clone()
    })?;
    let reader = new_service(ctx.delegate.clone(), account())?;
    let post = first_post_of(&reader, site_id).await?;
    let post_id = post.identity.id as i64;
    if server_says_post_liked(ctx, &post).await? {
        println!("The account already likes the post: nothing to do");
        return Ok(());
    }
    receiver.sync().await.map_err(|e| e.to_string())?;
    let like_notes = |receiver: &NotificationService| -> Result<Vec<NotificationId>, Failed> {
        Ok(receiver
            .load_notes(NotificationsFilter {
                types: vec![NotificationType::Like],
                ..Default::default()
            })
            .map_err(|e| e.to_string())?
            .iter()
            .filter(|note| {
                note.meta
                    .as_ref()
                    .and_then(|meta| meta.ids.as_ref())
                    .and_then(|ids| ids.post)
                    .is_some_and(|post| post.0 == post_id)
            })
            .map(|note| note.id)
            .collect())
    };
    if !like_notes(&receiver)?.is_empty() {
        println!("The receiver already has a like note for the post: nothing to do");
        return Ok(());
    }

    let liked = like_and_check(ctx, &reader, &post, true).await;
    let arrived = match &liked {
        Ok(()) => {
            wait_for(&receiver, |receiver| {
                Ok(like_notes(receiver)?.first().copied())
            })
            .await
        }
        Err(_) => Ok(None),
    };
    // Whatever happened, the like is taken back.
    let unliked = like_and_check(ctx, &reader, &post, false).await;
    liked?;
    unliked?;
    let Some(note_id) = arrived? else {
        return Err("No like note arrived within a minute".into());
    };
    println!("The like note arrived; the post is unliked");

    let gone = wait_for(&receiver, |receiver| {
        Ok((!like_notes(receiver)?.contains(&note_id)).then_some(()))
    })
    .await?;
    if gone.is_none() {
        return Err("A sync didn't remove the deleted like note within a minute".into());
    }
    println!("A sync removed the deleted like note");
    Ok(())
}

/// Syncs `receiver` every [`NOTE_POLL`] until `found` finds something, for [`NOTE_WAIT`] at
/// most.
async fn wait_for<T>(
    receiver: &NotificationService,
    found: impl Fn(&NotificationService) -> Result<Option<T>, Failed>,
) -> Result<Option<T>, Failed> {
    let start = std::time::Instant::now();
    loop {
        receiver.sync().await.map_err(|e| e.to_string())?;
        if let Some(found) = found(receiver)? {
            return Ok(Some(found));
        }
        if start.elapsed() >= NOTE_WAIT {
            return Ok(None);
        }
        tokio::time::sleep(NOTE_POLL).await;
    }
}
