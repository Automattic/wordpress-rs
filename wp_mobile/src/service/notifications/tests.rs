//! Every note, ID, hash and time here is invented.

use super::*;
use crate::testing::scripted::{ScriptedExecutor, Step, delegate};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Mutex;
use wp_api::prelude::RequestMethod;
use wp_api::wp_com::language::WpComLanguageProvider;
use wp_api::wp_com::notifications::NotificationType;

const USER: WpComUserId = WpComUserId(1);

/// A time well past 1970, so `T + n` is a dated second.
const T: i64 = 1_800_000_000;

fn id(n: u64) -> u64 {
    10_000_000_000 + n
}

fn cache() -> Arc<WpApiCache> {
    let cache = WpApiCache::new(None).expect("an in-memory cache opens");
    cache.perform_migrations().expect("migrations run");
    Arc::new(cache)
}

fn service_in(
    cache: Arc<WpApiCache>,
    user_id: WpComUserId,
    page_size: u32,
) -> (NotificationService, Arc<ScriptedExecutor>) {
    let executor = Arc::new(ScriptedExecutor::default());
    let service = NotificationService::new(delegate(executor.clone()), cache, user_id)
        .unwrap()
        .with_page_size(page_size);
    (service, executor)
}

/// A service that checks and pages three notes at a time.
fn service() -> (NotificationService, Arc<ScriptedExecutor>) {
    service_in(cache(), USER, 3)
}

fn rfc3339(time: i64) -> String {
    chrono::DateTime::from_timestamp(time, 0)
        .unwrap()
        .to_rfc3339()
}

/// Note `n` at `time`, as a whole note comes, with a field the library doesn't model.
fn note_json(n: u64, time: i64, hash: u64) -> Value {
    json!({
        "id": id(n),
        "type": "like",
        "read": 0,
        "timestamp": rfc3339(time),
        "note_hash": hash,
        "subject": [{ "text": format!("Note {n}") }],
        "a_later_field": "kept",
    })
}

/// A list of IDs and hashes.
fn checked(notes: &[(u64, u64)]) -> Value {
    let notes: Vec<Value> = notes
        .iter()
        .map(|(n, hash)| json!({ "id": id(*n), "note_hash": hash }))
        .collect();
    json!({ "last_seen_time": T.to_string(), "notes": notes })
}

/// A page of whole notes.
fn whole(notes: &[(u64, i64, u64)]) -> Value {
    let notes: Vec<Value> = notes
        .iter()
        .map(|(n, time, hash)| note_json(*n, *time, *hash))
        .collect();
    json!({ "last_seen_time": T.to_string(), "notes": notes })
}

/// The cached notes, newest first, by `n`.
fn shown(service: &NotificationService) -> Vec<u64> {
    service
        .load_notes(NotificationsFilter::default())
        .unwrap()
        .iter()
        .map(|note| note.id.0 - id(0))
        .collect()
}

/// Each request's query parameters, decoded.
fn params(executor: &ScriptedExecutor) -> Vec<BTreeMap<String, String>> {
    executor
        .sent()
        .iter()
        .map(|sent| {
            url::Url::parse(&sent.url)
                .unwrap()
                .query_pairs()
                .into_owned()
                .collect()
        })
        .collect()
}

/// Each request's `ids`, as `n`s, or empty for a list of hashes.
fn downloads(executor: &ScriptedExecutor) -> Vec<Vec<u64>> {
    params(executor)
        .iter()
        .filter_map(|params| params.get("ids"))
        .map(|ids| {
            ids.split(',')
                .map(|n| n.parse::<u64>().unwrap() - id(0))
                .collect()
        })
        .collect()
}

/// Syncs once with `check` as the list and `download` as the whole notes, if any are asked for.
async fn sync_with(
    service: &NotificationService,
    executor: &ScriptedExecutor,
    check: Value,
    download: Option<Value>,
) -> NotificationSyncResult {
    executor.push(Step::Respond(200, check));
    if let Some(download) = download {
        executor.push(Step::Respond(200, download));
    }
    service.sync().await.unwrap()
}

async fn older_with(
    service: &NotificationService,
    executor: &ScriptedExecutor,
    check: Value,
    download: Option<Value>,
) -> Result<NotificationSyncResult, NotificationServiceError> {
    executor.push(Step::Respond(200, check));
    if let Some(download) = download {
        executor.push(Step::Respond(200, download));
    }
    service.load_older().await
}

#[tokio::test]
async fn a_first_sync_downloads_the_newest_notes_whole() {
    let (service, executor) = service();
    let result = sync_with(
        &service,
        &executor,
        checked(&[(3, 31), (2, 21), (1, 11)]),
        Some(whole(&[(3, T + 3, 32), (2, T + 2, 22), (1, T + 1, 12)])),
    )
    .await;

    assert_eq!(
        result,
        NotificationSyncResult {
            listed_count: 3,
            downloaded_count: 3,
            not_sent_count: 0,
            removed_count: 0,
            reached_end: false,
        }
    );
    assert_eq!(shown(&service), [3, 2, 1]);
    let sent = params(&executor);
    assert_eq!(sent[0]["fields"], "id,note_hash");
    assert_eq!(sent[0]["number"], "3");
    assert!(!sent[0].contains_key("before"));
    assert_eq!(sent[1]["ids"], format!("{},{},{}", id(3), id(2), id(1)));
    assert_eq!(sent[1]["number"], "3");
    assert!(sent[1]["fields"].ends_with(",header,body,note_context"));
    assert!(
        executor
            .sent()
            .iter()
            .all(|sent| sent.method == RequestMethod::GET)
    );
    let info = service.info().unwrap();
    assert_eq!(info.note_count, 3);
    assert!(info.last_synced_at.is_some());
    assert_eq!(info.last_seen_time, date(T));
}

