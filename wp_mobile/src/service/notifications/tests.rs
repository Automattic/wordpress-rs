//! Every note, ID, hash and time here is invented.

use super::*;
use crate::testing::scripted::{ScriptedExecutor, Step, delegate};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Mutex;
use wp_api::prelude::RequestMethod;
use wp_api::wp_com::language::WpComLanguageProvider;
use wp_api::wp_com::notifications::NotificationType;
use wp_mobile_cache::repository::notifications::NotificationRepository;

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
    // A like or follow waiting to be sent shows on the notes.
    assert!(service.is_relevant_update(&hook(DbTable::ReaderActions)));
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

// Marking notes read, unread and seen

/// Three unread notes, 3 newest, synced through the All tab. Notes 1 and 3 are likes.
async fn three_unread(service: &NotificationService, executor: &ScriptedExecutor) {
    sync_with(
        service,
        executor,
        checked(&[(3, 31), (2, 21), (1, 11)]),
        Some(whole_typed(&[
            (3, T + 3, 32, "like", false),
            (2, T + 2, 22, "comment", false),
            (1, T + 1, 12, "like", false),
        ])),
    )
    .await;
}

/// Each POST request's path and query, after `rest/v1.1`, and its body.
fn posts(executor: &ScriptedExecutor) -> Vec<(String, Value)> {
    executor
        .sent()
        .iter()
        .filter(|sent| sent.method == RequestMethod::POST)
        .map(|sent| {
            let url = url::Url::parse(&sent.url).unwrap();
            let path = url.path().trim_start_matches("/rest/v1.1").to_string();
            let path = match url.query() {
                Some(query) => format!("{path}?{query}"),
                None => path,
            };
            (path, sent.body.clone().unwrap_or(Value::Null))
        })
        .collect()
}

fn counts(pairs: &[(u64, i64)]) -> Value {
    let counts: serde_json::Map<String, Value> = pairs
        .iter()
        .map(|(n, count)| (id(*n).to_string(), json!(count)))
        .collect();
    json!({ "counts": counts, "source": "mobile" })
}

fn taken(unread_afterwards: &[u64]) -> Value {
    let updated: Vec<u64> = unread_afterwards.iter().map(|n| id(*n)).collect();
    json!({ "success": true, "updated": updated })
}

fn note_ids(ns: &[u64]) -> Vec<NotificationId> {
    ns.iter().map(|n| NotificationId(id(*n))).collect()
}

/// Whether each cached note shows as read, newest first.
fn read_states(service: &NotificationService) -> Vec<(u64, Option<bool>)> {
    service
        .load_notes(NotificationsFilter::default())
        .unwrap()
        .iter()
        .map(|note| (note.id.0 - id(0), note.read))
        .collect()
}

#[tokio::test]
async fn marking_a_note_read_shows_at_once_and_sends_one_post() {
    let cache = cache();
    let (service, executor) = service_in(cache.clone(), USER, 3);
    service.set_language(Some(WPComLanguage::German));
    three_unread(&service, &executor).await;

    executor.push(Step::Respond(200, taken(&[])));
    let (outcome, hooks) = hooks_of(&cache, service.mark_read(note_ids(&[1]))).await;

    assert_eq!(outcome.unwrap(), NotificationChangeOutcome::Sent);
    // The write is sent in the service's language, as every request is, and names the client.
    assert_eq!(
        posts(&executor),
        [(
            "/notifications/read?locale=de".to_string(),
            counts(&[(1, 9999)])
        )]
    );
    assert_eq!(tab_shown(&service, NotificationTab::Unread), [3, 2]);
    assert_eq!(
        read_states(&service),
        [(3, Some(false)), (2, Some(false)), (1, Some(true))]
    );
    assert_eq!(service.pending_change_count().unwrap(), 0);
    assert!(hooks.iter().any(|hook| service.is_relevant_update(hook)));
    assert!(
        hooks
            .iter()
            .any(|hook| hook.table == DbTable::NotificationReadChanges)
    );

    // The server's state is stored in the note itself, so it outlasts the change.
    let stored = service.cache.execute(|connection| {
        NotificationRepository::read_states(&*connection, service.account, &note_ids(&[1]))
    });
    assert_eq!(stored.unwrap()[&NotificationId(id(1))], Some(true));
}

#[tokio::test]
async fn marking_a_note_unread_brings_it_back_to_the_unread_tab() {
    let (service, executor) = service();
    three_unread(&service, &executor).await;
    executor.push(Step::Respond(200, taken(&[])));
    service.mark_read(note_ids(&[2, 3])).await.unwrap();
    assert_eq!(tab_shown(&service, NotificationTab::Unread), [1]);

    executor.push(Step::Respond(200, taken(&[2])));
    let outcome = service.mark_unread(note_ids(&[2])).await.unwrap();
    assert_eq!(outcome, NotificationChangeOutcome::Sent);
    assert_eq!(tab_shown(&service, NotificationTab::Unread), [2, 1]);
    let posts = posts(&executor);
    assert_eq!(posts[0].1, counts(&[(2, 9999), (3, 9999)]));
    assert_eq!(posts[1].1, counts(&[(2, -1)]));
}

#[tokio::test]
async fn a_note_that_already_shows_that_way_sends_nothing() {
    let (service, executor) = service();
    three_unread(&service, &executor).await;
    let before = executor.sent().len();
    let outcome = service.mark_unread(note_ids(&[1, 2])).await.unwrap();
    assert_eq!(outcome, NotificationChangeOutcome::Unchanged);
    assert_eq!(
        service.mark_read(Vec::new()).await.unwrap(),
        NotificationChangeOutcome::Unchanged
    );
    assert_eq!(executor.sent().len(), before);
}

#[tokio::test]
async fn a_note_the_cache_does_not_have_is_still_sent() {
    let (service, executor) = service();
    executor.push(Step::Respond(200, taken(&[])));
    let outcome = service.mark_read(note_ids(&[7])).await.unwrap();
    assert_eq!(outcome, NotificationChangeOutcome::Sent);
    assert_eq!(posts(&executor)[0].1, counts(&[(7, 9999)]));
}

