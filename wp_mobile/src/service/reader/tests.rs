use super::*;
use crate::testing::scripted::{ScriptedExecutor, Step, delegate};
use futures::channel::oneshot;
use serde_json::{Value, json};
use std::time::Duration;
use wp_api::posts::PostId;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::reader_posts::ReaderTagSlug;
use wp_api::wp_com::reader_saved::{ReaderFeedId, ReaderSavedKey, ReaderSavedSource};
use wp_mobile_cache::list_metadata::ListState;
use wp_mobile_cache::repository::reader::ReaderUnreadablePostRepository;

const ACCOUNT: ReaderOwner = ReaderOwner::Account {
    user_id: WpComUserId(1),
};

fn cache() -> Arc<WpApiCache> {
    let cache = WpApiCache::new(None).expect("an in-memory cache opens");
    cache.perform_migrations().expect("migrations run");
    Arc::new(cache)
}

fn service_for(owner: ReaderOwner) -> (ReaderService, Arc<ScriptedExecutor>) {
    let executor = Arc::new(ScriptedExecutor::default());
    let service = ReaderService::new(
        delegate(executor.clone()),
        cache(),
        owner,
        ReaderSavedSource::Ios,
    )
    .unwrap();
    (service, executor)
}

fn service() -> (ReaderService, Arc<ScriptedExecutor>) {
    service_for(ACCOUNT)
}

/// A site post dated at an hour of one day, so a larger hour is a newer post.
fn post_json(id: u64, hour: u32) -> Value {
    json!({
        "ID": id,
        "site_ID": 123,
        "is_external": false,
        "title": format!("Post {id}"),
        "content": format!("<p>The content of post {id}.</p>"),
        "date": date(hour),
    })
}

fn date(hour: u32) -> String {
    format!("2026-11-18T{hour:02}:00:00+00:00")
}

fn page(posts: &[(u64, u32)]) -> Value {
    let posts: Vec<Value> = posts
        .iter()
        .map(|(id, hour)| post_json(*id, *hour))
        .collect();
    json!({ "posts": posts })
}

fn tag_page(posts: &[u64], next_page_handle: Option<&str>) -> Value {
    let posts: Vec<Value> = posts.iter().map(|id| post_json(*id, 10)).collect();
    json!({ "posts": posts, "next_page_handle": next_page_handle })
}

fn key(id: i64) -> ReaderSavedKey {
    ReaderSavedKey::Post {
        blog_id: WpComSiteId(123),
        post_id: PostId(id),
    }
}

fn tag() -> ReaderStream {
    ReaderStream::Tag {
        slug: ReaderTagSlug("photography".to_string()),
    }
}

/// The stream's post IDs in order, each with `*` in front if it has a gap before it.
async fn shown(stream: &ReaderStreamCollection) -> Vec<String> {
    stream
        .load_items()
        .await
        .unwrap()
        .iter()
        .map(|item| {
            let gap = if item.gap_before { "*" } else { "" };
            format!("{gap}{}", item.post.identity.id)
        })
        .collect()
}

/// The query of each request sent, in order.
fn queries(executor: &ScriptedExecutor) -> Vec<String> {
    executor
        .sent()
        .iter()
        .map(|sent| {
            sent.url
                .split_once('?')
                .map(|(_, query)| query)
                .unwrap_or_default()
                .to_string()
        })
        .collect()
}

#[tokio::test]
async fn a_refresh_stores_the_first_page() {
    let (service, executor) = service();
    let stream = service.stream(ReaderStream::Following);
    assert!(shown(&stream).await.is_empty());
    assert_eq!(stream.info().unwrap().last_refreshed_at, None);
    executor.push(Step::Respond(200, page(&[(3, 12), (2, 11), (1, 10)])));

    let result = stream.refresh().await.unwrap();

    assert_eq!(
        result,
        ReaderStreamPageResult {
            post_count: 3,
            new_post_count: 3,
            unreadable_post_count: 0,
            has_more: true,
        }
    );
    assert_eq!(shown(&stream).await, ["3", "2", "1"]);
    let info = stream.info().unwrap();
    assert_eq!(info.state, ListState::Idle);
    assert_eq!(info.has_more, Some(true));
    assert!(info.last_refreshed_at.is_some());
    let sent = executor.sent();
    assert_eq!(
        sent[0].url,
        "https://public-api.wordpress.com/rest/v1.2/read/following?number=20&meta=site%2Cfeed"
    );
}