#[tokio::test]
async fn a_cached_note_is_read_from_the_json_it_came_as() {
    let (service, executor) = service();
    sync_with(
        &service,
        &executor,
        checked(&[(1, 11)]),
        Some(whole(&[(1, T + 1, 12)])),
    )
    .await;
    let notes = service.load_notes(NotificationsFilter::default()).unwrap();
    let expected: WpComNotification = serde_json::from_value(note_json(1, T + 1, 12)).unwrap();
    assert_eq!(notes, [expected]);
}

/// Point 1 and 2: the check's hash differs from the one the whole note came with, and the next
/// check gives the whole note's.
#[tokio::test]
async fn the_hash_kept_is_the_one_the_whole_note_came_with() {
    let (service, executor) = service();
    sync_with(
        &service,
        &executor,
        checked(&[(1, 11)]),
        Some(whole(&[(1, T + 1, 12)])),
    )
    .await;

    // Nothing changed: the check gives the hash the fetch made, and nothing is downloaded.
    let result = sync_with(&service, &executor, checked(&[(1, 12)]), None).await;
    assert_eq!(result.downloaded_count, 0);
    assert_eq!(executor.sent().len(), 3);

    // Another client fetched it, or it changed: it's downloaded again, and its new hash kept.
    let result = sync_with(
        &service,
        &executor,
        checked(&[(1, 13)]),
        Some(whole(&[(1, T + 1, 14)])),
    )
    .await;
    assert_eq!(result.downloaded_count, 1);
    let result = sync_with(&service, &executor, checked(&[(1, 14)]), None).await;
    assert_eq!(result.downloaded_count, 0);
    assert_eq!(downloads(&executor), [vec![1], vec![1]]);
}

/// Point 3: one fixed form of one language on every request, checks included, whatever the
/// delegate's language provider says.
#[tokio::test]
async fn every_request_sends_one_fixed_locale() {
    #[derive(Debug)]
    struct German;
    impl WpComLanguageProvider for German {
        fn current_language(&self) -> Option<WPComLanguage> {
            Some(WPComLanguage::German)
        }
    }
    let executor = Arc::new(ScriptedExecutor::default());
    let service = NotificationService::new(
        WpApiClientDelegate {
            language_provider: Some(Arc::new(German)),
            ..delegate(executor.clone())
        },
        cache(),
        USER,
    )
    .unwrap()
    .with_page_size(3);

    sync_with(
        &service,
        &executor,
        checked(&[(1, 11)]),
        Some(whole(&[(1, T + 1, 12)])),
    )
    .await;
    service.set_language(Some(WPComLanguage::PortugueseBrazil));
    sync_with(
        &service,
        &executor,
        checked(&[(1, 31)]),
        Some(whole(&[(1, T + 1, 32)])),
    )
    .await;

    let locales: Vec<Vec<String>> = executor
        .sent()
        .iter()
        .map(|sent| {
            url::Url::parse(&sent.url)
                .unwrap()
                .query_pairs()
                .filter(|(name, _)| name.contains("locale"))
                .map(|(name, value)| format!("{name}={value}"))
                .collect()
        })
        .collect();
    assert_eq!(
        locales,
        [
            ["locale=en"],
            ["locale=en"],
            ["locale=pt-br"],
            ["locale=pt-br"],
        ]
    );
}

/// Point 4: a note the list has and the API doesn't send whole is remembered with its hash.
#[tokio::test]
async fn a_note_never_sent_whole_is_asked_for_again_only_when_its_hash_changes() {
    let (service, executor) = service();
    // Note 2 doesn't come. Something that isn't a note, with no ID, is no note of the list's.
    let mut download = whole(&[(3, T + 3, 32), (1, T + 1, 12)]);
    download["notes"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "id": "not a number", "type": "like" }));
    let result = sync_with(
        &service,
        &executor,
        checked(&[(3, 31), (2, 21), (1, 11)]),
        Some(download),
    )
    .await;
    assert_eq!((result.downloaded_count, result.not_sent_count), (2, 1));
    let info = service.info().unwrap();
    assert_eq!((info.note_count, info.listed_only_count), (2, 1));

    // The same hashes: nothing is asked for.
    let result = sync_with(
        &service,
        &executor,
        checked(&[(3, 32), (2, 21), (1, 12)]),
        None,
    )
    .await;
    assert_eq!((result.downloaded_count, result.not_sent_count), (0, 0));

    // Note 2's hash changes, so it's asked for again, alone.
    let result = sync_with(
        &service,
        &executor,
        checked(&[(3, 32), (2, 22), (1, 12)]),
        Some(whole(&[])),
    )
    .await;
    assert_eq!((result.downloaded_count, result.not_sent_count), (0, 1));
    assert_eq!(downloads(&executor), [vec![3, 2, 1], vec![2]]);
    assert_eq!(shown(&service), [3, 1]);
}

/// Point 5: a short or empty page of whole notes isn't the end, and the end is a short list of
/// IDs.
#[tokio::test]
async fn a_short_page_of_whole_notes_is_not_the_end() {
    let (service, executor) = service();
    // The newest list is full; only two of its notes come whole.
    let result = sync_with(
        &service,
        &executor,
        checked(&[(9, 91), (8, 81), (7, 71)]),
        Some(whole(&[(9, T + 9, 92), (7, T + 7, 72)])),
    )
    .await;
    assert!(!result.reached_end);

    // An older page with no whole note at all: it moves on by the note it shares with the last
    // page, made bigger, and doesn't end.
    let result = older_with(
        &service,
        &executor,
        checked(&[(7, 72), (6, 61), (5, 51)]),
        Some(whole(&[])),
    )
    .await
    .unwrap();
    assert!(!result.reached_end);
    let sent = params(&executor);
    assert_eq!(sent[2]["before"], (T + 7).to_string());
    assert_eq!(sent.last().unwrap()["ids"], format!("{},{}", id(6), id(5)));

    // Asked again at the largest page, the list is short: every note left was listed.
    let result = older_with(
        &service,
        &executor,
        checked(&[(7, 72), (6, 61), (5, 51)]),
        None,
    )
    .await
    .unwrap();
    assert!(result.reached_end);
    assert_eq!(params(&executor)[4]["number"], "200");
    assert_eq!(shown(&service), [9, 7]);

    // At the end, loading older sends nothing.
    let before = executor.sent().len();
    assert!(service.load_older().await.unwrap().reached_end);
    assert_eq!(executor.sent().len(), before);
    assert!(service.info().unwrap().reached_end);
}