/// Offline, the change shows and waits; the next `send_pending_changes` sends it.
#[tokio::test]
async fn a_change_made_offline_shows_and_is_sent_later() {
    let (service, executor) = service();
    three_unread(&service, &executor).await;

    executor.push(Step::Offline);
    let outcome = service.mark_read(note_ids(&[3])).await.unwrap();
    assert_eq!(outcome, NotificationChangeOutcome::Waiting);
    assert_eq!(tab_shown(&service, NotificationTab::Unread), [2, 1]);
    assert_eq!(service.pending_change_count().unwrap(), 1);

    // Still offline: it stays, and nothing else is tried after the first request fails.
    executor.push(Step::TimedOut);
    assert_eq!(service.send_pending_changes().await.unwrap(), 1);

    executor.push(Step::Respond(200, taken(&[])));
    assert_eq!(service.send_pending_changes().await.unwrap(), 0);
    assert_eq!(tab_shown(&service, NotificationTab::Unread), [2, 1]);
    assert_eq!(posts(&executor).len(), 3);
    assert!(
        posts(&executor)
            .iter()
            .all(|(_, body)| *body == counts(&[(3, 9999)]))
    );
}

/// A rejected sign-in is no answer: the change waits for the user to sign in again.
#[tokio::test]
async fn a_change_the_server_rejects_the_sign_in_for_waits() {
    let (service, executor) = service();
    three_unread(&service, &executor).await;
    executor.push(Step::Respond(
        401,
        json!({ "error": "invalid_token", "message": "An invented message." }),
    ));
    let outcome = service.mark_read(note_ids(&[3])).await.unwrap();
    assert_eq!(outcome, NotificationChangeOutcome::Waiting);
    assert_eq!(service.pending_change_count().unwrap(), 1);
}

/// Refused for good: the change goes, the note shows as the server has it, and the call fails.
#[tokio::test]
async fn a_change_the_server_refuses_goes_back() {
    let (service, executor) = service();
    three_unread(&service, &executor).await;

    executor.push(Step::Respond(
        403,
        json!({ "error": "unauthorized", "message": "An invented message." }),
    ));
    let error = service.mark_read(note_ids(&[3])).await.unwrap_err();
    assert!(
        matches!(error, NotificationServiceError::Api(_)),
        "{error:?}"
    );
    assert_eq!(tab_shown(&service, NotificationTab::Unread), [3, 2, 1]);
    assert_eq!(service.pending_change_count().unwrap(), 0);

    // A success that says it failed is a refusal too.
    executor.push(Step::Respond(200, json!({ "success": false })));
    let error = service.mark_read(note_ids(&[3])).await.unwrap_err();
    assert!(
        matches!(error, NotificationServiceError::ChangeNotMade),
        "{error:?}"
    );
    assert_eq!(tab_shown(&service, NotificationTab::Unread), [3, 2, 1]);
    assert_eq!(service.pending_change_count().unwrap(), 0);
}

/// The answer lists the notes unread afterwards. A note marked unread that isn't in it stayed
/// read, as the web panel reads the answer.
#[tokio::test]
async fn the_answer_decides_what_the_server_has() {
    let (service, executor) = service();
    three_unread(&service, &executor).await;
    executor.push(Step::Respond(200, taken(&[])));
    service.mark_read(note_ids(&[3])).await.unwrap();

    executor.push(Step::Respond(200, taken(&[])));
    let error = service.mark_unread(note_ids(&[3])).await.unwrap_err();
    assert!(
        matches!(error, NotificationServiceError::ChangeNotMade),
        "{error:?}"
    );
    assert_eq!(read_states(&service)[0], (3, Some(true)));
}

/// The server still has the note unread until the change lands, and a sync stores that. The note
/// still shows as read.
#[tokio::test]
async fn a_sync_before_the_change_is_sent_does_not_undo_it() {
    let (service, executor) = service();
    three_unread(&service, &executor).await;

    executor.push(Step::Offline);
    service.mark_read(note_ids(&[3])).await.unwrap();
    // Another client fetched the note, so its hash changed, and it's downloaded unread.
    sync_with(
        &service,
        &executor,
        checked(&[(3, 33), (2, 22), (1, 12)]),
        Some(whole_typed(&[(3, T + 3, 34, "like", false)])),
    )
    .await;
    assert_eq!(read_states(&service)[0], (3, Some(true)));
    assert_eq!(tab_shown(&service, NotificationTab::Unread), [2, 1]);
    // The Unread tab's own sync lists the note, as the server has it unread, and keeps it read.
    sync_tab_with(
        &service,
        &executor,
        NotificationTab::Unread,
        checked(&[(3, 34), (2, 22), (1, 12)]),
        Vec::new(),
    )
    .await;
    assert_eq!(tab_shown(&service, NotificationTab::Unread), [2, 1]);

    executor.push(Step::Respond(200, taken(&[])));
    assert_eq!(service.send_pending_changes().await.unwrap(), 0);
    assert_eq!(read_states(&service)[0], (3, Some(true)));
}

/// A sync that's running when the user marks a note holds the lane: the note shows as read at
/// once, the sync stores the server's unread copy, and the change is sent after the sync.
#[tokio::test]
async fn a_change_made_during_a_sync_shows_at_once_and_is_sent_after_it() {
    let (service, executor) = service();
    three_unread(&service, &executor).await;

    let (release, held) = futures::channel::oneshot::channel();
    executor.push(Step::Hold(held, 200, checked(&[(3, 33), (2, 22), (1, 12)])));
    executor.push(Step::Respond(
        200,
        whole_typed(&[(3, T + 3, 34, "like", false)]),
    ));
    executor.push(Step::Respond(200, taken(&[])));
    let (synced, marked, ()) =
        tokio::join!(service.sync(), service.mark_read(note_ids(&[3])), async {
            assert_eq!(tab_shown(&service, NotificationTab::Unread), [2, 1]);
            release.send(()).unwrap();
        });
    synced.unwrap();
    assert_eq!(marked.unwrap(), NotificationChangeOutcome::Sent);
    // The sync's two requests went before the change.
    let methods: Vec<RequestMethod> = executor
        .sent()
        .into_iter()
        .map(|sent| sent.method)
        .collect();
    assert_eq!(
        methods[methods.len() - 3..],
        [RequestMethod::GET, RequestMethod::GET, RequestMethod::POST]
    );
    assert_eq!(read_states(&service)[0], (3, Some(true)));
}

