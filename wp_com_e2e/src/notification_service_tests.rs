//! `NotificationService` against the live API: the account's notes synced into a cache of the
//! trial's own, in memory, and paged back to the oldest, in every tab. Every request is a GET:
//! nothing here marks a note read or seen.
//!
//! The service compares hashes, and any request for whole notes gives the notes it sends new
//! hashes. So each trial holds `AccountLocks::notification_hashes` for writing, and runs while no
//! other notification trial does. New notes can still arrive between two requests, so a trial
//! allows for notes newer than its start. No trial prints anything from a note but its type.

use libtest_mimic::{Failed, Trial};
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use wp_api::date::WpGmtDateTime;
use wp_api::prelude::WpApiClientDelegate;
use wp_api::wp_com::language::WPComLanguage;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::notifications::{
    NotificationId, NotificationTab, NotificationType, NotificationsParams, NotificationsQuery,
    WpComNotification,
};
use wp_mobile::service::notifications::{
    NotificationRefreshResult, NotificationService, NotificationSyncResult, NotificationsFilter,
};
use wp_mobile_cache::WpApiCache;

use crate::context::{CountingExecutor, TestContext, changing};
use crate::walk::{Page, Step, Walk};

/// The most older pages a trial loads before it gives up: more than any test account has.
const MAX_OLDER_PAGES: usize = 200;

/// A pause between pages, as the sweep makes.
const PAUSE: Duration = Duration::from_millis(250);

/// How many times a sync right after a sync may be tried before it must download nothing.
const AGAIN_ATTEMPTS: usize = 3;

fn service(ctx: &TestContext) -> Result<NotificationService, Failed> {
    service_with(ctx.delegate.clone())
}

fn service_with(delegate: WpApiClientDelegate) -> Result<NotificationService, Failed> {
    let cache = WpApiCache::new(None).map_err(|e| e.to_string())?;
    cache.perform_migrations().map_err(|e| e.to_string())?;
    // The cache is the trial's own, so any user ID keys it.
    let reader = wp_mobile::service::reader::ReaderService::new(
        delegate.clone(),
        Arc::new(cache),
        wp_mobile_cache::repository::wordpress_com_accounts::ReaderOwner::Account {
            user_id: WpComUserId(1),
        },
        wp_api::wp_com::reader_saved::ReaderSavedSource::Ios,
    )
    .map_err(|e| e.to_string())?;
    Ok(NotificationService::new(delegate, Arc::new(reader)).map_err(|e| e.to_string())?)
}

/// A service whose requests are counted.
fn counted_service(
    ctx: &TestContext,
) -> Result<(NotificationService, Arc<CountingExecutor>), Failed> {
    let executor = Arc::new(CountingExecutor {
        inner: Arc::clone(&ctx.delegate.request_executor),
        sent: AtomicU64::new(0),
    });
    let service = service_with(WpApiClientDelegate {
        request_executor: executor.clone(),
        ..ctx.delegate.clone()
    })?;
    Ok((service, executor))
}

fn ids(notes: &[WpComNotification]) -> HashSet<NotificationId> {
    notes.iter().map(|note| note.id).collect()
}

/// Syncs `tab` and loads its older pages until its end, and returns how many older pages it
/// took.
async fn page_to_the_end(
    service: &NotificationService,
    tab: NotificationTab,
) -> Result<usize, Failed> {
    let mut result = service.sync_tab(tab).await.map_err(|e| e.to_string())?;
    let mut pages = 0;
    while !result.reached_end {
        if pages == MAX_OLDER_PAGES {
            return Err(format!("{tab:?}: no end after {pages} older pages").into());
        }
        tokio::time::sleep(PAUSE).await;
        result = service
            .load_older_in_tab(tab)
            .await
            .map_err(|e| e.to_string())?;
        pages += 1;
    }
    Ok(pages)
}

fn cached(service: &NotificationService) -> Result<Vec<WpComNotification>, Failed> {
    Ok(service
        .load_notes(NotificationsFilter::default())
        .map_err(|e| e.to_string())?)
}

fn counts(result: &NotificationSyncResult) -> String {
    format!(
        "{} listed, {} downloaded, {} not sent whole, {} removed, end reached: {}",
        result.listed_count,
        result.downloaded_count,
        result.not_sent_count,
        result.removed_count,
        result.reached_end
    )
}

fn types(notes: &[WpComNotification]) -> BTreeMap<&str, usize> {
    let mut types = BTreeMap::new();
    for note in notes {
        let name = note
            .note_type
            .as_ref()
            .map_or("(no type)", NotificationType::as_str);
        *types.entry(name).or_default() += 1;
    }
    types
}