/// Point 5: a page that left a note out can be refilled with an older note, so it's full, and
/// paging carries on from its oldest note.
#[tokio::test]
async fn a_refilled_page_carries_on_from_its_oldest_note() {
    let (service, executor) = service();
    sync_with(
        &service,
        &executor,
        checked(&[(9, 91), (8, 81), (7, 71)]),
        // Note 8 can't be sent; note 6 fills its place.
        Some(whole(&[(9, T + 9, 92), (7, T + 7, 72)])),
    )
    .await;
    older_with(
        &service,
        &executor,
        checked(&[(7, 72), (6, 61), (5, 51)]),
        Some(whole(&[(6, T + 6, 62), (5, T + 5, 52)])),
    )
    .await
    .unwrap();
    assert_eq!(
        params(&executor).last().unwrap()["ids"],
        format!("{},{}", id(6), id(5))
    );
    let result = older_with(&service, &executor, checked(&[(5, 52)]), None)
        .await
        .unwrap();
    assert_eq!(
        params(&executor).last().unwrap()["before"],
        (T + 5).to_string()
    );
    assert!(result.reached_end);
    assert_eq!(shown(&service), [9, 7, 6, 5]);
}

/// Point 5 and 6: `before` includes its second, so notes sharing the oldest second come back on
/// the next page and none is skipped, and a page that can't move on grows to 200 once and then
/// stops rather than loop.
#[tokio::test]
async fn notes_sharing_a_second_are_not_skipped_and_paging_cannot_loop() {
    let (service, executor) = service();
    sync_with(
        &service,
        &executor,
        checked(&[(9, 91), (8, 81), (7, 71)]),
        Some(whole(&[(9, T + 9, 92), (8, T + 5, 82), (7, T + 5, 72)])),
    )
    .await;
    // Notes 8, 7, 6 and 5 share a second: the page is full of it.
    older_with(
        &service,
        &executor,
        checked(&[(8, 82), (7, 72), (6, 61)]),
        Some(whole(&[(6, T + 5, 62)])),
    )
    .await
    .unwrap();
    let sent = params(&executor);
    assert_eq!(sent[2]["before"], (T + 5).to_string());
    assert_eq!(sent[2]["number"], "3");

    // At 200, a full list that still brings nothing older is stuck: an error, and paging stays
    // where it was rather than skip the notes it can't get past.
    let full: Vec<(u64, u64)> = (1..=200).map(|n| (n + 100, n)).collect();
    executor.push(Step::Respond(200, checked(&full)));
    executor.push(Step::Respond(200, whole(&[])));
    executor.push(Step::Respond(200, whole(&[])));
    let error = service.load_older().await;
    assert!(matches!(
        error,
        Err(NotificationServiceError::CannotPageFurther)
    ));
    let sent = params(&executor);
    assert_eq!(sent[4]["number"], "200");
    assert_eq!(sent[4]["before"], (T + 5).to_string());
    // The notes it couldn't get are known now, so asking again downloads nothing, and fails the
    // same way.
    let error = older_with(&service, &executor, checked(&full), None).await;
    assert!(matches!(
        error,
        Err(NotificationServiceError::CannotPageFurther)
    ));
    let sent = params(&executor);
    assert_eq!(sent.len(), 8);
    assert_eq!(sent[7]["number"], "200");
    assert_eq!(sent[7]["before"], (T + 5).to_string());
}

/// Point 6: 200 notes at most per request, and downloads of at most 100.
#[tokio::test]
async fn no_request_asks_for_more_than_the_api_sends() {
    let (service, executor) = service_in(cache(), USER, 150);
    let listed: Vec<(u64, u64)> = (1..=150).rev().map(|n| (n, n)).collect();
    let first: Vec<(u64, i64, u64)> = (51..=150).rev().map(|n| (n, T + n as i64, n)).collect();
    let second: Vec<(u64, i64, u64)> = (1..=50).rev().map(|n| (n, T + n as i64, n)).collect();
    executor.push(Step::Respond(200, checked(&listed)));
    executor.push(Step::Respond(200, whole(&first)));
    executor.push(Step::Respond(200, whole(&second)));
    let result = service.sync().await.unwrap();
    assert_eq!(result.downloaded_count, 150);
    let sizes: Vec<usize> = downloads(&executor).iter().map(Vec::len).collect();
    assert_eq!(sizes, [100, 50]);
    assert!(
        params(&executor)
            .iter()
            .all(|params| params["number"].parse::<u32>().unwrap() <= MAX_PAGE_SIZE)
    );
}