/// A change to a note made while its first change is being sent is sent after it, and the note
/// ends as the user last left it.
#[tokio::test]
async fn a_change_made_while_one_is_sent_is_sent_after_it() {
    let (service, executor) = service();
    three_unread(&service, &executor).await;

    let (release, held) = futures::channel::oneshot::channel();
    executor.push(Step::Hold(held, 200, taken(&[])));
    executor.push(Step::Respond(200, taken(&[3])));
    let (read, unread, ()) = tokio::join!(
        service.mark_read(note_ids(&[3])),
        service.mark_unread(note_ids(&[3])),
        async {
            assert_eq!(tab_shown(&service, NotificationTab::Unread), [3, 2, 1]);
            release.send(()).unwrap();
        }
    );
    assert_eq!(read.unwrap(), NotificationChangeOutcome::Sent);
    assert_eq!(unread.unwrap(), NotificationChangeOutcome::Sent);
    let bodies: Vec<Value> = posts(&executor).into_iter().map(|(_, body)| body).collect();
    assert_eq!(bodies, [counts(&[(3, 9999)]), counts(&[(3, -1)])]);
    assert_eq!(read_states(&service)[0], (3, Some(false)));
    assert_eq!(service.pending_change_count().unwrap(), 0);
}

/// The All tab marks every note read with one request, up to its newest cached note, and a note
/// that arrives after stays unread.
#[tokio::test]
async fn marking_every_note_read_sends_the_newest_time() {
    let (service, executor) = service();
    three_unread(&service, &executor).await;

    executor.push(Step::Offline);
    let outcome = service
        .mark_all_read(NotificationTab::All, None)
        .await
        .unwrap();
    assert_eq!(outcome, NotificationChangeOutcome::Waiting);
    assert!(tab_shown(&service, NotificationTab::Unread).is_empty());
    assert_eq!(service.pending_change_count().unwrap(), 1);

    // A newer note arrives before it's sent.
    sync_with(
        &service,
        &executor,
        checked(&[(4, 41), (3, 32), (2, 22)]),
        Some(whole_typed(&[(4, T + 4, 42, "like", false)])),
    )
    .await;
    assert_eq!(tab_shown(&service, NotificationTab::Unread), [4]);

    executor.push(Step::Respond(
        200,
        json!({ "success": true, "count": 3, "last_seen_time": T.to_string() }),
    ));
    assert_eq!(service.send_pending_changes().await.unwrap(), 0);
    let posts = posts(&executor);
    assert_eq!(posts[0].0, "/notifications/read/all?locale=en");
    assert_eq!(posts[1].1, json!({ "time": T + 3, "source": "mobile" }));
    assert_eq!(tab_shown(&service, NotificationTab::Unread), [4]);
}

#[tokio::test]
async fn marking_every_note_read_up_to_a_time_leaves_newer_ones() {
    let (service, executor) = service();
    three_unread(&service, &executor).await;
    executor.push(Step::Respond(200, json!({ "success": true, "count": 2 })));
    let outcome = service
        .mark_all_read(NotificationTab::Unread, date(T + 2))
        .await
        .unwrap();
    assert_eq!(outcome, NotificationChangeOutcome::Sent);
    assert_eq!(
        posts(&executor)[0].1,
        json!({ "time": T + 2, "source": "mobile" })
    );
    assert_eq!(tab_shown(&service, NotificationTab::Unread), [3]);
}

/// Another tab marks its own cached unread notes, by ID: marking every note read can't be
/// narrowed to a type.
#[tokio::test]
async fn marking_a_tab_read_marks_its_cached_unread_notes() {
    let (service, executor) = service();
    three_unread(&service, &executor).await;
    executor.push(Step::Respond(200, taken(&[])));
    let outcome = service
        .mark_all_read(NotificationTab::Likes, None)
        .await
        .unwrap();
    assert_eq!(outcome, NotificationChangeOutcome::Sent);
    let posts = posts(&executor);
    assert_eq!(posts[0].0, "/notifications/read?locale=en");
    assert_eq!(posts[0].1, counts(&[(3, 9999), (1, 9999)]));
    assert_eq!(tab_shown(&service, NotificationTab::Unread), [2]);
    assert_eq!(
        service
            .mark_all_read(NotificationTab::Likes, None)
            .await
            .unwrap(),
        NotificationChangeOutcome::Unchanged
    );
}

#[tokio::test]
async fn nothing_cached_is_nothing_to_mark() {
    let (service, executor) = service();
    assert_eq!(
        service
            .mark_all_read(NotificationTab::All, None)
            .await
            .unwrap(),
        NotificationChangeOutcome::Unchanged
    );
    assert_eq!(
        service.mark_seen(None).await.unwrap(),
        NotificationChangeOutcome::Unchanged
    );
    assert!(!service.has_unseen_notes().unwrap());
    assert!(executor.sent().is_empty());
}

/// Marking seen clears the unseen flag at once, and the time only moves forward.
#[tokio::test]
async fn marking_seen_clears_the_flag_and_only_moves_forward() {
    let (service, executor) = service();
    // The notes are all newer than the time the user last looked, `T`.
    three_unread(&service, &executor).await;
    assert!(service.has_unseen_notes().unwrap());

    executor.push(Step::Respond(
        200,
        json!({ "success": true, "last_seen_time": (T + 3).to_string() }),
    ));
    let outcome = service.mark_seen(None).await.unwrap();
    assert_eq!(outcome, NotificationChangeOutcome::Sent);
    assert_eq!(
        posts(&executor),
        [(
            "/notifications/seen?locale=en".to_string(),
            json!({ "time": T + 3, "source": "mobile" })
        )]
    );
    assert!(!service.has_unseen_notes().unwrap());
    assert_eq!(service.info().unwrap().last_seen_time, date(T + 3));

    // An older time, or the same one, sends nothing.
    for time in [T + 2, T + 3] {
        assert_eq!(
            service.mark_seen(date(time)).await.unwrap(),
            NotificationChangeOutcome::Unchanged
        );
    }
    assert_eq!(posts(&executor).len(), 1);

    // A sync that brings the server's older time doesn't move it back.
    sync_with(
        &service,
        &executor,
        checked(&[(3, 32), (2, 22), (1, 12)]),
        None,
    )
    .await;
    assert_eq!(service.info().unwrap().last_seen_time, date(T + 3));
    assert!(!service.has_unseen_notes().unwrap());
}