#[tokio::test]
async fn each_stream_reads_its_own_endpoint() {
    let (service, executor) = service();
    for stream in [
        ReaderStream::Site {
            site_id: WpComSiteId(123),
        },
        ReaderStream::Feed {
            feed_id: ReaderFeedId(456),
        },
        tag(),
    ] {
        executor.push(Step::Respond(200, page(&[(1, 10)])));
        service.stream(stream).refresh().await.unwrap();
    }
    let paths: Vec<String> = executor
        .sent()
        .iter()
        .map(|sent| {
            sent.url
                .trim_start_matches("https://public-api.wordpress.com")
                .to_string()
        })
        .collect();
    assert_eq!(
        paths,
        [
            "/rest/v1.2/read/sites/123/posts?number=20&meta=site%2Cfeed",
            "/rest/v1.2/read/feed/456/posts?number=20&meta=site%2Cfeed",
            "/wpcom/v2/read/tags/photography/posts?sort=date",
        ]
    );
}

#[tokio::test]
async fn items_come_without_content_and_the_cache_keeps_it() {
    let (service, executor) = service();
    let stream = service.stream(ReaderStream::Following);
    executor.push(Step::Respond(200, page(&[(1, 10)])));
    stream.refresh().await.unwrap();

    let items = stream.load_items().await.unwrap();
    assert_eq!(items[0].post.title.as_deref(), Some("Post 1"));
    assert_eq!(items[0].post.content, None);
    assert_eq!(items[0].sort_date, Some(date(10).parse().unwrap()));

    let stored = service.context.cache.execute(|connection| {
        ReaderPostRepository::select_by_key(&*connection, service.context.owner_row, &key(1))
    });
    assert_eq!(
        stored.unwrap().unwrap().1.content.as_deref(),
        Some("<p>The content of post 1.</p>")
    );
}

#[tokio::test]
async fn a_refresh_that_reaches_the_cached_posts_adds_the_new_ones_on_top() {
    let (service, executor) = service();
    let stream = service.stream(ReaderStream::Following);
    executor.push(Step::Respond(200, page(&[(3, 12), (2, 11), (1, 10)])));
    stream.refresh().await.unwrap();
    executor.push(Step::Respond(200, page(&[(5, 14), (4, 13), (3, 12)])));

    let result = stream.refresh().await.unwrap();

    assert_eq!(result.post_count, 3);
    assert_eq!(result.new_post_count, 2);
    assert_eq!(shown(&stream).await, ["5", "4", "3", "2", "1"]);
}

#[tokio::test]
async fn a_refresh_that_does_not_reach_the_cached_posts_leaves_a_gap_to_fill() {
    let (service, executor) = service();
    let stream = service.stream(ReaderStream::Following);
    executor.push(Step::Respond(200, page(&[(3, 12), (2, 11), (1, 10)])));
    stream.refresh().await.unwrap();
    executor.push(Step::Respond(200, page(&[(9, 20), (8, 19)])));
    stream.refresh().await.unwrap();
    assert_eq!(shown(&stream).await, ["9", "8", "*3", "2", "1"]);

    // The first page of the gap doesn't reach post 3.
    executor.push(Step::Respond(200, page(&[(7, 18), (6, 17)])));
    let result = stream.fill_gap(key(3)).await.unwrap();
    assert_eq!(result.new_post_count, 2);
    assert_eq!(shown(&stream).await, ["9", "8", "7", "6", "*3", "2", "1"]);

    // The second does, and the gap closes.
    executor.push(Step::Respond(200, page(&[(5, 16), (4, 15), (3, 12)])));
    stream.fill_gap(key(3)).await.unwrap();
    assert_eq!(
        shown(&stream).await,
        ["9", "8", "7", "6", "5", "4", "3", "2", "1"]
    );

    // Each page of the gap was asked for from the post above it.
    assert_eq!(
        queries(&executor)[2..],
        [
            "number=20&before=2026-11-18T19%3A00%3A00%2B00%3A00&meta=site%2Cfeed",
            "number=20&before=2026-11-18T17%3A00%3A00%2B00%3A00&meta=site%2Cfeed",
        ]
    );

    // With no gap left, filling it sends nothing.
    stream.fill_gap(key(3)).await.unwrap();
    assert_eq!(executor.sent().len(), 4);
}