/// Point 7: a note goes when the list of IDs for its window doesn't have it, and only then.
#[tokio::test]
async fn a_note_deleted_on_the_server_is_removed_and_older_notes_stay() {
    let (service, executor) = service();
    sync_with(
        &service,
        &executor,
        checked(&[(5, 51), (4, 41), (3, 31)]),
        Some(whole(&[(5, T + 5, 52), (4, T + 4, 42), (3, T + 3, 32)])),
    )
    .await;
    older_with(
        &service,
        &executor,
        checked(&[(3, 32), (2, 21), (1, 11)]),
        Some(whole(&[(2, T + 3, 22), (1, T + 1, 12)])),
    )
    .await
    .unwrap();
    assert_eq!(shown(&service), [5, 4, 3, 2, 1]);

    // Note 4 was deleted. The newest list now reaches note 2, which shares note 3's second:
    // note 4 goes, note 1 is older than the list reaches and stays.
    let result = sync_with(
        &service,
        &executor,
        checked(&[(5, 52), (3, 32), (2, 22)]),
        None,
    )
    .await;
    assert_eq!(result.removed_count, 1);
    assert_eq!(shown(&service), [5, 3, 2, 1]);

    // A note the whole notes leave out isn't removed for that: only the list of IDs counts.
    let result = sync_with(
        &service,
        &executor,
        checked(&[(6, 61), (5, 52), (3, 32)]),
        Some(whole(&[])),
    )
    .await;
    assert_eq!(result.removed_count, 0);
    assert_eq!(shown(&service), [5, 3, 2, 1]);

    // A short list of the newest notes is all there is: what it doesn't have goes.
    let result = sync_with(&service, &executor, checked(&[(5, 52)]), None).await;
    assert_eq!(result.removed_count, 3);
    assert!(result.reached_end);
    assert_eq!(shown(&service), [5]);
    assert_eq!(service.info().unwrap().listed_only_count, 0);
}

#[tokio::test]
async fn a_short_older_page_removes_what_it_does_not_list_from_its_before_back() {
    let (service, executor) = service();
    sync_with(
        &service,
        &executor,
        checked(&[(5, 51), (4, 41), (3, 31)]),
        Some(whole(&[(5, T + 5, 52), (4, T + 4, 42), (3, T + 3, 32)])),
    )
    .await;
    older_with(
        &service,
        &executor,
        checked(&[(3, 32), (2, 21), (1, 11)]),
        Some(whole(&[(2, T + 2, 22), (1, T + 1, 12)])),
    )
    .await
    .unwrap();
    // Note 2 was deleted before the next page.
    let result = older_with(&service, &executor, checked(&[(1, 12)]), None)
        .await
        .unwrap();
    assert_eq!(
        params(&executor).last().unwrap()["before"],
        (T + 1).to_string()
    );
    assert_eq!(result.removed_count, 0, "Note 2 is newer than that page");
    assert!(result.reached_end);
    assert_eq!(shown(&service), [5, 4, 3, 2, 1]);
}

/// Point 8: the service sets no `User-Agent` of its own, so the app's executor decides it.
#[tokio::test]
async fn the_user_agent_is_left_to_the_apps_executor() {
    let (service, executor) = service();
    sync_with(
        &service,
        &executor,
        checked(&[(1, 11)]),
        Some(whole(&[(1, T + 1, 12)])),
    )
    .await;
    assert!(executor.sent().iter().all(|sent| sent.user_agent.is_none()));
}

/// Point 9: a note with no date doesn't pull paging to 1970, isn't lost, and is shown last.
#[tokio::test]
async fn a_note_with_no_date_does_not_move_paging_to_1970() {
    let (service, executor) = service();
    sync_with(
        &service,
        &executor,
        checked(&[(9, 91), (2, 21), (8, 81)]),
        Some(whole(&[(9, T + 9, 92), (2, 0, 22), (8, T + 8, 82)])),
    )
    .await;
    assert_eq!(shown(&service), [9, 8, 2]);
    older_with(
        &service,
        &executor,
        checked(&[(8, 82), (7, 71), (1, 11)]),
        Some(whole(&[(7, T + 7, 72), (1, 1, 12)])),
    )
    .await
    .unwrap();
    let sent = params(&executor);
    assert_eq!(sent[2]["before"], (T + 8).to_string());
    assert_eq!(sent.last().unwrap()["ids"], format!("{},{}", id(7), id(1)));
    let result = older_with(&service, &executor, checked(&[(7, 72), (1, 12)]), None)
        .await
        .unwrap();
    assert_eq!(
        params(&executor).last().unwrap()["before"],
        (T + 7).to_string()
    );
    assert!(result.reached_end);
    assert_eq!(shown(&service), [9, 8, 7, 1, 2]);
}

#[tokio::test]
async fn a_page_of_notes_with_no_date_pages_from_second_one() {
    let (service, executor) = service_in(cache(), USER, 2);
    sync_with(
        &service,
        &executor,
        checked(&[(2, 21), (1, 11)]),
        Some(whole(&[(2, 0, 22), (1, 0, 12)])),
    )
    .await;
    let result = older_with(&service, &executor, checked(&[(2, 22), (1, 12)]), None)
        .await
        .unwrap();
    assert_eq!(params(&executor)[2]["before"], "1");
    assert!(!result.reached_end);
    let result = older_with(&service, &executor, checked(&[(2, 22), (1, 12)]), None)
        .await
        .unwrap();
    assert!(result.reached_end);
    assert_eq!(shown(&service), [2, 1]);
}

/// More new notes than a sync checks: the older pages run again from the new window, so the
/// notes between are loaded.
#[tokio::test]
async fn a_gap_after_many_new_notes_is_paged_again() {
    let (service, executor) = service();
    let result = sync_with(
        &service,
        &executor,
        checked(&[(2, 21), (1, 11)]),
        Some(whole(&[(2, T + 2, 22), (1, T + 1, 12)])),
    )
    .await;
    assert!(result.reached_end);
    sync_with(
        &service,
        &executor,
        checked(&[(9, 91), (8, 81), (7, 71)]),
        Some(whole(&[(9, T + 9, 92), (8, T + 8, 82), (7, T + 7, 72)])),
    )
    .await;
    assert!(!service.info().unwrap().reached_end);
    let result = older_with(
        &service,
        &executor,
        checked(&[(7, 72), (6, 61), (5, 51)]),
        Some(whole(&[(6, T + 6, 62), (5, T + 5, 52)])),
    )
    .await
    .unwrap();
    assert!(!result.reached_end);
    assert_eq!(
        params(&executor).last().unwrap()["ids"],
        format!("{},{}", id(6), id(5))
    );
    assert_eq!(shown(&service), [9, 8, 7, 6, 5, 2, 1]);
}