/// Offline, the flag clears at once and the time waits. A sync that brings a newer time from
/// another device makes it stale, and it's dropped without being sent.
#[tokio::test]
async fn a_seen_time_made_offline_clears_the_flag_and_is_never_sent_backwards() {
    let (service, executor) = service();
    three_unread(&service, &executor).await;
    executor.push(Step::Offline);
    let outcome = service.mark_seen(date(T + 2)).await.unwrap();
    assert_eq!(outcome, NotificationChangeOutcome::Waiting);
    assert_eq!(service.pending_change_count().unwrap(), 1);
    assert!(
        service.has_unseen_notes().unwrap(),
        "note 3 is newer than T + 2"
    );

    let mut check = checked(&[(3, 32), (2, 22), (1, 12)]);
    check["last_seen_time"] = json!((T + 5).to_string());
    sync_with(&service, &executor, check, None).await;
    let posts_before = posts(&executor).len();
    assert_eq!(service.send_pending_changes().await.unwrap(), 0);
    assert_eq!(posts(&executor).len(), posts_before);
    assert!(!service.has_unseen_notes().unwrap());
}

#[tokio::test]
async fn a_refused_seen_time_is_dropped() {
    let (service, executor) = service();
    three_unread(&service, &executor).await;
    executor.push(Step::Respond(200, json!({ "success": false })));
    let error = service.mark_seen(None).await.unwrap_err();
    assert!(
        matches!(error, NotificationServiceError::ChangeNotMade),
        "{error:?}"
    );
    assert_eq!(service.pending_change_count().unwrap(), 0);
    assert!(service.has_unseen_notes().unwrap());
}

/// The server's flag is asked for with one small GET, unless a seen time still waits.
#[tokio::test]
async fn the_servers_unseen_flag() {
    let (service, executor) = service();
    three_unread(&service, &executor).await;
    executor.push(Step::Respond(200, json!({ "has_unseen_notes": true })));
    assert!(service.fetch_has_unseen_notes().await.unwrap());
    let last = executor.sent().last().cloned().unwrap();
    assert_eq!(last.method, RequestMethod::GET);
    assert!(
        last.url.ends_with("/rest/v1.1/me?fields=has_unseen_notes"),
        "{}",
        last.url
    );

    executor.push(Step::Offline);
    service.mark_seen(None).await.unwrap();
    // The seen time is tried again first, and still can't be sent, so the server isn't asked.
    executor.push(Step::Offline);
    let before = executor.sent().len();
    assert!(!service.fetch_has_unseen_notes().await.unwrap());
    assert_eq!(executor.sent().len(), before + 1);
}

/// Every change waits in order: marking every note read, the notes, then the seen time.
#[tokio::test]
async fn waiting_changes_are_sent_in_order() {
    let (service, executor) = service();
    three_unread(&service, &executor).await;
    executor.push(Step::Offline);
    service.mark_read(note_ids(&[3])).await.unwrap();
    executor.push(Step::Offline);
    service.mark_seen(None).await.unwrap();
    executor.push(Step::Offline);
    service
        .mark_all_read(NotificationTab::All, date(T + 2))
        .await
        .unwrap();
    assert_eq!(service.pending_change_count().unwrap(), 3);

    executor.push(Step::Respond(200, json!({ "success": true, "count": 2 })));
    executor.push(Step::Respond(200, taken(&[])));
    executor.push(Step::Respond(200, json!({ "success": true })));
    assert_eq!(service.send_pending_changes().await.unwrap(), 0);
    let paths: Vec<String> = posts(&executor)
        .into_iter()
        .skip(3)
        .map(|(path, _)| path)
        .collect();
    assert_eq!(
        paths,
        [
            "/notifications/read/all?locale=en",
            "/notifications/read?locale=en",
            "/notifications/seen?locale=en"
        ]
    );
    assert!(tab_shown(&service, NotificationTab::Unread).is_empty());
}

#[tokio::test]
async fn each_account_keeps_its_own_changes() {
    let cache = cache();
    let (first, first_executor) = service_in(cache.clone(), USER, 3);
    let (second, second_executor) = service_in(cache, WpComUserId(2), 3);
    three_unread(&first, &first_executor).await;
    three_unread(&second, &second_executor).await;
    first_executor.push(Step::Offline);
    first.mark_read(note_ids(&[3])).await.unwrap();
    assert_eq!(first.pending_change_count().unwrap(), 1);
    assert_eq!(second.pending_change_count().unwrap(), 0);
    assert_eq!(tab_shown(&second, NotificationTab::Unread), [3, 2, 1]);
}

/// Liking and following from a note. Sites 7001 to 7003, post 42 and comment 55 are invented.
mod note_actions {
    use super::*;
    use crate::service::reader::{
        ReaderActionOutcome, ReaderFollowTarget, ReaderPageViewChoice, ReaderPostLikeResult,
        ReaderService,
    };
    use futures::channel::oneshot;
    use wp_api::wp_com::WpComSiteId;
    use wp_api::wp_com::notifications::{
        NotificationActionKind, NotificationLayoutContent, NotificationNewPostByline,
        notification_layout,
    };
    use wp_api::wp_com::reader_saved::{ReaderPostSaveIdentity, ReaderSavedSource};
    use wp_mobile_cache::repository::wordpress_com_accounts::ReaderOwner;

    const SITE: u64 = 7001;
    const POST: u64 = 42;
    const COMMENT: u64 = 55;
    /// The site of the person behind the notes.
    const AUTHOR_SITE: u64 = 7002;

    /// A person's block, offering to follow their site.
    fn person(site: u64, following: bool) -> Value {
        json!({
            "type": "user",
            "text": "A person",
            "meta": {
                "ids": { "site": site, "user": 9 },
                "links": { "home": "https://author.example" },
                "titles": { "home": "An author's site" },
            },
            "actions": { "follow": following },
        })
    }

    /// Note `n`, a new post on site 7001, whose post the user can like, by a person whose site
    /// the user can follow.
    fn new_post(n: u64, liked: bool, following: bool) -> Value {
        json!({
            "id": id(n),
            "type": "new_post",
            "read": 0,
            "timestamp": rfc3339(T + n as i64),
            "note_hash": 100 + n,
            "subject": [{ "text": "A new post" }],
            "meta": {
                "ids": { "site": SITE, "post": POST },
                "actor": { "name": "A person" },
                "actor_count": 1,
                "post": { "title": "A post", "url": "https://site.example/a-post" },
                "site": { "name": "A site", "url": "https://site.example" },
            },
            "body": [
                person(AUTHOR_SITE, following),
                {
                    "type": "post",
                    "text": "The post",
                    "meta": { "ids": { "site": SITE, "post": POST } },
                    "actions": { "like-post": liked },
                },
            ],
        })
    }