fn seconds(note: &WpComNotification) -> Option<i64> {
    note.timestamp.map(|time| time.0.timestamp())
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Fails if the cache has a note it couldn't read.
fn no_unreadable(service: &NotificationService) -> Result<(), Failed> {
    let info = service.info().map_err(|e| e.to_string())?;
    if info.unreadable_count > 0 {
        return Err(format!(
            "{} note(s) came and couldn't be read",
            info.unreadable_count
        )
        .into());
    }
    Ok(())
}

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    vec![
        Trial::test("notification_service::sync_fills_an_empty_cache", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _hashes = changing(&ctx.locks.notification_hashes);
                ctx.runtime.block_on(async {
                    let service = service(&ctx)?;
                    let result = service.sync().await.map_err(|e| e.to_string())?;
                    let notes = cached(&service)?;
                    if notes.len() as u64 != result.downloaded_count {
                        return Err(format!(
                            "{} notes were downloaded and the cache has {}",
                            result.downloaded_count,
                            notes.len()
                        )
                        .into());
                    }
                    if result.downloaded_count + result.not_sent_count != result.listed_count {
                        return Err(format!(
                            "Not every listed note was asked for: {}",
                            counts(&result)
                        )
                        .into());
                    }
                    let info = service.info().map_err(|e| e.to_string())?;
                    if info.last_synced_at.is_none() {
                        return Err("The sync wasn't recorded".into());
                    }
                    no_unreadable(&service)?;
                    println!("{}: {:?}", counts(&result), types(&notes));
                    Ok(())
                })
            }
        }),
        Trial::test("notification_service::a_second_sync_downloads_nothing", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _hashes = changing(&ctx.locks.notification_hashes);
                ctx.runtime.block_on(async {
                    let (service, executor) = counted_service(&ctx)?;
                    let first = service.sync().await.map_err(|e| e.to_string())?;
                    println!("First sync: {}", counts(&first));
                    // Another client of the account, such as another CI build, can fetch notes
                    // whole between two syncs, which gives them new hashes. So a sync that
                    // downloaded more than what arrived is tried again.
                    let mut failure = String::new();
                    for _ in 0..AGAIN_ATTEMPTS {
                        let before: HashSet<NotificationId> =
                            cached(&service)?.iter().map(|note| note.id).collect();
                        let sent_before = executor.sent.load(Ordering::SeqCst);
                        let again = service.sync().await.map_err(|e| e.to_string())?;
                        let sent = executor.sent.load(Ordering::SeqCst) - sent_before;
                        // A note that arrived in between is new, not changed.
                        let arrived = cached(&service)?
                            .iter()
                            .filter(|note| !before.contains(&note.id))
                            .count() as u64;
                        println!(
                            "Sync again: {}, {arrived} new, {sent} request(s)",
                            counts(&again)
                        );
                        if again.downloaded_count <= arrived && again.not_sent_count == 0 {
                            // With nothing to download, the check is the only request.
                            if again.downloaded_count == 0 && sent != 1 {
                                return Err(format!(
                                    "A sync that downloaded nothing sent {sent} requests"
                                )
                                .into());
                            }
                            return Ok(());
                        }
                        failure = format!(
                            "A sync right after a sync downloaded {} note(s) that hadn't changed, \
                             and asked again for {} the API doesn't send whole",
                            again.downloaded_count.saturating_sub(arrived),
                            again.not_sent_count
                        );
                    }
                    Err(failure.into())
                })
            }
        }),
        Trial::test("notification_service::paging_reaches_the_end", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _hashes = changing(&ctx.locks.notification_hashes);
                ctx.runtime.block_on(async {
                    let started = now();
                    let service = service(&ctx)?;
                    let mut result = service.sync().await.map_err(|e| e.to_string())?;
                    println!("Sync: {}", counts(&result));
                    let mut pages = 0;
                    while !result.reached_end {
                        if pages == MAX_OLDER_PAGES {
                            return Err(format!("No end after {pages} older pages").into());
                        }
                        tokio::time::sleep(PAUSE).await;
                        result = service.load_older().await.map_err(|e| e.to_string())?;
                        pages += 1;
                        println!("Older page {pages}: {}", counts(&result));
                    }
                    no_unreadable(&service)?;
                    let notes = cached(&service)?;
                    let info = service.info().map_err(|e| e.to_string())?;
                    println!(
                        "{} notes cached, {} listed and never sent whole, after {pages} older pages: {:?}",
                        notes.len(),
                        info.listed_only_count,
                        types(&notes)
                    );

                    let walked = walk(&ctx).await?;
                    let in_cache: HashSet<NotificationId> = notes.iter().map(|note| note.id).collect();
                    let missing = walked
                        .iter()
                        .filter(|(id, time)| {
                            !in_cache.contains(id) && time.is_none_or(|time| time < started)
                        })
                        .count();
                    let walked_ids: HashSet<NotificationId> =
                        walked.iter().map(|(id, _)| *id).collect();
                    let extra = notes
                        .iter()
                        .filter(|note| !walked_ids.contains(&note.id))
                        .count();
                    println!(
                        "A walk found {} notes whole: {missing} of them not cached, and {extra} cached notes it didn't find",
                        walked.len()
                    );
                    if missing > 0 || extra > 0 {
                        return Err(format!(
                            "The cache and a walk differ: {missing} walked notes not cached, {extra} cached notes not walked"
                        )
                        .into());
                    }
                    // Newest first, with any note that has no date last.
                    let dated: Vec<i64> = notes.iter().filter_map(seconds).collect();
                    if dated.windows(2).any(|pair| pair[0] < pair[1]) {
                        return Err("The cached notes aren't newest first".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test(
            "notification_service::each_tab_has_the_notes_calypso_shows",
            {
                let ctx = Arc::clone(&ctx);
                move || {
                    let _hashes = changing(&ctx.locks.notification_hashes);
                    ctx.runtime.block_on(async {
                    let started = now();
                    // Notes newer than the start may have arrived in one walk and not another.
                    let before_start = |note: &&WpComNotification| {
                        seconds(note).is_none_or(|time| time < started)
                    };
                    // Each tab in a cache of its own, so no other tab's downloads fill it in.
                    let mut tabs = Vec::new();
                    for tab in NotificationTab::EVERY_TAB {
                        if tab == NotificationTab::All {
                            continue;
                        }
                        let service = service(&ctx)?;
                        let pages = page_to_the_end(&service, tab).await?;
                        no_unreadable(&service)?;
                        let notes = cached(&service)?;
                        // A tab's own syncs download only notes the tab shows.
                        let foreign = notes.iter().filter(|note| !tab.includes(note)).count();
                        if foreign > 0 {
                            return Err(format!(
                                "{tab:?} downloaded {foreign} note(s) it doesn't show"
                            )
                            .into());
                        }
                        tabs.push((tab, pages, notes));
                    }
                    let all = service(&ctx)?;
                    let all_pages = page_to_the_end(&all, NotificationTab::All).await?;
                    no_unreadable(&all)?;
                    let every_note = cached(&all)?;
                    let every_id = ids(&every_note);
                    println!(
                        "All: {} notes after {all_pages} older pages: {:?}",
                        every_note.len(),
                        types(&every_note)
                    );

                    let mut failures = Vec::new();
                    for (tab, pages, notes) in tabs {
                        let expected: HashSet<NotificationId> = every_note
                            .iter()
                            .filter(before_start)
                            .filter(|note| tab.includes(note))
                            .map(|note| note.id)
                            .collect();
                        let got: HashSet<NotificationId> =
                            notes.iter().filter(before_start).map(|note| note.id).collect();
                        let missing = expected.difference(&got).count();
                        let extra = got.difference(&every_id).count();
                        println!(
                            "{tab:?}: {} notes after {pages} older pages, {missing} missing, {extra} not in All: {:?}",
                            notes.len(),
                            types(&notes)
                        );
                        if missing > 0 || extra > 0 {
                            failures.push(format!(
                                "{tab:?}: {missing} note(s) Calypso's rule picks are missing, {extra} aren't in All"
                            ));
                        }
                    }
                    if failures.is_empty() {
                        Ok(())
                    } else {
                        Err(failures.join("; ").into())
                    }
                })
                }
            },
        ),
        Trial::test(
            "notification_service::a_tab_downloads_no_note_another_tab_has",
            {
                let ctx = Arc::clone(&ctx);
                move || {
                    let _hashes = changing(&ctx.locks.notification_hashes);
                    ctx.runtime.block_on(async {
                    let service = service(&ctx)?;
                    let first = service.sync().await.map_err(|e| e.to_string())?;
                    println!("All: {}", counts(&first));
                    for tab in NotificationTab::EVERY_TAB {
                        if tab == NotificationTab::All {
                            continue;
                        }
                        // Another client of the account can fetch notes whole in between, which
                        // gives them new hashes, so a tab that downloaded more is tried again.
                        let mut failure = None;
                        for _ in 0..AGAIN_ATTEMPTS {
                            let before = ids(&cached(&service)?);
                            let result = service.sync_tab(tab).await.map_err(|e| e.to_string())?;
                            let new = ids(&cached(&service)?).difference(&before).count() as u64;
                            println!("{tab:?}: {}, {new} new to the cache", counts(&result));
                            if result.downloaded_count <= new {
                                failure = None;
                                break;
                            }
                            failure = Some(format!(
                                "{tab:?} downloaded {} note(s) the cache had with the same hash",
                                result.downloaded_count - new
                            ));
                        }
                        if let Some(failure) = failure {
                            return Err(failure.into());
                        }
                    }
                    let info = service.info().map_err(|e| e.to_string())?;
                    let stored = cached(&service)?.len() as u64;
                    if info.note_count != stored {
                        return Err(format!(
                            "{} rows for {stored} notes: a note is stored more than once",
                            info.note_count
                        )
                        .into());
                    }
                    Ok(())
                })
                }
            },
        ),
        Trial::test("notification_service::refresh_and_remove_a_note", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _hashes = changing(&ctx.locks.notification_hashes);
                ctx.runtime.block_on(async {
                    let (service, executor) = counted_service(&ctx)?;
                    service.sync().await.map_err(|e| e.to_string())?;
                    let Some(newest) = cached(&service)?.first().map(|note| note.id) else {
                        println!("The account has no note the API sends whole: nothing to refresh");
                        return Ok(());
                    };
                    let count = || -> Result<u64, Failed> {
                        Ok(service.info().map_err(|e| e.to_string())?.note_count)
                    };
                    let notes = count()?;

                    let sent = executor.sent.load(Ordering::SeqCst);
                    let refreshed = service
                        .refresh_note(newest)
                        .await
                        .map_err(|e| e.to_string())?;
                    let sent = executor.sent.load(Ordering::SeqCst) - sent;
                    println!("Refreshing the newest note: {refreshed:?}, {sent} request(s)");
                    if refreshed != NotificationRefreshResult::Stored
                        || sent != 1
                        || count()? != notes
                    {
                        return Err(format!(
                            "Refreshing a cached note gave {refreshed:?} with {sent} request(s)"
                        )
                        .into());
                    }

                    // No note has an ID this small.
                    let missing = service
                        .refresh_note(NotificationId(1))
                        .await
                        .map_err(|e| e.to_string())?;
                    println!("Refreshing a note that doesn't exist: {missing:?}");
                    if missing != NotificationRefreshResult::NotSent || count()? != notes {
                        return Err(format!(
                            "Refreshing a note that doesn't exist gave {missing:?}"
                        )
                        .into());
                    }

                    let sent = executor.sent.load(Ordering::SeqCst);
                    let removed = service
                        .remove_note(newest)
                        .await
                        .map_err(|e| e.to_string())?;
                    let sent = executor.sent.load(Ordering::SeqCst) - sent;
                    if !removed || sent != 0 || ids(&cached(&service)?).contains(&newest) {
                        return Err(format!(
                            "Removing a cached note: removed {removed}, {sent} request(s)"
                        )
                        .into());
                    }
                    // The server still has it, so the next sync brings it back.
                    let again = service.sync().await.map_err(|e| e.to_string())?;
                    println!(
                        "Removing it sent nothing; the next sync: {}",
                        counts(&again)
                    );
                    if !ids(&cached(&service)?).contains(&newest) {
                        return Err("A sync after removing a note didn't bring it back".into());
                    }
                    Ok(())
                })
            }
        }),
    ]
}

/// Walks the account's whole notes from newest to oldest, as `notifications_sweep` does, and
/// returns each note's ID and time in seconds.
async fn walk(ctx: &TestContext) -> Result<Vec<(NotificationId, Option<i64>)>, Failed> {
    let mut walk = Walk::new(100);
    let mut found = Vec::new();
    let mut seen = HashSet::new();
    for _ in 0..MAX_OLDER_PAGES {
        let query = NotificationsQuery {
            number: Some(walk.number),
            before: walk
                .before
                .and_then(|before| chrono::DateTime::from_timestamp(before, 0))
                .map(WpGmtDateTime),
            // The service's language, so this doesn't change how the hashes are made either.
            locale: Some(WPComLanguage::English),
            ..Default::default()
        };
        let whole = ctx
            .client
            .notifications()
            .list(&NotificationsParams {
                query: query.clone(),
                include_body: true,
                include_context: true,
            })
            .await
            .map_err(|e| e.to_string())?
            .data;
        let listed = ctx
            .client
            .notifications()
            .list_ids(&query)
            .await
            .map_err(|e| e.to_string())?
            .data;
        let notes: Vec<(NotificationId, Option<i64>)> = whole
            .notes
            .iter()
            .map(|note| (note.id, seconds(note)))
            .collect();
        for note in &notes {
            if seen.insert(note.0) {
                found.push(*note);
            }
        }
        let unreadable: Vec<NotificationId> = whole
            .unreadable_notes
            .iter()
            .filter_map(|note| note.id)
            .collect();
        let (step, _) = walk.take(Page {
            notes: &notes,
            unreadable: &unreadable,
            listed: &listed.ids,
        });
        match step {
            Step::Continue => tokio::time::sleep(PAUSE).await,
            Step::End => return Ok(found),
            Step::Stuck => return Err("The walk couldn't move on".into()),
        }
    }
    Err("The walk found no end".into())
}