#[tokio::test]
async fn two_accounts_keep_their_own_notes() {
    let cache = cache();
    let (first, first_executor) = service_in(cache.clone(), WpComUserId(1), 3);
    let (second, second_executor) = service_in(cache.clone(), WpComUserId(2), 3);
    sync_with(
        &first,
        &first_executor,
        checked(&[(1, 11)]),
        Some(whole(&[(1, T + 1, 12)])),
    )
    .await;
    // The same note ID with another hash, for the second account, is downloaded for it.
    sync_with(
        &second,
        &second_executor,
        checked(&[(2, 21), (1, 99)]),
        Some(whole(&[(2, T + 2, 22), (1, T + 1, 98)])),
    )
    .await;
    assert_eq!(shown(&first), [1]);
    assert_eq!(shown(&second), [2, 1]);
    // The first account's short list removes nothing of the second's.
    sync_with(&first, &first_executor, checked(&[]), None).await;
    assert!(shown(&first).is_empty());
    assert_eq!(shown(&second), [2, 1]);
}

#[tokio::test]
async fn signing_out_removes_the_accounts_notes() {
    let cache = cache();
    let (first, first_executor) = service_in(cache.clone(), WpComUserId(1), 3);
    let (second, second_executor) = service_in(cache.clone(), WpComUserId(2), 3);
    for (service, executor) in [(&first, &first_executor), (&second, &second_executor)] {
        sync_with(
            service,
            executor,
            checked(&[(2, 21), (1, 11)]),
            Some(whole(&[(2, T + 2, 22)])),
        )
        .await;
    }
    assert!(cache.remove_wordpress_com_account(WpComUserId(1)).unwrap());

    let (signed_in_again, _) = service_in(cache.clone(), WpComUserId(1), 3);
    assert!(shown(&signed_in_again).is_empty());
    let info = signed_in_again.info().unwrap();
    assert_eq!((info.note_count, info.listed_only_count), (0, 0));
    assert!(info.last_synced_at.is_none());
    assert_eq!(shown(&second), [2]);
    assert_eq!(second.info().unwrap().listed_only_count, 1);
}

#[tokio::test]
async fn the_cached_list_filters_by_read_state_and_type() {
    let (service, executor) = service();
    let mut download = whole(&[(3, T + 3, 32), (2, T + 2, 22), (1, T + 1, 12)]);
    download["notes"][1]["read"] = json!(1);
    download["notes"][2]["type"] = json!("comment");
    sync_with(
        &service,
        &executor,
        checked(&[(3, 31), (2, 21), (1, 11)]),
        Some(download),
    )
    .await;
    let ids = |filter: NotificationsFilter| -> Vec<u64> {
        service
            .load_notes(filter)
            .unwrap()
            .iter()
            .map(|note| note.id.0 - id(0))
            .collect()
    };
    assert_eq!(
        ids(NotificationsFilter {
            unread_only: true,
            types: Vec::new()
        }),
        [3, 1]
    );
    assert_eq!(
        ids(NotificationsFilter {
            unread_only: false,
            types: vec![NotificationType::Comment]
        }),
        [1]
    );
}

#[tokio::test]
async fn a_change_to_the_notification_tables_is_relevant() {
    let (service, _) = service();
    let hook = |table| UpdateHook {
        action: wp_mobile_cache::HookAction::Insert,
        db_name: "main".to_string(),
        table,
        row_id: 1,
    };
    assert!(service.is_relevant_update(&hook(DbTable::Notifications)));
    assert!(service.is_relevant_update(&hook(DbTable::NotificationSyncState)));
    assert!(!service.is_relevant_update(&hook(DbTable::ReaderPosts)));
}

#[tokio::test]
async fn a_failed_check_changes_nothing() {
    let (service, executor) = service();
    sync_with(
        &service,
        &executor,
        checked(&[(1, 11)]),
        Some(whole(&[(1, T + 1, 12)])),
    )
    .await;
    let before = service.info().unwrap();
    executor.push(Step::Offline);
    assert!(matches!(
        service.sync().await,
        Err(NotificationServiceError::Api(_))
    ));
    assert_eq!(service.info().unwrap(), before);
    assert_eq!(shown(&service), [1]);
}

// The tabs

/// Note `n` at `time`, of `note_type`, read or not, as a whole note comes.
fn typed_json(n: u64, time: i64, hash: u64, note_type: &str, read: bool) -> Value {
    let mut note = note_json(n, time, hash);
    note["type"] = json!(note_type);
    note["read"] = json!(u8::from(read));
    note
}

/// A page of whole notes, each `(n, time, hash, type, read)`.
fn whole_typed(notes: &[(u64, i64, u64, &str, bool)]) -> Value {
    let notes: Vec<Value> = notes
        .iter()
        .map(|(n, time, hash, note_type, read)| typed_json(*n, *time, *hash, note_type, *read))
        .collect();
    json!({ "last_seen_time": T.to_string(), "notes": notes })
}

fn tab_shown(service: &NotificationService, tab: NotificationTab) -> Vec<u64> {
    service
        .load_tab_notes(tab)
        .unwrap()
        .iter()
        .map(|note| note.id.0 - id(0))
        .collect()
}

async fn sync_tab_with(
    service: &NotificationService,
    executor: &ScriptedExecutor,
    tab: NotificationTab,
    check: Value,
    downloads: Vec<Value>,
) -> NotificationSyncResult {
    executor.push(Step::Respond(200, check));
    for download in downloads {
        executor.push(Step::Respond(200, download));
    }
    service.sync_tab(tab).await.unwrap()
}