    /// Note `n`, a comment on post 42 whose comment the user can like.
    fn comment(n: u64, liked: bool) -> Value {
        json!({
            "id": id(n),
            "type": "comment",
            "read": 0,
            "timestamp": rfc3339(T + n as i64),
            "note_hash": 100 + n,
            "subject": [{ "text": "A comment" }],
            "meta": { "ids": { "site": SITE, "post": POST, "comment": COMMENT } },
            "body": [
                person(7003, false),
                {
                    "type": "comment",
                    "text": "The comment",
                    "meta": { "ids": { "site": SITE, "post": POST, "comment": COMMENT } },
                    "actions": { "like-comment": liked, "replyto-comment": true },
                },
            ],
        })
    }

    /// A service that checks ten notes at a time, and its executor. No Reader service exists.
    fn services() -> (NotificationService, Arc<ScriptedExecutor>) {
        service_in(cache(), USER, 10)
    }

    /// The Reader service of `user_id` in `cache`, sending through `executor`.
    fn reader_in(
        executor: &Arc<ScriptedExecutor>,
        cache: Arc<WpApiCache>,
        user_id: WpComUserId,
    ) -> ReaderService {
        ReaderService::new(
            delegate(executor.clone()),
            cache,
            ReaderOwner::Account { user_id },
            ReaderSavedSource::Ios,
        )
        .unwrap()
    }

    /// A service that checks ten notes at a time, the account's Reader service on the same
    /// cache, and their executor.
    fn services_with_reader() -> (NotificationService, ReaderService, Arc<ScriptedExecutor>) {
        let cache = cache();
        let (service, executor) = service_in(cache.clone(), USER, 10);
        let reader = reader_in(&executor, cache, USER);
        (service, reader, executor)
    }

    /// Syncs `notes` into the cache, each listed with its own hash.
    async fn store(service: &NotificationService, executor: &ScriptedExecutor, notes: &[Value]) {
        let listed: Vec<Value> = notes
            .iter()
            .map(|note| json!({ "id": note["id"], "note_hash": note["note_hash"] }))
            .collect();
        executor.push(Step::Respond(
            200,
            json!({ "last_seen_time": T.to_string(), "notes": listed }),
        ));
        executor.push(Step::Respond(
            200,
            json!({ "last_seen_time": T.to_string(), "notes": notes }),
        ));
        service.sync().await.unwrap();
    }

    /// Whether cached note `n` shows its first `kind` action on.
    fn shown_action(
        service: &NotificationService,
        n: u64,
        kind: NotificationActionKind,
    ) -> Option<bool> {
        let notes = service.load_notes(NotificationsFilter::default()).unwrap();
        let note = notes.iter().find(|note| note.id.0 == id(n))?;
        note.body
            .iter()
            .flat_map(|block| block.actions.iter())
            .find(|action| action.kind == kind)
            .and_then(|action| action.is_on)
    }

    fn liked(service: &NotificationService, n: u64) -> Option<bool> {
        shown_action(service, n, NotificationActionKind::LikePost)
    }

    /// The request sent last: its method, path and body.
    fn last_sent(executor: &ScriptedExecutor) -> (RequestMethod, String, Option<Value>) {
        let sent = executor.sent().last().cloned().unwrap();
        let url = url::Url::parse(&sent.url).unwrap();
        (sent.method, url.path().to_string(), sent.body)
    }

    fn like_answer(liked: bool) -> Step {
        Step::Respond(
            200,
            json!({ "success": true, "i_like": liked, "like_count": u64::from(liked) }),
        )
    }

    #[tokio::test]
    async fn a_like_from_a_note_shows_at_once_on_every_note_about_the_post_and_stays_when_taken() {
        let (service, executor) = services();
        store(
            &service,
            &executor,
            &[new_post(1, false, false), new_post(2, false, false)],
        )
        .await;
        let (release, held) = oneshot::channel();
        executor.push(Step::Hold(
            held,
            200,
            json!({ "success": true, "i_like": true, "like_count": 1 }),
        ));

        let (outcome, ()) =
            futures::join!(service.set_post_liked(NotificationId(id(1)), true), async {
                assert_eq!(liked(&service, 1), Some(true));
                assert_eq!(
                    liked(&service, 2),
                    Some(true),
                    "Another note about the post"
                );
                release.send(()).unwrap();
            });

        assert_eq!(outcome.unwrap(), ReaderActionOutcome::Synced);
        assert_eq!(
            last_sent(&executor),
            (
                RequestMethod::POST,
                format!("/rest/v1.1/sites/{SITE}/posts/{POST}/likes/new"),
                None
            )
        );
        // Nothing waits, so the stored notes have the like.
        assert_eq!(service.pending_action_count().unwrap(), 0);
        assert_eq!(liked(&service, 1), Some(true));
        assert_eq!(liked(&service, 2), Some(true));
    }

    #[tokio::test]
    async fn an_unlike_from_a_note_unlikes_its_comment() {
        let (service, executor) = services();
        store(&service, &executor, &[comment(1, true)]).await;
        executor.push(like_answer(false));

        let outcome = service
            .set_comment_liked(NotificationId(id(1)), false)
            .await
            .unwrap();

        assert_eq!(outcome, ReaderActionOutcome::Synced);
        assert_eq!(
            shown_action(&service, 1, NotificationActionKind::LikeComment),
            Some(false)
        );
        assert_eq!(
            last_sent(&executor).1,
            format!("/rest/v1.1/sites/{SITE}/comments/{COMMENT}/likes/mine/delete")
        );
    }

    #[tokio::test]
    async fn a_follow_from_a_note_follows_the_blocks_site_and_its_layout_shows_it_at_once() {
        let (service, executor) = services();
        store(&service, &executor, &[new_post(1, false, false)]).await;
        let following = |service: &NotificationService| {
            let note = service.load_notes(NotificationsFilter::default()).unwrap()[0].clone();
            let NotificationLayoutContent::NewPost { layout } = notification_layout(&note).content
            else {
                panic!("Expected the new post layout");
            };
            let Some(NotificationNewPostByline::Site { meta_line, .. }) = layout.byline else {
                panic!("Expected a site byline");
            };
            meta_line.follow.unwrap()
        };
        assert_eq!(following(&service).site_id, WpComSiteId(AUTHOR_SITE));
        assert!(!following(&service).is_following);
        let (release, held) = oneshot::channel();
        executor.push(Step::Hold(held, 200, json!({ "subscribed": true })));

        let (outcome, ()) = futures::join!(
            service.set_site_followed(NotificationId(id(1)), WpComSiteId(AUTHOR_SITE), true),
            async {
                assert!(following(&service).is_following);
                release.send(()).unwrap();
            }
        );

        assert_eq!(outcome.unwrap(), ReaderActionOutcome::Synced);
        assert!(following(&service).is_following);
        // A follow goes by the block's site address, as the Reader follows, with no source, as
        // the web notifications panel sends it.
        assert_eq!(
            last_sent(&executor),
            (
                RequestMethod::POST,
                "/rest/v1.1/read/following/mine/new".to_string(),
                Some(json!({ "url": "https://author.example" }))
            )
        );

        // An unfollow goes by the site's ID.
        executor.push(Step::Respond(
            200,
            json!({ "success": true, "is_following": false }),
        ));
        service
            .set_site_followed(NotificationId(id(1)), WpComSiteId(AUTHOR_SITE), false)
            .await
            .unwrap();
        assert!(!following(&service).is_following);
        assert_eq!(
            last_sent(&executor).1,
            format!("/rest/v1.1/sites/{AUTHOR_SITE}/follows/mine/delete")
        );
    }