#[tokio::test]
async fn a_gap_with_nothing_in_it_closes() {
    let (service, executor) = service();
    let stream = service.stream(ReaderStream::Following);
    executor.push(Step::Respond(200, page(&[(3, 12), (2, 11)])));
    stream.refresh().await.unwrap();
    executor.push(Step::Respond(200, page(&[(9, 20), (8, 19)])));
    stream.refresh().await.unwrap();

    // Nothing is older than post 8 any more: posts 3 and 2 are gone from the server.
    executor.push(Step::Respond(200, page(&[])));
    stream.fill_gap(key(3)).await.unwrap();

    assert_eq!(shown(&stream).await, ["9", "8"]);
}

#[tokio::test]
async fn filling_a_gap_before_a_post_that_is_not_in_the_stream_is_an_error() {
    let (service, executor) = service();
    let stream = service.stream(ReaderStream::Following);
    executor.push(Step::Respond(200, page(&[(1, 10)])));
    stream.refresh().await.unwrap();

    let result = stream.fill_gap(key(99)).await;

    assert!(matches!(result, Err(ReaderError::PostNotInStream)));
    assert_eq!(executor.sent().len(), 1);
}

#[tokio::test]
async fn load_more_asks_for_the_posts_before_the_oldest_until_there_are_none() {
    let (service, executor) = service();
    let stream = service.stream(ReaderStream::Following);
    executor.push(Step::Respond(200, page(&[(6, 15), (5, 14)])));
    stream.refresh().await.unwrap();

    executor.push(Step::Respond(200, page(&[(4, 13), (3, 12)])));
    let result = stream.load_more().await.unwrap();
    assert_eq!(result.new_post_count, 2);
    assert!(result.has_more);
    assert_eq!(shown(&stream).await, ["6", "5", "4", "3"]);

    executor.push(Step::Respond(200, page(&[])));
    let result = stream.load_more().await.unwrap();
    assert!(!result.has_more);
    assert_eq!(stream.info().unwrap().has_more, Some(false));
    assert_eq!(shown(&stream).await, ["6", "5", "4", "3"]);

    // At the end, nothing more is sent.
    stream.load_more().await.unwrap();
    assert_eq!(
        queries(&executor),
        [
            "number=20&meta=site%2Cfeed",
            "number=20&before=2026-11-18T14%3A00%3A00%2B00%3A00&meta=site%2Cfeed",
            "number=20&before=2026-11-18T12%3A00%3A00%2B00%3A00&meta=site%2Cfeed",
        ]
    );

    // A refresh that reaches the cached posts doesn't forget where the end is.
    executor.push(Step::Respond(200, page(&[(7, 16), (6, 15)])));
    let result = stream.refresh().await.unwrap();
    assert!(!result.has_more);
}

#[tokio::test]
async fn load_more_on_a_stream_that_has_not_loaded_refreshes_it() {
    let (service, executor) = service();
    let stream = service.stream(ReaderStream::Following);
    executor.push(Step::Respond(200, page(&[(2, 11), (1, 10)])));

    stream.load_more().await.unwrap();

    assert_eq!(queries(&executor), ["number=20&meta=site%2Cfeed"]);
    assert_eq!(shown(&stream).await, ["2", "1"]);
    assert!(stream.info().unwrap().last_refreshed_at.is_some());
}