/// One note of each kind a tab picks out, synced through the All tab.
async fn one_of_each(service: &NotificationService, executor: &ScriptedExecutor) {
    sync_with(
        service,
        executor,
        checked(&[(6, 61), (5, 51), (4, 41), (3, 31), (2, 21), (1, 11)]),
        Some(whole_typed(&[
            (6, T + 6, 62, "comment_like", true),
            (5, T + 5, 52, "like", true),
            (4, T + 4, 42, "follow", true),
            (3, T + 3, 32, "automattcher", false),
            (2, T + 2, 22, "comment", false),
            (1, T + 1, 12, "comment", true),
        ])),
    )
    .await;
}

#[tokio::test]
async fn each_tab_shows_the_cached_notes_the_web_panel_does() {
    let (service, executor) = service_in(cache(), USER, 10);
    one_of_each(&service, &executor).await;
    assert_eq!(
        tab_shown(&service, NotificationTab::All),
        [6, 5, 4, 3, 2, 1]
    );
    assert_eq!(tab_shown(&service, NotificationTab::Unread), [3, 2]);
    assert_eq!(tab_shown(&service, NotificationTab::Comments), [2, 1]);
    assert_eq!(tab_shown(&service, NotificationTab::Subscribers), [4]);
    assert_eq!(tab_shown(&service, NotificationTab::Likes), [6, 5]);
}

#[tokio::test]
async fn each_tab_asks_the_server_for_its_own_notes() {
    let (service, executor) = service();
    for tab in NotificationTab::EVERY_TAB {
        sync_tab_with(&service, &executor, tab, checked(&[]), Vec::new()).await;
    }
    let filters: Vec<(Option<String>, Option<String>)> = params(&executor)
        .iter()
        .map(|params| {
            assert_eq!(params["fields"], "id,note_hash");
            assert_eq!(params["locale"], "en");
            (params.get("type").cloned(), params.get("unread").cloned())
        })
        .collect();
    assert_eq!(
        filters,
        [
            (None, None),
            (None, Some("1".to_string())),
            (Some("comment".to_string()), None),
            (Some("follow".to_string()), None),
            (Some("like,comment_like".to_string()), None),
        ]
    );
}

/// A tab's list has the same hashes as the list of every note, so what one tab downloaded
/// another doesn't download again, and the note is stored once.
#[tokio::test]
async fn a_note_another_tab_downloaded_is_not_downloaded_again() {
    let (service, executor) = service();
    sync_tab_with(
        &service,
        &executor,
        NotificationTab::Comments,
        checked(&[(2, 21), (1, 11)]),
        vec![whole_typed(&[
            (2, T + 2, 22, "comment", false),
            (1, T + 1, 12, "comment", true),
        ])],
    )
    .await;
    // The Unread tab lists note 2 with the hash it came with: nothing is asked for.
    let result = sync_tab_with(
        &service,
        &executor,
        NotificationTab::Unread,
        checked(&[(3, 31), (2, 22)]),
        vec![whole_typed(&[(3, T + 3, 32, "like", false)])],
    )
    .await;
    assert_eq!(result.downloaded_count, 1);
    // The All tab has every note already.
    let result = sync_with(
        &service,
        &executor,
        checked(&[(3, 32), (2, 22), (1, 12)]),
        None,
    )
    .await;
    assert_eq!(result.downloaded_count, 0);
    assert_eq!(downloads(&executor), [vec![2, 1], vec![3]]);
    assert_eq!(service.info().unwrap().note_count, 3);
    assert_eq!(tab_shown(&service, NotificationTab::Unread), [3, 2]);
}

#[tokio::test]
async fn each_tab_keeps_its_own_place_and_end() {
    let (service, executor) = service();
    // The Likes tab's newest list is full, and its next page is short: it reaches its end.
    sync_tab_with(
        &service,
        &executor,
        NotificationTab::Likes,
        checked(&[(9, 91), (8, 81), (7, 71)]),
        vec![whole_typed(&[
            (9, T + 9, 92, "like", true),
            (8, T + 8, 82, "like", true),
            (7, T + 7, 72, "comment_like", true),
        ])],
    )
    .await;
    assert!(!service.tab_reached_end(NotificationTab::Likes).unwrap());
    executor.push(Step::Respond(200, checked(&[(7, 72), (2, 21)])));
    executor.push(Step::Respond(
        200,
        whole_typed(&[(2, T + 2, 22, "like", true)]),
    ));
    let result = service
        .load_older_in_tab(NotificationTab::Likes)
        .await
        .unwrap();
    assert!(result.reached_end);
    let older = &params(&executor)[2];
    assert_eq!(older["before"], (T + 7).to_string());
    assert_eq!(older["type"], "like,comment_like");
    assert!(service.tab_reached_end(NotificationTab::Likes).unwrap());

    // The other tabs haven't started.
    for tab in [
        NotificationTab::All,
        NotificationTab::Unread,
        NotificationTab::Comments,
    ] {
        assert!(!service.tab_reached_end(tab).unwrap());
    }
    assert!(!service.info().unwrap().reached_end);
    // The All tab pages from its own newest list, not from where Likes stopped.
    sync_with(
        &service,
        &executor,
        checked(&[(10, 101), (9, 92), (8, 82)]),
        Some(whole_typed(&[(10, T + 10, 102, "comment", false)])),
    )
    .await;
    executor.push(Step::Respond(200, checked(&[(8, 82), (7, 72)])));
    service.load_older().await.unwrap();
    assert_eq!(
        params(&executor).last().unwrap()["before"],
        (T + 8).to_string()
    );
    assert!(!params(&executor).last().unwrap().contains_key("type"));
    // At its end, the Likes tab sends nothing.
    let sent = executor.sent().len();
    assert!(
        service
            .load_older_in_tab(NotificationTab::Likes)
            .await
            .unwrap()
            .reached_end
    );
    assert_eq!(executor.sent().len(), sent);
}