    #[tokio::test]
    async fn a_like_made_offline_shows_at_once_and_the_services_own_send_sends_it() {
        let (service, executor) = services();
        store(&service, &executor, &[new_post(1, false, false)]).await;
        executor.push(Step::Offline);

        let outcome = service
            .set_post_liked(NotificationId(id(1)), true)
            .await
            .unwrap();

        assert!(
            matches!(outcome, ReaderActionOutcome::Pending { .. }),
            "{outcome:?}"
        );
        assert_eq!(liked(&service, 1), Some(true));
        assert_eq!(service.pending_action_count().unwrap(), 1);
        // A read change waiting too is counted apart, and both are sent.
        executor.push(Step::Offline);
        service.mark_read(note_ids(&[1])).await.unwrap();
        assert_eq!(service.pending_change_count().unwrap(), 1);
        assert_eq!(service.pending_action_count().unwrap(), 1);

        executor.push(Step::Respond(200, taken(&[])));
        executor.push(like_answer(true));
        assert_eq!(service.send_pending_changes().await.unwrap(), 0);
        assert_eq!(service.pending_action_count().unwrap(), 0);
        assert_eq!(liked(&service, 1), Some(true));
        assert_eq!(
            last_sent(&executor).1,
            format!("/rest/v1.1/sites/{SITE}/posts/{POST}/likes/new")
        );
    }

    #[tokio::test]
    async fn a_refused_like_shows_the_servers_state_and_fails_with_a_message_for_the_user() {
        let (service, executor) = services();
        store(&service, &executor, &[new_post(1, false, false)]).await;
        executor.push(Step::Respond(
            403,
            json!({ "error": "unauthorized", "message": "You can't like this post." }),
        ));

        let error = service
            .set_post_liked(NotificationId(id(1)), true)
            .await
            .unwrap_err();

        assert!(
            matches!(
                &error,
                NotificationServiceError::ActionRefused {
                    change: NotificationNoteChange::LikePost,
                    error: ReaderError::Api(_),
                }
            ),
            "{error:?}"
        );
        assert_eq!(liked(&service, 1), Some(false));
        assert_eq!(service.pending_action_count().unwrap(), 0);
        assert_eq!(
            notification_service_error_user_message(&error, None).as_deref(),
            Some("Your like was not added to this post.")
        );
    }

    #[tokio::test]
    async fn a_follow_the_server_answers_without_making_shows_the_site_unfollowed_again() {
        let (service, executor) = services();
        store(&service, &executor, &[new_post(1, false, false)]).await;
        // The API answers 200 for an address it can't follow.
        executor.push(Step::Respond(
            200,
            json!({ "subscribed": false, "info": "invalid_feed" }),
        ));

        let error = service
            .set_site_followed(NotificationId(id(1)), WpComSiteId(AUTHOR_SITE), true)
            .await
            .unwrap_err();

        assert!(
            matches!(
                &error,
                NotificationServiceError::ActionRefused {
                    change: NotificationNoteChange::FollowSite,
                    error: ReaderError::Refused { .. },
                }
            ),
            "{error:?}"
        );
        assert_eq!(
            shown_action(&service, 1, NotificationActionKind::Follow),
            Some(false)
        );
        assert_eq!(service.pending_action_count().unwrap(), 0);
        assert_eq!(
            notification_service_error_user_message(&error, None).as_deref(),
            Some("You are not subscribed to this site.")
        );
    }

    #[test]
    fn only_a_refusal_has_a_message_for_the_user() {
        let messages: Vec<Option<String>> = [
            NotificationNoteChange::UnlikePost,
            NotificationNoteChange::LikeComment,
            NotificationNoteChange::UnlikeComment,
            NotificationNoteChange::FollowSite,
            NotificationNoteChange::UnfollowSite,
        ]
        .into_iter()
        .map(|change| {
            notification_service_error_user_message(
                &NotificationServiceError::ActionRefused {
                    change,
                    error: ReaderError::MissingIds,
                },
                None,
            )
        })
        .collect();
        assert_eq!(
            messages,
            [
                Some("Your like is still on this post.".to_string()),
                Some("Your like was not added to this comment.".to_string()),
                Some("Your like is still on this comment.".to_string()),
                Some("You are not subscribed to this site.".to_string()),
                Some("You are still subscribed to this site.".to_string()),
            ]
        );
        for developer_only in [
            NotificationServiceError::NoteNotCached {
                note_id: NotificationId(id(1)),
            },
            NotificationServiceError::ActionNotOffered {
                change: NotificationNoteChange::LikePost,
            },
            NotificationServiceError::CannotPageFurther,
        ] {
            assert_eq!(
                notification_service_error_user_message(&developer_only, None),
                None
            );
        }
    }

    #[tokio::test]
    async fn a_sync_before_the_like_is_sent_does_not_undo_it_and_the_sync_after_agrees() {
        let (service, executor) = services();
        store(&service, &executor, &[new_post(1, false, false)]).await;
        executor.push(Step::Offline);
        service
            .set_post_liked(NotificationId(id(1)), true)
            .await
            .unwrap();

        // The note changed on the server, and still isn't liked there.
        let mut unliked = new_post(1, false, false);
        unliked["note_hash"] = json!(201);
        store(&service, &executor, &[unliked]).await;
        assert_eq!(liked(&service, 1), Some(true));

        executor.push(like_answer(true));
        assert_eq!(service.send_pending_changes().await.unwrap(), 0);
        assert_eq!(liked(&service, 1), Some(true));

        // The server's copy now has the like.
        let mut taken = new_post(1, true, false);
        taken["note_hash"] = json!(301);
        store(&service, &executor, &[taken]).await;
        assert_eq!(liked(&service, 1), Some(true));
        assert_eq!(service.pending_action_count().unwrap(), 0);
    }