#[tokio::test]
async fn an_empty_first_page_is_an_empty_stream() {
    let (service, executor) = service();
    let stream = service.stream(ReaderStream::Following);
    executor.push(Step::Respond(200, page(&[(2, 11), (1, 10)])));
    stream.refresh().await.unwrap();
    executor.push(Step::Respond(200, page(&[])));

    let result = stream.refresh().await.unwrap();

    assert!(!result.has_more);
    assert!(shown(&stream).await.is_empty());
}

#[tokio::test]
async fn a_stream_that_needs_an_account_sends_nothing_signed_out() {
    let (service, executor) = service_for(ReaderOwner::SignedOut);
    let stream = service.stream(ReaderStream::Following);

    assert!(matches!(
        stream.refresh().await,
        Err(ReaderError::NeedsAccount)
    ));
    assert!(matches!(
        stream.load_more().await,
        Err(ReaderError::NeedsAccount)
    ));
    assert!(executor.sent().is_empty());
    assert!(shown(&stream).await.is_empty());
}

#[tokio::test]
async fn a_signed_out_reader_reads_a_tag() {
    let (service, executor) = service_for(ReaderOwner::SignedOut);
    let stream = service.stream(tag());
    executor.push(Step::Respond(200, tag_page(&[2, 1], None)));

    stream.refresh().await.unwrap();

    assert_eq!(shown(&stream).await, ["2", "1"]);
    assert_eq!(service.owner(), ReaderOwner::SignedOut);
}

#[tokio::test]
async fn a_failed_refresh_is_recorded_and_keeps_the_cached_posts() {
    let (service, executor) = service();
    let stream = service.stream(ReaderStream::Following);
    executor.push(Step::Respond(200, page(&[(2, 11), (1, 10)])));
    stream.refresh().await.unwrap();
    executor.push(Step::Offline);

    let result = stream.refresh().await;

    assert!(matches!(result, Err(ReaderError::Api(_))));
    let info = stream.info().unwrap();
    assert_eq!(info.state, ListState::Error);
    assert!(info.error_message.is_some());
    assert_eq!(shown(&stream).await, ["2", "1"]);

    // The next one that works clears the error.
    executor.push(Step::Respond(200, page(&[(2, 11), (1, 10)])));
    stream.refresh().await.unwrap();
    let info = stream.info().unwrap();
    assert_eq!(info.state, ListState::Idle);
    assert_eq!(info.error_message, None);
}

#[tokio::test]
async fn a_post_that_cannot_be_read_is_kept_by_its_ids_until_a_page_reads_it() {
    let (service, executor) = service();
    let stream = service.stream(ReaderStream::Following);
    let mut unreadable = post_json(2, 11);
    unreadable["date"] = json!("2026-11-18T11:00:00+91:00");
    executor.push(Step::Respond(
        200,
        json!({ "posts": [post_json(3, 12), unreadable, post_json(1, 10)] }),
    ));

    let result = stream.refresh().await.unwrap();

    assert_eq!(result.post_count, 2);
    assert_eq!(result.unreadable_post_count, 1);
    assert_eq!(shown(&stream).await, ["3", "1"]);
    assert_eq!(stream.info().unwrap().unreadable_post_count, 1);
    let kept = service.context.cache.execute(|connection| {
        let stream =
            ReaderStreamRepository::select(&*connection, service.context.owner_row, "following")?
                .unwrap();
        ReaderUnreadablePostRepository::select(&*connection, stream.row_id)
    });
    assert_eq!(kept.unwrap()[0].identity.id, 2);

    // The server now sends it in a form that reads.
    executor.push(Step::Respond(200, page(&[(3, 12), (2, 11), (1, 10)])));
    let result = stream.refresh().await.unwrap();
    assert_eq!(result.unreadable_post_count, 0);
    assert_eq!(shown(&stream).await, ["3", "2", "1"]);
    assert_eq!(stream.info().unwrap().unreadable_post_count, 0);
}