/// A note missing from the Unread tab's list may only have been read. It's downloaded again,
/// which brings whether it was read, and never removed for that.
#[tokio::test]
async fn the_unread_tab_never_removes_a_note() {
    let (service, executor) = service_in(cache(), USER, 10);
    one_of_each(&service, &executor).await;
    assert_eq!(tab_shown(&service, NotificationTab::Unread), [3, 2]);

    // Note 2 was read elsewhere, and note 3 was deleted: the unread list has neither. Both are
    // downloaded again; note 2 comes, read, and note 3 doesn't come.
    let result = sync_tab_with(
        &service,
        &executor,
        NotificationTab::Unread,
        checked(&[]),
        vec![whole_typed(&[(2, T + 2, 23, "comment", true)])],
    )
    .await;
    assert_eq!(result.removed_count, 0);
    assert_eq!(result.downloaded_count, 1);
    assert_eq!(downloads(&executor).last().unwrap(), &vec![3, 2]);
    assert_eq!(tab_shown(&service, NotificationTab::Unread), [3]);
    // Nothing was removed: note 3 is left for the All tab's sync, which lists every note.
    assert_eq!(
        tab_shown(&service, NotificationTab::All),
        [6, 5, 4, 3, 2, 1]
    );
    let result = sync_with(
        &service,
        &executor,
        checked(&[(6, 62), (5, 52), (4, 42), (2, 23), (1, 12)]),
        None,
    )
    .await;
    assert_eq!(result.removed_count, 1);
    assert!(tab_shown(&service, NotificationTab::Unread).is_empty());
}

/// A cached note that the Unread tab's list has, though it was cached as read, was marked
/// unread since: it's downloaded again, even if its hash didn't change.
#[tokio::test]
async fn a_note_the_unread_list_has_and_the_cache_has_as_read_is_downloaded_again() {
    let (service, executor) = service_in(cache(), USER, 10);
    one_of_each(&service, &executor).await;
    let result = sync_tab_with(
        &service,
        &executor,
        NotificationTab::Unread,
        checked(&[(5, 52), (3, 32), (2, 22)]),
        vec![whole_typed(&[(5, T + 5, 53, "like", false)])],
    )
    .await;
    assert_eq!(result.downloaded_count, 1);
    assert_eq!(downloads(&executor).last().unwrap(), &vec![5]);
    assert_eq!(tab_shown(&service, NotificationTab::Unread), [5, 3, 2]);
}

/// A note's type never changes, so a note of the tab's type that its list doesn't have is gone.
/// Notes of other types, and notes the API never sent whole, stay.
#[tokio::test]
async fn a_type_tab_removes_only_its_own_notes() {
    let (service, executor) = service_in(cache(), USER, 10);
    sync_with(
        &service,
        &executor,
        checked(&[(4, 41), (3, 31), (2, 21), (1, 11)]),
        Some(whole_typed(&[
            (3, T + 3, 32, "like", true),
            (2, T + 2, 22, "comment", true),
            (1, T + 1, 12, "comment", true),
        ])),
    )
    .await;
    assert_eq!(service.info().unwrap().listed_only_count, 1);
    // Note 2 was deleted.
    let result = sync_tab_with(
        &service,
        &executor,
        NotificationTab::Comments,
        checked(&[(1, 12)]),
        Vec::new(),
    )
    .await;
    assert_eq!(result.removed_count, 1);
    assert_eq!(tab_shown(&service, NotificationTab::All), [3, 1]);
    assert_eq!(service.info().unwrap().listed_only_count, 1);
}

/// Another tab can download notes newer than the All tab's last sync. They mustn't hide that
/// more notes arrived than the All tab's list holds.
#[tokio::test]
async fn notes_another_tab_downloaded_do_not_hide_a_gap() {
    let (service, executor) = service();
    let result = sync_with(
        &service,
        &executor,
        checked(&[(2, 21), (1, 11)]),
        Some(whole(&[(2, T + 2, 22), (1, T + 1, 12)])),
    )
    .await;
    assert!(result.reached_end);
    sync_tab_with(
        &service,
        &executor,
        NotificationTab::Comments,
        checked(&[(9, 91), (8, 81)]),
        vec![whole_typed(&[
            (9, T + 9, 92, "comment", true),
            (8, T + 8, 82, "comment", true),
        ])],
    )
    .await;
    // Notes 9 and 8 are cached, but the list doesn't reach note 2: a gap.
    sync_with(
        &service,
        &executor,
        checked(&[(9, 92), (8, 82), (7, 71)]),
        Some(whole(&[(7, T + 7, 72)])),
    )
    .await;
    assert!(!service.info().unwrap().reached_end);
    let result = older_with(
        &service,
        &executor,
        checked(&[(7, 72), (6, 61), (2, 22)]),
        Some(whole(&[(6, T + 6, 62)])),
    )
    .await
    .unwrap();
    assert_eq!(
        params(&executor)[6]["before"],
        (T + 7).to_string(),
        "{:?}",
        params(&executor)
    );
    assert!(!result.reached_end);
    assert_eq!(shown(&service), [9, 8, 7, 6, 2, 1]);
}

// When the app hears of a change

#[derive(Default)]
struct Hooks(Mutex<Vec<UpdateHook>>);

impl wp_mobile_cache::DatabaseDelegate for Hooks {
    fn did_update(&self, update_hook: UpdateHook) {
        self.0.lock().unwrap().push(update_hook);
    }
}