    /// A whole note fetched while a like is on its way could be the copy from before the like,
    /// stored over the like the server took. So the download waits for the like's answer.
    #[tokio::test]
    async fn a_sync_downloads_no_note_while_a_like_is_being_sent() {
        let (service, executor) = services();
        store(&service, &executor, &[new_post(1, false, false)]).await;
        let sent = executor.sent().len();
        let (release, held) = oneshot::channel();
        executor.push(Step::Hold(
            held,
            200,
            json!({ "success": true, "i_like": true, "like_count": 1 }),
        ));
        let mut changed = new_post(1, true, false);
        changed["note_hash"] = json!(201);
        executor.push(Step::Respond(
            200,
            json!({ "last_seen_time": T.to_string(), "notes": [{ "id": id(1), "note_hash": 201 }] }),
        ));
        executor.push(Step::Respond(
            200,
            json!({ "last_seen_time": T.to_string(), "notes": [changed] }),
        ));

        let (like, sync, ()) = futures::join!(
            service.set_post_liked(NotificationId(id(1)), true),
            service.sync(),
            async {
                // The like and the sync's list of hashes are sent; the download waits.
                while executor.sent().len() < sent + 2 {
                    tokio::task::yield_now().await;
                }
                for _ in 0..10 {
                    tokio::task::yield_now().await;
                }
                assert_eq!(executor.sent().len(), sent + 2);
                release.send(()).unwrap();
            }
        );

        assert_eq!(like.unwrap(), ReaderActionOutcome::Synced);
        assert_eq!(sync.unwrap().downloaded_count, 1);
        assert_eq!(executor.sent().len(), sent + 3);
        assert_eq!(liked(&service, 1), Some(true));
        assert_eq!(service.pending_action_count().unwrap(), 0);
    }

    #[tokio::test]
    async fn a_like_made_in_the_reader_shows_on_the_note_and_is_the_same_like() {
        let (service, reader, executor) = services_with_reader();
        store(&service, &executor, &[new_post(1, false, false)]).await;
        let post = ReaderPostSaveIdentity {
            id: POST,
            site_id: Some(WpComSiteId(SITE)),
            feed_id: Some(wp_api::wp_com::reader_saved::ReaderFeedId(5)),
            feed_item_id: None,
            is_external: false,
            url: None,
        };
        executor.push(Step::Offline);

        let outcome = reader
            .set_post_liked(post, true, ReaderPageViewChoice::DontCount)
            .await
            .unwrap();

        assert!(matches!(
            outcome.like,
            ReaderPostLikeResult::Done {
                outcome: ReaderActionOutcome::Pending { .. }
            }
        ));
        assert_eq!(liked(&service, 1), Some(true));

        // Unliking it from the note replaces the Reader's like: one row, one request.
        executor.push(Step::Offline);
        service
            .set_post_liked(NotificationId(id(1)), false)
            .await
            .unwrap();
        assert_eq!(liked(&service, 1), Some(false));
        assert_eq!(service.pending_action_count().unwrap(), 1);
        assert_eq!(reader.pending_action_count().unwrap(), 1);
        executor.push(like_answer(false));
        assert_eq!(reader.sync_actions().await.unwrap(), 0);
        assert_eq!(
            last_sent(&executor).1,
            format!("/rest/v1.1/sites/{SITE}/posts/{POST}/likes/mine/delete")
        );
        assert_eq!(liked(&service, 1), Some(false));
    }

    #[tokio::test]
    async fn a_note_without_the_action_or_not_in_the_cache_sends_nothing() {
        let (service, executor) = services();
        store(
            &service,
            &executor,
            &[new_post(1, false, false), note_json(2, T + 2, 12)],
        )
        .await;
        let sent = executor.sent().len();

        let errors = [
            service.set_post_liked(NotificationId(id(2)), true).await,
            service.set_comment_liked(NotificationId(id(1)), true).await,
            service
                .set_site_followed(NotificationId(id(1)), WpComSiteId(SITE), true)
                .await,
            service.set_post_liked(NotificationId(id(9)), false).await,
        ]
        .map(Result::unwrap_err);

        assert!(matches!(
            errors[0],
            NotificationServiceError::ActionNotOffered {
                change: NotificationNoteChange::LikePost
            }
        ));
        assert!(matches!(
            errors[1],
            NotificationServiceError::ActionNotOffered {
                change: NotificationNoteChange::LikeComment
            }
        ));
        assert!(matches!(
            errors[2],
            NotificationServiceError::ActionNotOffered {
                change: NotificationNoteChange::FollowSite
            }
        ));
        assert!(matches!(
            errors[3],
            NotificationServiceError::NoteNotCached { note_id } if note_id == NotificationId(id(9))
        ));
        assert_eq!(executor.sent().len(), sent);
        assert_eq!(service.pending_action_count().unwrap(), 0);
    }

    /// The paths of the requests sent after the first `skip`, with their bodies.
    fn sent_after(executor: &ScriptedExecutor, skip: usize) -> Vec<(String, Option<Value>)> {
        executor
            .sent()
            .iter()
            .skip(skip)
            .map(|sent| {
                let url = url::Url::parse(&sent.url).unwrap();
                (url.path().to_string(), sent.body.clone())
            })
            .collect()
    }

    #[tokio::test]
    async fn unliking_a_post_and_liking_a_comment_need_no_reader_service() {
        let (service, executor) = services();
        store(
            &service,
            &executor,
            &[new_post(1, true, false), comment(2, false)],
        )
        .await;
        let sent = executor.sent().len();

        executor.push(like_answer(false));
        let unliked = service.set_post_liked(NotificationId(id(1)), false);
        assert_eq!(unliked.await.unwrap(), ReaderActionOutcome::Synced);
        executor.push(like_answer(true));
        let liked_comment = service.set_comment_liked(NotificationId(id(2)), true);
        assert_eq!(liked_comment.await.unwrap(), ReaderActionOutcome::Synced);

        assert_eq!(
            sent_after(&executor, sent),
            [
                (
                    format!("/rest/v1.1/sites/{SITE}/posts/{POST}/likes/mine/delete"),
                    None
                ),
                (
                    format!("/rest/v1.1/sites/{SITE}/comments/{COMMENT}/likes/new"),
                    None
                ),
            ]
        );
        // Nothing waits, so what shows is what the stored notes now have.
        assert_eq!(service.pending_action_count().unwrap(), 0);
        assert_eq!(liked(&service, 1), Some(false));
        assert_eq!(
            shown_action(&service, 2, NotificationActionKind::LikeComment),
            Some(true)
        );
    }