#[tokio::test]
async fn a_page_with_nothing_readable_changes_nothing() {
    let (service, executor) = service();
    let stream = service.stream(ReaderStream::Following);
    executor.push(Step::Respond(200, page(&[(2, 11), (1, 10)])));
    stream.refresh().await.unwrap();
    let mut unreadable = post_json(3, 12);
    unreadable["date"] = json!("2026-11-18T12:00:00+91:00");
    executor.push(Step::Respond(200, json!({ "posts": [unreadable] })));

    let result = stream.refresh().await.unwrap();

    assert_eq!(result.post_count, 0);
    assert_eq!(result.unreadable_post_count, 1);
    assert_eq!(shown(&stream).await, ["2", "1"]);
}

#[tokio::test]
async fn a_post_without_a_date_cannot_be_placed_in_a_stream_ordered_by_date() {
    let (service, executor) = service();
    let stream = service.stream(ReaderStream::Following);
    let mut undated = post_json(2, 11);
    undated.as_object_mut().unwrap().remove("date");
    executor.push(Step::Respond(
        200,
        json!({ "posts": [post_json(3, 12), undated] }),
    ));

    let result = stream.refresh().await.unwrap();

    assert_eq!(result.unreadable_post_count, 1);
    assert_eq!(shown(&stream).await, ["3"]);
}

#[tokio::test]
async fn a_tag_stream_follows_its_page_handles_and_a_refresh_replaces_it() {
    let (service, executor) = service();
    let stream = service.stream(tag());
    executor.push(Step::Respond(200, tag_page(&[1, 2], Some("second"))));
    let result = stream.refresh().await.unwrap();
    assert!(result.has_more);

    executor.push(Step::Respond(200, tag_page(&[3, 2, 4], None)));
    let result = stream.load_more().await.unwrap();
    assert_eq!(result.new_post_count, 2);
    assert!(!result.has_more);
    assert_eq!(shown(&stream).await, ["1", "2", "3", "4"]);
    assert_eq!(
        queries(&executor),
        ["sort=date", "page_handle=second&sort=date"]
    );

    // At the end, nothing more is sent.
    stream.load_more().await.unwrap();
    assert_eq!(executor.sent().len(), 2);

    executor.push(Step::Respond(200, tag_page(&[5, 1], Some("again"))));
    stream.refresh().await.unwrap();
    assert_eq!(shown(&stream).await, ["5", "1"]);
    assert_eq!(stream.info().unwrap().has_more, Some(true));
    assert!(
        stream
            .load_items()
            .await
            .unwrap()
            .iter()
            .all(|item| item.sort_date.is_none())
    );
}

#[tokio::test]
async fn a_tag_page_that_arrives_after_a_refresh_is_discarded() {
    let (service, executor) = service();
    let stream = service.stream(tag());
    executor.push(Step::Respond(200, tag_page(&[1, 2], Some("second"))));
    stream.refresh().await.unwrap();

    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, tag_page(&[3, 4], Some("third"))));
    executor.push(Step::Respond(200, tag_page(&[9, 8], Some("new second"))));
    let (more, refreshed) = futures::join!(stream.load_more(), async {
        // The refresh is answered while the next page is still on its way.
        let refreshed = stream.refresh().await;
        release.send(()).unwrap();
        refreshed
    });

    refreshed.unwrap();
    assert_eq!(more.unwrap().post_count, 0);
    assert_eq!(shown(&stream).await, ["9", "8"]);
    // The next page follows from the refresh, not from the page that was discarded.
    executor.push(Step::Respond(200, tag_page(&[7], None)));
    stream.load_more().await.unwrap();
    assert_eq!(
        queries(&executor).last().unwrap(),
        "page_handle=new+second&sort=date"
    );
}

#[tokio::test]
async fn two_refreshes_at_once_send_one_request() {
    let (service, executor) = service();
    let stream = service.stream(ReaderStream::Following);
    // A second collection of the same stream takes turns with the first.
    let same_stream = service.stream(ReaderStream::Following);
    let (release, held) = oneshot::channel();
    executor.push(Step::Hold(held, 200, page(&[(2, 11), (1, 10)])));

    let (first, second, ()) = futures::join!(stream.refresh(), same_stream.refresh(), async {
        release.send(()).unwrap();
    });

    assert_eq!(first.unwrap(), second.unwrap());
    assert_eq!(executor.sent().len(), 1);
}