/// Runs `act` while listening to the cache, and returns the hooks it fired.
async fn hooks_of<F: std::future::Future>(
    cache: &WpApiCache,
    act: F,
) -> (F::Output, Vec<UpdateHook>) {
    let hooks = Arc::new(Hooks::default());
    cache.start_listening_for_updates(hooks.clone());
    let output = act.await;
    cache.stop_listening_for_updates();
    let fired = hooks.0.lock().unwrap().clone();
    (output, fired)
}

#[tokio::test]
async fn refreshing_a_note_stores_it_and_tells_observers() {
    let cache = cache();
    let (service, executor) = service_in(cache.clone(), USER, 3);
    sync_with(
        &service,
        &executor,
        checked(&[(1, 11)]),
        Some(whole(&[(1, T + 1, 12)])),
    )
    .await;

    // A push says note 2 arrived, and note 1 changed.
    executor.push(Step::Respond(200, whole(&[(2, T + 2, 22)])));
    executor.push(Step::Respond(200, whole(&[(1, T + 1, 13)])));
    let ((new, changed), hooks) = hooks_of(&cache, async {
        (
            service.refresh_note(NotificationId(id(2))).await.unwrap(),
            service.refresh_note(NotificationId(id(1))).await.unwrap(),
        )
    })
    .await;
    assert_eq!(
        (new, changed),
        (
            NotificationRefreshResult::Stored,
            NotificationRefreshResult::Stored
        )
    );
    assert_eq!(shown(&service), [2, 1]);
    assert!(!hooks.is_empty());
    assert!(hooks.iter().all(|hook| service.is_relevant_update(hook)));
    assert!(
        hooks
            .iter()
            .any(|hook| hook.table == DbTable::Notifications)
    );

    // Each was fetched the way a sync downloads notes, so the next sync downloads neither.
    let sent = params(&executor);
    for (request, n) in sent[2..].iter().zip([2, 1]) {
        assert_eq!(request["ids"], id(n).to_string());
        assert_eq!(request["number"], "1");
        assert_eq!(request["locale"], "en");
        assert!(request["fields"].ends_with(",header,body,note_context"));
    }
    let result = sync_with(&service, &executor, checked(&[(2, 22), (1, 13)]), None).await;
    assert_eq!(result.downloaded_count, 0);
}

#[tokio::test]
async fn refreshing_a_note_that_does_not_come_removes_it() {
    let cache = cache();
    let (service, executor) = service_in(cache.clone(), USER, 3);
    sync_with(
        &service,
        &executor,
        checked(&[(2, 21), (1, 11)]),
        Some(whole(&[(2, T + 2, 22), (1, T + 1, 12)])),
    )
    .await;
    executor.push(Step::Respond(200, whole(&[])));
    let (result, hooks) = hooks_of(&cache, service.refresh_note(NotificationId(id(2)))).await;
    assert_eq!(result.unwrap(), NotificationRefreshResult::NotSent);
    assert_eq!(shown(&service), [1]);
    assert!(hooks.iter().any(|hook| hook.table == DbTable::Notifications
        && hook.action == wp_mobile_cache::HookAction::Delete));
    assert!(hooks.iter().all(|hook| service.is_relevant_update(hook)));

    // One that comes and can't be read, here because it has a key twice, is left out until it
    // changes.
    executor.push(Step::RespondText(
        200,
        format!(
            r#"{{"last_seen_time":"{T}","notes":[{{"id":{},"type":"like","read":0,"read":1}}]}}"#,
            id(1)
        ),
    ));
    assert_eq!(
        service.refresh_note(NotificationId(id(1))).await.unwrap(),
        NotificationRefreshResult::Unreadable
    );
    assert!(shown(&service).is_empty());
    assert_eq!(service.info().unwrap().unreadable_count, 1);
}

#[tokio::test]
async fn removing_a_note_sends_nothing_and_tells_observers() {
    let cache = cache();
    let (service, executor) = service_in(cache.clone(), USER, 3);
    sync_with(
        &service,
        &executor,
        checked(&[(3, 31), (2, 21), (1, 11)]),
        Some(whole(&[(2, T + 2, 22), (1, T + 1, 12)])),
    )
    .await;
    let sent = executor.sent().len();
    let ((whole_note, listed_only, unknown), hooks) = hooks_of(&cache, async {
        (
            service.remove_note(NotificationId(id(2))).await.unwrap(),
            service.remove_note(NotificationId(id(3))).await.unwrap(),
            service.remove_note(NotificationId(id(4))).await.unwrap(),
        )
    })
    .await;
    assert_eq!((whole_note, listed_only, unknown), (true, true, false));
    assert_eq!(executor.sent().len(), sent);
    assert_eq!(shown(&service), [1]);
    assert_eq!(service.info().unwrap().listed_only_count, 0);
    let tables: HashSet<DbTable> = hooks.iter().map(|hook| hook.table).collect();
    assert_eq!(
        tables,
        HashSet::from([DbTable::Notifications, DbTable::NotificationListedOnly])
    );
    assert!(hooks.iter().all(|hook| service.is_relevant_update(hook)));
}

/// With no socket, an app runs `sync` on a timer. When nothing changed, that's one GET that asks
/// only for IDs and hashes.
#[tokio::test]
async fn a_check_with_nothing_changed_is_one_small_request() {
    let (service, executor) = service();
    sync_with(
        &service,
        &executor,
        checked(&[(3, 31), (2, 21), (1, 11)]),
        Some(whole(&[(3, T + 3, 32), (2, T + 2, 22), (1, T + 1, 12)])),
    )
    .await;
    let before = executor.sent().len();
    let result = sync_with(
        &service,
        &executor,
        checked(&[(3, 32), (2, 22), (1, 12)]),
        None,
    )
    .await;
    assert_eq!(result.downloaded_count, 0);
    let sent = &executor.sent()[before..];
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].method, RequestMethod::GET);
    let query = &params(&executor)[before];
    assert_eq!(query["fields"], "id,note_hash");
    assert!(!query.contains_key("ids"));
}