    #[test]
    fn an_account_has_one_queue_on_one_cache_whichever_services_exist() {
        let cache = cache();
        let (service, executor) = service_in(cache.clone(), USER, 10);
        let reader = reader_in(&executor, cache.clone(), USER);
        let second_reader = reader_in(&executor, cache.clone(), USER);
        let (second_service, _) = service_in(cache.clone(), USER, 10);
        for queue in [
            &reader.actions().queue,
            &second_reader.actions().queue,
            &second_service.actions.queue,
        ] {
            assert!(Arc::ptr_eq(&service.actions.queue, queue));
        }

        let (other_account, _) = service_in(cache.clone(), WpComUserId(2), 10);
        let (other_cache, _) = service_in(super::cache(), USER, 10);
        assert!(!Arc::ptr_eq(
            &service.actions.queue,
            &other_account.actions.queue
        ));
        assert!(!Arc::ptr_eq(
            &service.actions.queue,
            &other_cache.actions.queue
        ));

        // Signing out drops the account's queue, so whoever signs in next starts with a new one.
        cache.remove_wordpress_com_account(USER).unwrap();
        let (signed_in_again, _) = service_in(cache, USER, 10);
        assert!(!Arc::ptr_eq(
            &service.actions.queue,
            &signed_in_again.actions.queue
        ));
    }

    #[tokio::test]
    async fn a_like_made_offline_in_either_service_is_sent_once_by_the_other() {
        let (service, reader, executor) = services_with_reader();
        store(
            &service,
            &executor,
            &[new_post(1, false, false), comment(2, false)],
        )
        .await;

        // Made from a note, sent by the Reader.
        executor.push(Step::Offline);
        service
            .set_comment_liked(NotificationId(id(2)), true)
            .await
            .unwrap();
        let sent = executor.sent().len();
        executor.push(like_answer(true));
        assert_eq!(reader.sync_actions().await.unwrap(), 0);
        assert_eq!(
            sent_after(&executor, sent),
            [(
                format!("/rest/v1.1/sites/{SITE}/comments/{COMMENT}/likes/new"),
                None
            )]
        );

        // Made in the Reader, sent by the notifications service.
        let post = ReaderPostSaveIdentity {
            id: POST,
            site_id: Some(WpComSiteId(SITE)),
            feed_id: None,
            feed_item_id: None,
            is_external: false,
            url: None,
        };
        executor.push(Step::Offline);
        reader
            .set_post_liked(post, true, ReaderPageViewChoice::DontCount)
            .await
            .unwrap();
        assert_eq!(service.pending_action_count().unwrap(), 1);
        let sent = executor.sent().len();
        executor.push(like_answer(true));
        assert_eq!(service.send_pending_changes().await.unwrap(), 0);
        assert_eq!(
            sent_after(&executor, sent),
            [(
                format!("/rest/v1.1/sites/{SITE}/posts/{POST}/likes/new"),
                None
            )]
        );
        assert_eq!(liked(&service, 1), Some(true));
        assert_eq!(reader.pending_action_count().unwrap(), 0);
    }

    /// Each row carries its own source, so the service that sends it doesn't change what's sent.
    #[tokio::test]
    async fn the_readers_rows_keep_the_readers_source_when_the_notifications_service_sends_them() {
        let (service, reader, executor) = services_with_reader();
        let post = ReaderPostSaveIdentity {
            id: POST,
            site_id: Some(WpComSiteId(SITE)),
            feed_id: None,
            feed_item_id: None,
            is_external: false,
            url: None,
        };
        executor.push(Step::Offline);
        reader.set_post_seen(post, true).await.unwrap();
        // The seen mark waits first, and gets no answer again.
        executor.push(Step::Offline);
        reader
            .set_followed(
                ReaderFollowTarget {
                    site_id: Some(WpComSiteId(7004)),
                    feed_id: None,
                    url: Some("https://another.example".to_string()),
                    name: None,
                },
                true,
            )
            .await
            .unwrap();
        assert_eq!(service.pending_action_count().unwrap(), 2);
        let sent = executor.sent().len();

        executor.push(Step::Respond(200, json!({ "status": true })));
        executor.push(Step::Respond(200, json!({ "subscribed": true })));
        assert_eq!(service.send_pending_changes().await.unwrap(), 0);

        assert_eq!(
            sent_after(&executor, sent),
            [
                (
                    "/wpcom/v2/seen-posts/seen/blog/new".to_string(),
                    Some(json!({ "blog_id": SITE, "post_ids": [POST], "source": "reader-ios" }))
                ),
                (
                    "/rest/v1.1/read/following/mine/new".to_string(),
                    Some(json!({ "url": "https://another.example", "source": "reader-ios" }))
                ),
            ]
        );
    }

    /// A seen mark stored without its source can't be sent as it was made, so it's dropped. A
    /// follow stored without one is sent with none.
    #[tokio::test]
    async fn a_row_stored_without_a_source() {
        use wp_mobile_cache::repository::reader_actions::ReaderActionRepository;
        let (service, executor) = services();
        let post = ReaderPostSaveIdentity {
            id: POST,
            site_id: Some(WpComSiteId(SITE)),
            feed_id: None,
            feed_item_id: None,
            is_external: false,
            url: None,
        };
        let seen = json!({ "action": "seen", "post": post });
        let follow = json!({
            "action": "follow",
            "site_id": 7004,
            "feed_id": null,
            "url": "https://another.example",
            "name": null,
        });
        service.cache.execute(|connection| {
            ReaderActionRepository::tap(
                &*connection,
                service.account,
                1,
                "a seen mark",
                true,
                None,
                &seen.to_string(),
            )
            .unwrap();
            ReaderActionRepository::tap(
                &*connection,
                service.account,
                4,
                "site:7004",
                true,
                None,
                &follow.to_string(),
            )
            .unwrap();
        });
        executor.push(Step::Respond(200, json!({ "subscribed": true })));

        assert_eq!(service.send_pending_changes().await.unwrap(), 0);
        assert_eq!(
            sent_after(&executor, 0),
            [(
                "/rest/v1.1/read/following/mine/new".to_string(),
                Some(json!({ "url": "https://another.example" }))
            )]
        );
    }
}