#[tokio::test]
async fn each_reader_has_its_own_streams() {
    let cache = cache();
    let executor = Arc::new(ScriptedExecutor::default());
    let new = |owner| {
        ReaderService::new(
            delegate(executor.clone()),
            cache.clone(),
            owner,
            ReaderSavedSource::Ios,
        )
        .unwrap()
    };
    let account = new(ACCOUNT);
    let signed_out = new(ReaderOwner::SignedOut);
    executor.push(Step::Respond(200, tag_page(&[1, 2], None)));
    account.stream(tag()).refresh().await.unwrap();

    assert_eq!(shown(&account.stream(tag())).await, ["1", "2"]);
    assert!(shown(&signed_out.stream(tag())).await.is_empty());

    // Removing the account's data leaves nothing for a new service to show.
    assert!(cache.remove_reader_data(ACCOUNT).unwrap());
    let account = new(ACCOUNT);
    assert!(shown(&account.stream(tag())).await.is_empty());
}

#[tokio::test]
async fn a_refresh_cleans_up_streams_nobody_has_refreshed_in_a_month() {
    let executor = Arc::new(ScriptedExecutor::default());
    let now = Arc::new(Mutex::new(SystemTime::now()));
    let clock = {
        let now = now.clone();
        Arc::new(move || *now.lock().unwrap())
    };
    let service = ReaderService::with_clock(
        delegate(executor.clone()),
        cache(),
        ACCOUNT,
        ReaderSavedSource::Ios,
        clock,
    )
    .unwrap();
    let old = service.stream(tag());
    executor.push(Step::Respond(200, tag_page(&[1, 2], None)));
    old.refresh().await.unwrap();

    // A month and a day later, another stream is refreshed.
    *now.lock().unwrap() += Duration::from_secs(31 * 24 * 60 * 60);
    executor.push(Step::Respond(200, page(&[(3, 12)])));
    service
        .stream(ReaderStream::Following)
        .refresh()
        .await
        .unwrap();

    assert!(shown(&old).await.is_empty());
    assert_eq!(old.info().unwrap().last_refreshed_at, None);
    assert_eq!(shown(&service.stream(ReaderStream::Following)).await, ["3"]);
    // The old stream's posts went with it.
    let post = service.context.cache.execute(|connection| {
        ReaderPostRepository::select_by_key(&*connection, service.context.owner_row, &key(1))
    });
    assert_eq!(post.unwrap(), None);
}

#[tokio::test]
async fn a_change_to_a_stream_table_is_relevant() {
    let (service, _) = service();
    let stream = service.stream(ReaderStream::Following);
    let hook = |table| wp_mobile_cache::UpdateHook {
        action: wp_mobile_cache::HookAction::Insert,
        db_name: "main".to_string(),
        table,
        row_id: 1,
    };
    assert!(stream.is_relevant_update(&hook(wp_mobile_cache::DbTable::ReaderStreamItems)));
    assert!(stream.is_relevant_update(&hook(wp_mobile_cache::DbTable::ReaderPosts)));
    assert!(!stream.is_relevant_update(&hook(wp_mobile_cache::DbTable::ReaderSavedPosts)));
}

#[tokio::test]
async fn the_saved_posts_are_the_readers() {
    let (service, executor) = service_for(ReaderOwner::SignedOut);
    let post: wp_api::wp_com::reader_posts::ReaderPost =
        serde_json::from_value(post_json(1, 10)).unwrap();

    // A signed-out reader's save stays on the device, with the post it was made from.
    service.saved().save_with_details(post).await.unwrap();

    assert!(executor.sent().is_empty());
    let saved = service.saved().saved_posts().load_items().await.unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(
        saved[0].post.as_ref().unwrap().title.as_deref(),
        Some("Post 1")
    );
}
