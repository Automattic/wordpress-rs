use super::*;
use crate::testing::scripted::{ScriptedExecutor, Step, delegate};
use futures::channel::oneshot;
use serde_json::{Value, json};
use std::time::Duration;
use wp_api::posts::PostId;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::reader_posts::{
    ReaderListOwner, ReaderListSlug, ReaderSearchSort, ReaderTagSlug,
};
use wp_api::wp_com::reader_saved::{ReaderFeedId, ReaderSavedKey, ReaderSavedSource};
use wp_mobile_cache::db_types::reader::ReaderPostRecord;
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
    let slug = |slug: &str| ReaderTagSlug(slug.to_string());
    let streams = [
        (
            ReaderStream::Following,
            "/rest/v1.2/read/following?number=20&meta=site%2Cfeed",
        ),
        (
            ReaderStream::Liked,
            "/rest/v1.2/read/liked?number=20&meta=site%2Cfeed",
        ),
        (
            ReaderStream::Automattic,
            "/rest/v1.2/read/a8c?number=20&meta=site%2Cfeed",
        ),
        (
            ReaderStream::P2,
            "/rest/v1.2/read/following/p2?number=20&meta=site%2Cfeed",
        ),
        (
            ReaderStream::FreshlyPressed,
            "/rest/v1.2/freshly-pressed?number=20&meta=site%2Cfeed",
        ),
        (
            ReaderStream::Site {
                site_id: WpComSiteId(123),
            },
            "/rest/v1.2/read/sites/123/posts?number=20&meta=site%2Cfeed",
        ),
        (
            ReaderStream::Feed {
                feed_id: ReaderFeedId(456),
            },
            "/rest/v1.2/read/feed/456/posts?number=20&meta=site%2Cfeed",
        ),
        (tag(), "/wpcom/v2/read/tags/photography/posts?sort=date"),
        (
            ReaderStream::Tags {
                slugs: vec![slug("art"), slug("travel")],
            },
            "/wpcom/v2/read/tags/posts?tags=art%2Ctravel&sort=date",
        ),
        (
            ReaderStream::Search {
                query: "cats and dogs".to_string(),
                sort: ReaderSearchSort::Date,
            },
            "/rest/v1.2/read/search?q=cats+and+dogs&number=20&sort=date&meta=site%2Cfeed",
        ),
        (
            ReaderStream::ReaderList {
                owner: ReaderListOwner("automattic".to_string()),
                slug: ReaderListSlug("design".to_string()),
            },
            "/rest/v1.3/read/list/automattic/design/posts?",
        ),
        (
            ReaderStream::Discover {
                tags: vec![slug("art")],
            },
            "/wpcom/v2/read/streams/discover?tags=art",
        ),
        (
            ReaderStream::FirstPosts,
            "/wpcom/v2/read/streams/first-posts?",
        ),
    ];
    for (stream, _) in &streams {
        executor.push(Step::Respond(200, page(&[(1, 10)])));
        service.stream(stream.clone()).refresh().await.unwrap();
    }
    let sent = executor.sent();
    for (index, (stream, path)) in streams.iter().enumerate() {
        assert_eq!(
            sent[index]
                .url
                .trim_start_matches("https://public-api.wordpress.com"),
            *path,
            "{stream:?}"
        );
    }
}

#[tokio::test]
async fn liked_posts_are_ordered_and_paged_by_when_they_were_liked() {
    let (service, executor) = service();
    let stream = service.stream(ReaderStream::Liked);
    // Post 1 was written last and liked first.
    let liked = |id: u64, written: u32, liked: u32| {
        let mut post = post_json(id, written);
        post["date_liked"] = json!(date(liked));
        post
    };
    executor.push(Step::Respond(
        200,
        json!({ "posts": [liked(2, 5, 12), liked(1, 20, 11)] }),
    ));
    stream.refresh().await.unwrap();

    assert_eq!(shown(&stream).await, ["2", "1"]);
    let items = stream.load_items().await.unwrap();
    assert_eq!(items[1].sort_date, Some(date(11).parse().unwrap()));

    executor.push(Step::Respond(200, page(&[])));
    stream.load_more().await.unwrap();
    assert_eq!(
        queries(&executor)[1],
        "number=20&before=2026-11-18T11%3A00%3A00%2B00%3A00&meta=site%2Cfeed"
    );
}

#[tokio::test]
async fn freshly_pressed_posts_are_ordered_by_when_they_were_shown() {
    let (service, executor) = service_for(ReaderOwner::SignedOut);
    let stream = service.stream(ReaderStream::FreshlyPressed);
    let shown_at = |id: u64, written: u32, displayed: u32| {
        let mut post = post_json(id, written);
        post["editorial"] = json!({ "displayed_on": date(displayed) });
        post
    };
    executor.push(Step::Respond(
        200,
        json!({ "posts": [shown_at(2, 5, 12), shown_at(1, 20, 11)] }),
    ));

    stream.refresh().await.unwrap();

    assert_eq!(shown(&stream).await, ["2", "1"]);
}

#[tokio::test]
async fn streams_of_an_account_need_one() {
    let (service, executor) = service_for(ReaderOwner::SignedOut);
    for stream in [
        ReaderStream::Following,
        ReaderStream::Liked,
        ReaderStream::Automattic,
        ReaderStream::P2,
    ] {
        assert!(
            matches!(
                service.stream(stream.clone()).refresh().await,
                Err(ReaderError::NeedsAccount)
            ),
            "{stream:?}"
        );
    }
    assert!(executor.sent().is_empty());
}

#[tokio::test]
async fn a_search_follows_its_page_handles() {
    let (service, executor) = service_for(ReaderOwner::SignedOut);
    let stream = service.stream(ReaderStream::Search {
        query: "cats".to_string(),
        sort: ReaderSearchSort::Relevance,
    });
    executor.push(Step::Respond(
        200,
        json!({ "posts": [post_json(2, 10), post_json(1, 12)], "next_page": "20" }),
    ));
    let result = stream.refresh().await.unwrap();
    assert!(result.has_more);
    // Results keep the server's order, which isn't by date.
    assert_eq!(shown(&stream).await, ["2", "1"]);

    executor.push(Step::Respond(200, json!({ "posts": [post_json(3, 11)] })));
    let result = stream.load_more().await.unwrap();
    assert!(!result.has_more);
    assert_eq!(shown(&stream).await, ["2", "1", "3"]);
    assert_eq!(
        queries(&executor)[1],
        "q=cats&number=20&page_handle=20&sort=relevance&meta=site%2Cfeed"
    );
}

#[tokio::test]
async fn discover_keeps_its_cards_among_its_posts() {
    let (service, executor) = service_for(ReaderOwner::SignedOut);
    let stream = service.stream(ReaderStream::Discover { tags: vec![] });
    let post_card = |id: u64| json!({ "type": "post", "data": post_json(id, 10) });
    executor.push(Step::Respond(
        200,
        json!({
            "cards": [
                post_card(1),
                { "type": "recommended_blogs", "data": [
                    { "ID": 7, "name": "A site", "URL": "https://seven.example", "feed_ID": 70 },
                ] },
                post_card(2),
                { "type": "interests_you_may_like", "data": [
                    { "slug": "art", "title": "Art", "score": 1.5 },
                ] },
                { "type": "something_new", "data": {} },
            ],
            "next_page_handle": "second",
        }),
    ));

    let result = stream.refresh().await.unwrap();

    assert_eq!(result.post_count, 2);
    assert!(result.has_more);
    assert_eq!(shown(&stream).await, ["1", "2"]);
    let cards = stream.load_cards().await.unwrap();
    assert_eq!(cards.len(), 2);
    assert_eq!(cards[0].after_post, Some(key(1)));
    let ReaderStreamCardKind::RecommendedSites { sites } = &cards[0].card else {
        panic!("The first card should be the recommended sites");
    };
    assert_eq!(sites[0].site_id, Some(WpComSiteId(7)));
    assert_eq!(sites[0].name.as_deref(), Some("A site"));
    assert_eq!(cards[1].after_post, Some(key(2)));
    assert!(matches!(
        &cards[1].card,
        ReaderStreamCardKind::SuggestedTags { tags } if tags[0].slug.0 == "art"
    ));

    // The next page's cards go after everything the stream has.
    executor.push(Step::Respond(
        200,
        json!({
            "cards": [post_card(3), { "type": "new_sites", "data": [{ "ID": 8 }] }],
            "next_page_handle": "",
        }),
    ));
    let result = stream.load_more().await.unwrap();
    assert!(!result.has_more);
    let cards = stream.load_cards().await.unwrap();
    assert_eq!(cards.len(), 3);
    assert_eq!(cards[2].after_post, Some(key(3)));

    // A refresh starts the cards again.
    executor.push(Step::Respond(200, json!({ "cards": [post_card(4)] })));
    stream.refresh().await.unwrap();
    assert_eq!(shown(&stream).await, ["4"]);
    assert!(stream.load_cards().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_lists_posts_leave_out_the_ones_the_server_no_longer_has() {
    let (service, executor) = service_for(ReaderOwner::SignedOut);
    let stream = service.stream(ReaderStream::ReaderList {
        owner: ReaderListOwner("automattic".to_string()),
        slug: ReaderListSlug("design".to_string()),
    });
    executor.push(Step::Respond(
        200,
        json!({
            "cards": [
                { "type": "post", "data": post_json(1, 10) },
                { "type": "post", "data": false },
                { "type": "post", "data": post_json(2, 9) },
            ],
        }),
    ));

    let result = stream.refresh().await.unwrap();

    assert_eq!(result.post_count, 2);
    assert_eq!(shown(&stream).await, ["1", "2"]);
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

    let stored = service.cached_post(key(1)).unwrap().unwrap();
    assert_eq!(
        stored.content.as_deref(),
        Some("<p>The content of post 1.</p>")
    );
    assert_eq!(service.cached_post(key(99)).unwrap(), None);
}

#[tokio::test]
async fn a_stored_post_has_its_summary_reading_time_and_card_type() {
    let (service, executor) = service();
    let stream = service.stream(ReaderStream::Following);
    let mut written = post_json(1, 10);
    written["excerpt"] = json!("<p>An excerpt &amp; more [&hellip;]</p>");
    written["word_count"] = json!(450);
    // A source that sends neither excerpt nor content has nothing to work them out from.
    let mut bare = post_json(2, 9);
    bare.as_object_mut().unwrap().remove("content");
    executor.push(Step::Respond(200, json!({ "posts": [written, bare] })));

    stream.refresh().await.unwrap();

    let items = stream.load_items().await.unwrap();
    assert_eq!(items[0].post.summary.as_deref(), Some("An excerpt & more…"));
    assert_eq!(items[0].post.reading_time_minutes, Some(3));
    assert_eq!(
        items[0].post.card_type,
        Some(wp_api::wp_com::reader_post_display::ReaderPostCardType::Default)
    );
    assert_eq!(items[1].post.summary, None);
    assert_eq!(items[1].post.reading_time_minutes, None);
    assert_eq!(items[1].post.card_type, None);
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

mod actions {
    use super::*;
    use wp_api::wp_com::reader_saved::{ReaderFeedItemId, ReaderPostSaveIdentity};

    fn identity(id: u64) -> ReaderPostSaveIdentity {
        ReaderPostSaveIdentity {
            id,
            site_id: Some(WpComSiteId(123)),
            feed_id: None,
            feed_item_id: None,
            is_external: false,
            url: None,
        }
    }

    /// A followed post that hasn't been seen, written by user 7.
    fn followed_post(id: u64, hour: u32) -> Value {
        let mut post = post_json(id, hour);
        post["is_seen"] = json!(false);
        post["site_name"] = json!("A site");
        post["author"] = json!({ "ID": 7, "name": "An author" });
        post
    }

    /// A service whose Following stream has posts 2 and 1.
    async fn following() -> (
        ReaderService,
        Arc<ScriptedExecutor>,
        Arc<ReaderStreamCollection>,
    ) {
        let (service, executor) = service();
        let stream = service.stream(ReaderStream::Following);
        executor.push(Step::Respond(
            200,
            json!({ "posts": [followed_post(2, 11), followed_post(1, 10)] }),
        ));
        stream.refresh().await.unwrap();
        (service, executor, stream)
    }

    async fn is_seen(stream: &ReaderStreamCollection, id: u64) -> Option<bool> {
        stream
            .load_items()
            .await
            .unwrap()
            .iter()
            .find(|item| item.post.identity.id == id)
            .and_then(|item| item.post.is_seen)
    }

    fn refused() -> Step {
        Step::Respond(
            404,
            json!({ "code": "post_not_found", "message": "Post not found.", "data": { "status": 404 } }),
        )
    }

    fn marked() -> Step {
        Step::Respond(200, json!({ "status": true }))
    }

    /// The path and body of each request sent after the first `skip`.
    fn sent_after(executor: &ScriptedExecutor, skip: usize) -> Vec<(String, Value)> {
        executor
            .sent()
            .iter()
            .skip(skip)
            .map(|sent| {
                (
                    sent.url
                        .trim_start_matches("https://public-api.wordpress.com")
                        .to_string(),
                    sent.body.clone().unwrap_or(Value::Null),
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn marking_a_post_as_seen_shows_at_once_and_is_sent() {
        let (service, executor, stream) = following().await;
        let (release, held) = oneshot::channel();
        executor.push(Step::Hold(held, 200, json!({ "status": true })));

        let (outcome, ()) = futures::join!(service.set_post_seen(identity(2), true), async {
            // It shows while the request is on its way.
            assert_eq!(is_seen(&stream, 2).await, Some(true));
            assert_eq!(service.pending_action_count().unwrap(), 1);
            release.send(()).unwrap();
        });

        assert_eq!(outcome.unwrap(), ReaderActionOutcome::Synced);
        assert_eq!(is_seen(&stream, 2).await, Some(true));
        assert_eq!(is_seen(&stream, 1).await, Some(false));
        assert_eq!(service.pending_action_count().unwrap(), 0);
        assert_eq!(
            sent_after(&executor, 1),
            [(
                "/wpcom/v2/seen-posts/seen/blog/new".to_string(),
                json!({ "blog_id": 123, "post_ids": [2], "source": "reader-ios" })
            )]
        );
    }

    #[tokio::test]
    async fn an_rss_post_is_marked_by_its_feed() {
        let (service, executor) = service();
        executor.push(marked());
        let post = ReaderPostSaveIdentity {
            id: 9,
            site_id: None,
            feed_id: Some(ReaderFeedId(456)),
            feed_item_id: Some(ReaderFeedItemId(9)),
            is_external: true,
            url: None,
        };

        service.set_post_seen(post, false).await.unwrap();

        assert_eq!(
            sent_after(&executor, 0),
            [(
                "/wpcom/v2/seen-posts/seen/delete".to_string(),
                json!({ "feed_id": 456, "feed_item_ids": [9], "source": "reader-ios" })
            )]
        );
    }

    #[tokio::test]
    async fn offline_it_waits_and_a_sync_sends_it() {
        let (service, executor, stream) = following().await;
        executor.push(Step::Offline);

        let outcome = service.set_post_seen(identity(2), true).await.unwrap();

        assert!(matches!(outcome, ReaderActionOutcome::Pending { .. }));
        assert_eq!(is_seen(&stream, 2).await, Some(true));
        assert_eq!(service.pending_action_count().unwrap(), 1);

        // A refresh brings the server's older state, which doesn't undo it.
        executor.push(Step::Respond(
            200,
            json!({ "posts": [followed_post(2, 11), followed_post(1, 10)] }),
        ));
        stream.refresh().await.unwrap();
        assert_eq!(is_seen(&stream, 2).await, Some(true));

        executor.push(marked());
        assert_eq!(service.sync_actions().await.unwrap(), 0);
        assert_eq!(is_seen(&stream, 2).await, Some(true));
        assert_eq!(
            sent_after(&executor, 3)[0].0,
            "/wpcom/v2/seen-posts/seen/blog/new"
        );
    }

    #[tokio::test]
    async fn a_refusal_puts_it_back_and_is_returned() {
        let (service, executor, stream) = following().await;
        executor.push(refused());

        let result = service.set_post_seen(identity(2), true).await;

        assert!(matches!(result, Err(ReaderError::Api(_))));
        assert_eq!(is_seen(&stream, 2).await, Some(false));
        assert_eq!(service.pending_action_count().unwrap(), 0);
    }

    #[tokio::test]
    async fn turning_it_on_and_off_offline_sends_only_the_last() {
        let (service, executor, stream) = following().await;
        executor.push(Step::Offline);
        service.set_post_seen(identity(2), true).await.unwrap();
        executor.push(Step::Offline);
        service.set_post_seen(identity(2), false).await.unwrap();
        assert_eq!(is_seen(&stream, 2).await, Some(false));
        assert_eq!(service.pending_action_count().unwrap(), 1);

        executor.push(marked());
        assert_eq!(service.sync_actions().await.unwrap(), 0);

        assert_eq!(
            sent_after(&executor, 3),
            [(
                "/wpcom/v2/seen-posts/seen/blog/delete".to_string(),
                json!({ "blog_id": 123, "post_ids": [2], "source": "reader-ios" })
            )]
        );
    }

    #[tokio::test]
    async fn a_tap_while_its_request_is_on_its_way_is_sent_after_it() {
        let (service, executor, stream) = following().await;
        let (release, held) = oneshot::channel();
        executor.push(Step::Hold(held, 200, json!({ "status": true })));
        executor.push(marked());

        let (first, second) = futures::join!(service.set_post_seen(identity(2), true), async {
            let second = service.set_post_seen(identity(2), false);
            release.send(()).unwrap();
            second.await
        });

        assert_eq!(first.unwrap(), ReaderActionOutcome::Synced);
        assert_eq!(second.unwrap(), ReaderActionOutcome::Synced);
        assert_eq!(is_seen(&stream, 2).await, Some(false));
        let paths: Vec<_> = sent_after(&executor, 1)
            .into_iter()
            .map(|(path, _)| path)
            .collect();
        assert_eq!(
            paths,
            [
                "/wpcom/v2/seen-posts/seen/blog/new",
                "/wpcom/v2/seen-posts/seen/blog/delete"
            ]
        );
    }

    #[tokio::test]
    async fn a_refusal_of_an_older_tap_leaves_the_newer_one() {
        let (service, executor, stream) = following().await;
        let (release, held) = oneshot::channel();
        // The server refuses the first tap, and takes the second.
        executor.push(Step::Hold(
            held,
            404,
            json!({ "code": "post_not_found", "message": "Post not found.", "data": { "status": 404 } }),
        ));
        executor.push(marked());

        let (first, second) = futures::join!(service.set_post_seen(identity(2), true), async {
            let second = service.set_post_seen(identity(2), false);
            release.send(()).unwrap();
            second.await
        });

        // The newer tap is what the reader wants, so the older one's refusal isn't reported.
        assert_eq!(first.unwrap(), ReaderActionOutcome::Synced);
        assert_eq!(second.unwrap(), ReaderActionOutcome::Synced);
        assert_eq!(is_seen(&stream, 2).await, Some(false));
    }

    #[tokio::test]
    async fn actions_need_an_account() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);

        assert!(matches!(
            service.set_post_seen(identity(2), true).await,
            Err(ReaderError::NeedsAccount)
        ));
        assert!(matches!(
            service.set_site_blocked(WpComSiteId(123), true).await,
            Err(ReaderError::NeedsAccount)
        ));
        assert!(matches!(
            service.refresh_blocked_sites().await,
            Err(ReaderError::NeedsAccount)
        ));
        assert_eq!(service.sync_actions().await.unwrap(), 0);
        assert!(executor.sent().is_empty());
    }

    #[tokio::test]
    async fn a_post_without_ids_cannot_be_marked() {
        let (service, executor) = service();
        let post = ReaderPostSaveIdentity {
            id: 0,
            site_id: None,
            feed_id: None,
            feed_item_id: None,
            is_external: false,
            url: None,
        };

        let result = service.set_post_seen(post, true).await;

        assert!(matches!(result, Err(ReaderError::MissingIds)));
        assert!(executor.sent().is_empty());
    }

    #[tokio::test]
    async fn blocking_a_site_hides_its_posts_at_once() {
        let (service, executor, stream) = following().await;
        executor.push(Step::Respond(
            200,
            json!({ "success": true, "modified_sub": true }),
        ));

        let outcome = service
            .set_site_blocked(WpComSiteId(123), true)
            .await
            .unwrap();

        assert_eq!(outcome, ReaderActionOutcome::Synced);
        assert!(shown(&stream).await.is_empty());
        // The site's name comes from a post of it the cache has.
        let blocked = service.blocked_sites().unwrap();
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].id, WpComSiteId(123));
        assert_eq!(blocked[0].name.as_deref(), Some("A site"));
        assert_eq!(
            sent_after(&executor, 1)[0].0,
            "/rest/v1.1/me/block/sites/123/new"
        );

        // Unblocking brings them back, since the cache kept them.
        executor.push(Step::Respond(
            200,
            json!({ "success": true, "modified_sub": true }),
        ));
        service
            .set_site_blocked(WpComSiteId(123), false)
            .await
            .unwrap();
        assert_eq!(shown(&stream).await, ["2", "1"]);
        assert!(service.blocked_sites().unwrap().is_empty());
        assert_eq!(
            sent_after(&executor, 2)[0].0,
            "/rest/v1.1/me/block/sites/123/delete"
        );
    }

    #[tokio::test]
    async fn a_refused_block_shows_the_posts_again() {
        let (service, executor, stream) = following().await;
        executor.push(refused());

        let result = service.set_site_blocked(WpComSiteId(123), true).await;

        assert!(matches!(result, Err(ReaderError::Api(_))));
        assert_eq!(shown(&stream).await, ["2", "1"]);
        assert!(service.blocked_sites().unwrap().is_empty());
    }

    #[tokio::test]
    async fn the_servers_blocked_sites_replace_the_cached_ones_except_what_is_waiting() {
        let (service, executor, _) = following().await;
        // Site 123's block is waiting to be sent.
        executor.push(Step::Offline);
        service
            .set_site_blocked(WpComSiteId(123), true)
            .await
            .unwrap();
        executor.push(Step::Respond(
            200,
            json!({
                "sites": [{ "ID": 7, "name": "Seven", "URL": "https://seven.example" }],
                "page": 1,
                "count": 1,
            }),
        ));

        service.refresh_blocked_sites().await.unwrap();

        let ids: Vec<_> = service
            .blocked_sites()
            .unwrap()
            .iter()
            .map(|site| site.id.0)
            .collect();
        assert_eq!(ids, [123, 7]);
        assert_eq!(
            executor.sent().last().unwrap().url,
            "https://public-api.wordpress.com/wpcom/v2/me/blocks/sites?page=1&per_page=100"
        );
    }

    #[tokio::test]
    async fn blocking_a_user_hides_their_posts_on_this_device() {
        let (service, executor, stream) = following().await;

        service.set_user_blocked(WpComUserId(7), true).unwrap();

        assert!(shown(&stream).await.is_empty());
        assert_eq!(service.blocked_users().unwrap(), [WpComUserId(7)]);
        // Nothing is sent: the server keeps no list of blocked users.
        assert_eq!(executor.sent().len(), 1);

        service.set_user_blocked(WpComUserId(7), false).unwrap();
        assert_eq!(shown(&stream).await, ["2", "1"]);

        // It works signed out too.
        let (signed_out, _) = service_for(ReaderOwner::SignedOut);
        signed_out.set_user_blocked(WpComUserId(7), true).unwrap();
        assert_eq!(signed_out.blocked_users().unwrap(), [WpComUserId(7)]);
    }
}

mod page_views {
    use super::*;

    /// A post of a site, as a stream sends it.
    fn site_post(id: u64, private: bool) -> Value {
        let mut post = post_json(id, 10);
        post["site_URL"] = json!("https://example.wordpress.com");
        post["site_is_private"] = json!(private);
        post
    }

    /// A service whose tag stream holds `posts`.
    async fn reading(
        owner: ReaderOwner,
        posts: Vec<Value>,
    ) -> (ReaderService, Arc<ScriptedExecutor>) {
        let (service, executor) = service_for(owner);
        executor.push(Step::Respond(200, json!({ "posts": posts })));
        service.stream(tag()).refresh().await.unwrap();
        (service, executor)
    }

    fn pixel() -> Step {
        // The pixel answers with an image, which nothing reads.
        Step::Respond(200, json!(null))
    }

    fn sites(ids: &[u64]) -> Step {
        let sites: Vec<Value> = ids.iter().map(|id| site_json(*id)).collect();
        Step::Respond(200, json!({ "sites": sites }))
    }

    /// A site as the account's site list sends one, with its ID changed.
    fn site_json(id: u64) -> Value {
        let mut site: Value = serde_json::from_str(include_str!(
            "../../../../wp_api/tests/wpcom/sites/v1.2-sites-01.json"
        ))
        .expect("the site fixture is JSON");
        site["ID"] = json!(id);
        site
    }

    #[tokio::test]
    async fn a_signed_out_readers_view_is_counted() {
        let (service, executor) = reading(ReaderOwner::SignedOut, vec![site_post(1, false)]).await;
        executor.push(pixel());

        let outcome = service.post_opened(key(1)).await.unwrap();

        assert_eq!(outcome, ReaderPageViewOutcome::Counted);
        let sent = executor.sent();
        let url = &sent.last().unwrap().url;
        assert!(
            url.starts_with(
                "https://pixel.wp.com/g.gif?v=wpcom&reader=1&blog=123&post=1&host=example.wordpress.com&ref=https%3A%2F%2Fwordpress.com%2F&t="
            ),
            "{url}"
        );
        assert!(!url.contains("priv=1"));
        // Nothing else was asked for: a signed-out reader is no site's member.
        assert_eq!(sent.len(), 2);
    }

    #[tokio::test]
    async fn a_view_by_a_member_of_a_public_site_is_not_counted() {
        let (service, executor) =
            reading(ACCOUNT, vec![site_post(1, false), site_post(2, false)]).await;
        // The account's sites are fetched the first time a post is opened.
        executor.push(sites(&[123]));

        let outcome = service.post_opened(key(1)).await.unwrap();

        assert_eq!(outcome, ReaderPageViewOutcome::NotCountedForSiteMember);
        assert!(
            executor
                .sent()
                .last()
                .unwrap()
                .url
                .starts_with("https://public-api.wordpress.com/rest/v1.2/me/sites")
        );
        // They're kept, so the next post asks for nothing.
        let outcome = service.post_opened(key(2)).await.unwrap();
        assert_eq!(outcome, ReaderPageViewOutcome::NotCountedForSiteMember);
        assert_eq!(executor.sent().len(), 2);
    }

    #[tokio::test]
    async fn a_view_by_someone_who_is_not_a_member_is_counted() {
        let (service, executor) = reading(ACCOUNT, vec![site_post(1, false)]).await;
        service.set_member_sites(vec![WpComSiteId(999)]).unwrap();
        executor.push(pixel());

        let outcome = service.post_opened(key(1)).await.unwrap();

        assert_eq!(outcome, ReaderPageViewOutcome::Counted);
        // The app said which sites the account belongs to, so they weren't fetched.
        assert_eq!(executor.sent().len(), 2);
    }

    #[tokio::test]
    async fn a_view_of_a_private_site_is_counted_even_by_a_member() {
        let (service, executor) = reading(ACCOUNT, vec![site_post(1, true)]).await;
        service.set_member_sites(vec![WpComSiteId(123)]).unwrap();
        executor.push(pixel());

        let outcome = service.post_opened(key(1)).await.unwrap();

        assert_eq!(outcome, ReaderPageViewOutcome::Counted);
        assert!(executor.sent().last().unwrap().url.ends_with("&priv=1"));
    }

    #[tokio::test]
    async fn a_view_of_an_rss_post_is_not_counted() {
        let rss = json!({
            "ID": 9, "feed_ID": 456, "feed_item_ID": 9, "is_external": true,
            "site_URL": "https://example.com", "date": date(10),
        });
        let (service, executor) = reading(ReaderOwner::SignedOut, vec![rss]).await;
        let key = ReaderSavedKey::FeedItem {
            feed_item_id: wp_api::wp_com::reader_saved::ReaderFeedItemId(9),
        };

        let outcome = service.post_opened(key).await.unwrap();

        assert_eq!(outcome, ReaderPageViewOutcome::NotCountedForRssPost);
        assert_eq!(executor.sent().len(), 1);
    }

    #[tokio::test]
    async fn a_post_the_cache_lacks_or_knows_too_little_of_is_not_counted() {
        // Post 1 has no site URL.
        let (service, executor) = reading(ReaderOwner::SignedOut, vec![post_json(1, 10)]).await;

        for post in [1, 99] {
            let outcome = service.post_opened(key(post)).await.unwrap();
            assert_eq!(outcome, ReaderPageViewOutcome::NotCountedForMissingDetails);
        }
        assert_eq!(executor.sent().len(), 1);
    }

    #[tokio::test]
    async fn a_view_that_cannot_be_sent_is_an_error_and_is_not_kept() {
        let (service, executor) = reading(ReaderOwner::SignedOut, vec![site_post(1, false)]).await;
        executor.push(Step::Offline);

        let result = service.post_opened(key(1)).await;

        assert!(matches!(result, Err(ReaderError::Api(_))));
        assert_eq!(service.pending_action_count().unwrap(), 0);
    }

    #[tokio::test]
    async fn the_sites_are_fetched_again_after_a_day_and_an_older_list_answers_if_that_fails() {
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
        executor.push(Step::Respond(
            200,
            json!({ "posts": [site_post(1, false)] }),
        ));
        service.stream(tag()).refresh().await.unwrap();
        service.set_member_sites(vec![WpComSiteId(123)]).unwrap();

        // Two days later the list is stale, and the device is offline.
        *now.lock().unwrap() += Duration::from_secs(2 * 24 * 60 * 60);
        executor.push(Step::Offline);
        let outcome = service.post_opened(key(1)).await.unwrap();
        assert_eq!(outcome, ReaderPageViewOutcome::NotCountedForSiteMember);

        // Back online, the list says the account has left the site.
        executor.push(sites(&[999]));
        executor.push(pixel());
        let outcome = service.post_opened(key(1)).await.unwrap();
        assert_eq!(outcome, ReaderPageViewOutcome::Counted);
    }

    #[tokio::test]
    async fn with_no_list_of_sites_a_failed_fetch_is_the_error() {
        let (service, executor) = reading(ACCOUNT, vec![site_post(1, false)]).await;
        executor.push(Step::Offline);

        let result = service.post_opened(key(1)).await;

        assert!(matches!(result, Err(ReaderError::Api(_))));
        assert!(matches!(
            service_for(ReaderOwner::SignedOut)
                .0
                .refresh_member_sites()
                .await,
            Err(ReaderError::NeedsAccount)
        ));
    }
}

mod posts {
    use super::*;
    use wp_api::wp_com::reader_saved::{ReaderFeedItemId, ReaderPostSaveIdentity};
    use wp_api::wp_com::sites::WpComSiteIdentifier;

    fn identity(id: u64) -> ReaderPostSaveIdentity {
        ReaderPostSaveIdentity {
            id,
            site_id: Some(WpComSiteId(123)),
            feed_id: None,
            feed_item_id: None,
            is_external: false,
            url: None,
        }
    }

    fn paths(executor: &ScriptedExecutor) -> Vec<String> {
        executor
            .sent()
            .iter()
            .map(|sent| {
                sent.url
                    .trim_start_matches("https://public-api.wordpress.com")
                    .to_string()
            })
            .collect()
    }

    #[tokio::test]
    async fn a_post_is_fetched_by_its_ids_and_kept() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        executor.push(Step::Respond(200, post_json(7, 10)));

        let post = service.fetch_post(identity(7)).await.unwrap();

        assert_eq!(post.title.as_deref(), Some("Post 7"));
        assert_eq!(
            post.content.as_deref(),
            Some("<p>The content of post 7.</p>")
        );
        assert_eq!(
            paths(&executor),
            ["/rest/v1.2/read/sites/123/posts/7?meta=site%2Cfeed"]
        );
        // It's in the cache, in no stream.
        assert_eq!(service.cached_post(key(7)).unwrap(), Some(post));
    }

    #[tokio::test]
    async fn an_rss_post_is_fetched_from_its_feed() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        executor.push(Step::Respond(
            200,
            json!({ "ID": 9, "feed_ID": 456, "feed_item_ID": 9, "is_external": true, "title": "From a feed" }),
        ));
        let rss = ReaderPostSaveIdentity {
            id: 9,
            site_id: None,
            feed_id: Some(ReaderFeedId(456)),
            feed_item_id: Some(ReaderFeedItemId(9)),
            is_external: true,
            url: None,
        };

        let post = service.fetch_post(rss).await.unwrap();

        assert_eq!(post.title.as_deref(), Some("From a feed"));
        assert_eq!(
            paths(&executor),
            ["/rest/v1.3/read/feed/456/posts/9?meta=site%2Cfeed"]
        );
    }

    #[tokio::test]
    async fn a_post_without_the_ids_to_fetch_it_by_is_an_error() {
        let (service, executor) = service();
        let post = ReaderPostSaveIdentity {
            id: 9,
            site_id: None,
            feed_id: None,
            feed_item_id: None,
            is_external: false,
            url: None,
        };

        assert!(matches!(
            service.fetch_post(post).await,
            Err(ReaderError::MissingIds)
        ));
        assert!(executor.sent().is_empty());
    }

    #[tokio::test]
    async fn a_fetched_post_keeps_what_the_reader_turned_on_since() {
        let (service, executor) = service();
        let mut post = post_json(7, 10);
        post["is_seen"] = json!(false);
        executor.push(Step::Respond(200, post.clone()));
        service.fetch_post(identity(7)).await.unwrap();
        // Marked as seen while offline.
        executor.push(Step::Offline);
        service.set_post_seen(identity(7), true).await.unwrap();

        // The server still says it isn't seen.
        executor.push(Step::Respond(200, post));
        let fetched = service.fetch_post(identity(7)).await.unwrap();

        assert_eq!(fetched.is_seen, Some(true));
    }

    #[tokio::test]
    async fn a_slug_finds_a_post_which_is_then_fetched_whole() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        executor.push(Step::Respond(
            200,
            json!({ "ID": 7, "site_ID": 123, "slug": "hello-world", "title": "Post 7" }),
        ));
        executor.push(Step::Respond(200, post_json(7, 10)));

        let post = service
            .fetch_post_by_slug(
                WpComSiteIdentifier::Slug {
                    value: "example.wordpress.com".to_string(),
                },
                "hello-world".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(post.identity.id, 7);
        assert_eq!(
            paths(&executor),
            [
                "/rest/v1.1/sites/example.wordpress.com/posts/slug:hello-world?",
                "/rest/v1.2/read/sites/123/posts/7?meta=site%2Cfeed",
            ]
        );
    }

    #[tokio::test]
    async fn an_address_is_resolved_to_a_post_for_an_account_only() {
        let (signed_out, executor) = service_for(ReaderOwner::SignedOut);
        let url = "https://example.wordpress.com/2026/11/18/hello-world/".to_string();
        assert!(matches!(
            signed_out.fetch_post_at_url(url.clone()).await,
            Err(ReaderError::NeedsAccount)
        ));
        assert!(executor.sent().is_empty());

        let (service, executor) = service();
        executor.push(Step::Respond(200, json!({ "site_id": 123, "post_id": 7 })));
        executor.push(Step::Respond(200, post_json(7, 10)));

        let post = service.fetch_post_at_url(url).await.unwrap();

        assert_eq!(post.identity.id, 7);
        assert!(paths(&executor)[0].starts_with("/wpcom/v2/mobile/resolve-reader-url?url=https"));
    }

    #[tokio::test]
    async fn related_posts_are_a_stream_of_one_page() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        let stream = service.stream(ReaderStream::Related {
            site_id: WpComSiteId(123),
            post_id: PostId(7),
        });
        executor.push(Step::Respond(200, page(&[(8, 10), (9, 11)])));

        let result = stream.refresh().await.unwrap();

        assert!(!result.has_more);
        assert_eq!(shown(&stream).await, ["8", "9"]);
        assert_eq!(
            paths(&executor),
            ["/rest/v1.2/read/site/123/post/7/related?size_global=3&size_local=3&meta=site%2Cfeed"]
        );
    }

    /// A service with a clock the test moves, whose Following stream had post 2 unreadable.
    async fn with_an_unreadable_post() -> (
        ReaderService,
        Arc<ScriptedExecutor>,
        Arc<ReaderStreamCollection>,
        Arc<Mutex<SystemTime>>,
    ) {
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
        let stream = service.stream(ReaderStream::Following);
        let mut unreadable = post_json(2, 11);
        unreadable["date"] = json!("2026-11-18T11:00:00+91:00");
        executor.push(Step::Respond(
            200,
            json!({ "posts": [post_json(3, 12), unreadable, post_json(1, 10)] }),
        ));
        stream.refresh().await.unwrap();
        (service, executor, stream, now)
    }

    #[tokio::test]
    async fn an_unreadable_post_is_asked_for_again_after_an_hour_and_takes_its_place() {
        let (_service, executor, stream, now) = with_an_unreadable_post().await;
        // Straight away, nothing is asked for: the server would send the same post.
        assert_eq!(stream.retry_unreadable_posts().await.unwrap(), 0);
        assert_eq!(executor.sent().len(), 1);

        *now.lock().unwrap() += Duration::from_secs(61 * 60);
        executor.push(Step::Respond(200, post_json(2, 11)));
        assert_eq!(stream.retry_unreadable_posts().await.unwrap(), 1);

        assert_eq!(shown(&stream).await, ["3", "2", "1"]);
        assert_eq!(stream.info().unwrap().unreadable_post_count, 0);
        assert_eq!(
            paths(&executor)[1],
            "/rest/v1.2/read/sites/123/posts/2?meta=site%2Cfeed"
        );
    }

    #[tokio::test]
    async fn an_unreadable_post_that_still_cannot_be_read_is_tried_five_times() {
        let (_service, executor, stream, now) = with_an_unreadable_post().await;
        let mut still_unreadable = post_json(2, 11);
        still_unreadable["date"] = json!("2026-11-18T11:00:00+91:00");

        for attempt in 1..=6 {
            *now.lock().unwrap() += Duration::from_secs(61 * 60);
            if attempt <= 5 {
                executor.push(Step::Respond(200, still_unreadable.clone()));
            }
            assert_eq!(stream.retry_unreadable_posts().await.unwrap(), 0);
        }

        // Five requests for the post, and none the sixth time.
        assert_eq!(executor.sent().len(), 6);
        assert_eq!(stream.info().unwrap().unreadable_post_count, 1);
    }

    #[tokio::test]
    async fn an_unreadable_post_the_server_no_longer_has_is_forgotten() {
        let (_service, executor, stream, now) = with_an_unreadable_post().await;
        *now.lock().unwrap() += Duration::from_secs(61 * 60);
        executor.push(Step::Respond(
            404,
            json!({ "error": "unknown_post", "message": "Unknown post" }),
        ));

        assert_eq!(stream.retry_unreadable_posts().await.unwrap(), 0);

        assert_eq!(stream.info().unwrap().unreadable_post_count, 0);
        assert_eq!(shown(&stream).await, ["3", "1"]);
    }

    #[tokio::test]
    async fn offline_an_unreadable_post_is_not_counted_as_tried() {
        let (_service, executor, stream, now) = with_an_unreadable_post().await;
        for _ in 0..6 {
            *now.lock().unwrap() += Duration::from_secs(61 * 60);
            executor.push(Step::Offline);
            assert_eq!(stream.retry_unreadable_posts().await.unwrap(), 0);
        }

        // Back online, it's still asked for.
        *now.lock().unwrap() += Duration::from_secs(61 * 60);
        executor.push(Step::Respond(200, post_json(2, 11)));
        assert_eq!(stream.retry_unreadable_posts().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn a_refresh_asks_again_for_unreadable_posts_that_are_due() {
        let (_service, executor, stream, now) = with_an_unreadable_post().await;
        *now.lock().unwrap() += Duration::from_secs(61 * 60);
        // The page still can't read post 2, and then the post on its own can be.
        let mut unreadable = post_json(2, 11);
        unreadable["date"] = json!("2026-11-18T11:00:00+91:00");
        executor.push(Step::Respond(
            200,
            json!({ "posts": [post_json(3, 12), unreadable, post_json(1, 10)] }),
        ));
        executor.push(Step::Respond(200, post_json(2, 11)));

        stream.refresh().await.unwrap();

        assert_eq!(shown(&stream).await, ["3", "2", "1"]);
        assert_eq!(stream.info().unwrap().unreadable_post_count, 0);
    }
}

mod menu_and_sites {
    use super::*;
    use wp_api::wp_com::reader_tags::ReaderMenuItemKind;

    fn fixture(json: &str) -> Value {
        serde_json::from_str(json).expect("the fixture is JSON")
    }

    fn menu() -> Value {
        fixture(include_str!(
            "../../../../wp_api/tests/wpcom/reader_tags/menu.json"
        ))
    }

    fn signed_out_menu() -> Value {
        fixture(include_str!(
            "../../../../wp_api/tests/wpcom/reader_tags/menu_signed_out.json"
        ))
    }

    #[tokio::test]
    async fn the_menu_is_fetched_and_kept() {
        let (service, executor) = service();
        assert!(service.menu().unwrap().items.is_empty());
        executor.push(Step::Respond(200, menu()));

        service.refresh_menu().await.unwrap();

        let cached = service.menu().unwrap();
        // The scripted answer is sent with its keys sorted, so the order here is by key. The cache
        // keeps whatever order the answer has.
        let keys: Vec<_> = cached.items.iter().map(|item| item.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "a8c",
                "discover",
                "following",
                "liked",
                "list-example-list",
                "p2"
            ]
        );
        assert_eq!(cached.items[0].kind, ReaderMenuItemKind::Organization);
        assert_eq!(cached.followed_tags.len(), 2);
        assert_eq!(cached.recommended_tags.len(), 3);
        assert_eq!(service.followed_tags().unwrap(), cached.followed_tags);
        assert_eq!(
            executor.sent()[0].url,
            "https://public-api.wordpress.com/rest/v1.3/read/menu"
        );

        // A second service on the same cache has it without asking.
        assert_eq!(service.menu().unwrap(), cached);
    }

    #[tokio::test]
    async fn signed_out_the_menu_has_the_default_streams_and_recommended_tags() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        executor.push(Step::Respond(200, signed_out_menu()));

        service.refresh_menu().await.unwrap();

        let cached = service.menu().unwrap();
        assert_eq!(cached.items.len(), 1);
        assert_eq!(cached.items[0].key, "discover");
        assert!(cached.followed_tags.is_empty());
        assert!(!cached.recommended_tags.is_empty());
    }

    #[tokio::test]
    async fn interests_and_a_tags_info_are_fetched_on_request() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        executor.push(Step::Respond(
            200,
            fixture(include_str!(
                "../../../../wp_api/tests/wpcom/reader_tags/interests.json"
            )),
        ));
        executor.push(Step::Respond(
            200,
            fixture(include_str!(
                "../../../../wp_api/tests/wpcom/reader_tags/tag.json"
            )),
        ));

        let interests = service.interests().await.unwrap();
        let tag = service
            .tag_info(ReaderTagSlug("photography".to_string()))
            .await
            .unwrap();

        assert!(!interests.is_empty());
        assert!(tag.title.is_some());
        let sent = executor.sent();
        assert!(sent[0].url.ends_with("/wpcom/v2/read/interests"));
        assert!(sent[1].url.ends_with("/rest/v1.2/read/tags/photography"));
    }

    #[tokio::test]
    async fn a_site_is_fetched_and_kept() {
        let (service, executor) = service();
        let site = fixture(include_str!(
            "../../../../wp_api/tests/wpcom/reader_sites/site_followed.json"
        ));
        let site_id = WpComSiteId(site["ID"].as_u64().unwrap());
        executor.push(Step::Respond(200, site.clone()));

        let fetched = service.fetch_site(site_id).await.unwrap();

        assert_eq!(fetched.site_id, Some(site_id));
        assert_eq!(fetched.name.as_deref(), site["name"].as_str());
        assert_eq!(fetched.is_following, Some(true));
        assert_eq!(
            service.cached_site(Some(site_id), None).unwrap(),
            Some(fetched)
        );
        assert_eq!(
            executor.sent()[0].url,
            format!(
                "https://public-api.wordpress.com/rest/v1.2/read/sites/{}?",
                site_id.0
            )
        );
        assert_eq!(
            service.cached_site(Some(WpComSiteId(1)), None).unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn a_feed_is_fetched_and_kept_by_its_feed_id() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        let feed = fixture(include_str!(
            "../../../../wp_api/tests/wpcom/reader_sites/feed.json"
        ));
        let feed_id = ReaderFeedId(
            feed["feed_ID"]
                .as_str()
                .map(|id| id.parse().unwrap())
                .or(feed["feed_ID"].as_u64())
                .unwrap(),
        );
        executor.push(Step::Respond(200, feed));

        let fetched = service.fetch_feed(feed_id).await.unwrap();

        assert_eq!(fetched.feed_id, Some(feed_id));
        assert_eq!(
            service.cached_site(fetched.site_id, Some(feed_id)).unwrap(),
            Some(fetched)
        );
    }

    #[tokio::test]
    async fn the_subscriptions_are_fetched_a_page_at_a_time_and_kept() {
        let (service, executor) = service();
        let page = fixture(include_str!(
            "../../../../wp_api/tests/wpcom/reader_subscriptions/subscriptions.json"
        ));
        let listed = page["subscriptions"].as_array().unwrap().len();
        // The first page says there are more subscriptions than it holds, and the second is short.
        let mut first = page.clone();
        first["page"] = json!(1);
        first["total_subscriptions"] = json!(150);
        let mut second = page;
        second["page"] = json!(2);
        second["total_subscriptions"] = json!(150);
        second["subscriptions"] = json!([]);
        executor.push(Step::Respond(200, first));
        executor.push(Step::Respond(200, second));

        let count = service.refresh_subscriptions().await.unwrap();

        assert_eq!(count as usize, listed);
        let cached = service.subscriptions().unwrap();
        assert_eq!(cached.len(), listed);
        assert!(
            cached
                .iter()
                .all(|subscription| subscription.subscription_id.is_some())
        );
        assert_eq!(
            queries(&executor),
            ["page=1&number=100", "page=2&number=100"]
        );
    }

    #[tokio::test]
    async fn an_accounts_lists_need_an_account() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        assert!(matches!(
            service.refresh_subscriptions().await,
            Err(ReaderError::NeedsAccount)
        ));
        assert!(matches!(
            service
                .user_profile(wp_api::wp_com::reader_users::WpComUserIdentifier::Login {
                    value: "someone".to_string()
                })
                .await,
            Err(ReaderError::NeedsAccount)
        ));
        assert!(executor.sent().is_empty());
        assert!(service.subscriptions().unwrap().is_empty());
    }

    #[tokio::test]
    async fn sites_are_searched_a_page_at_a_time() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        executor.push(Step::Respond(
            200,
            fixture(include_str!(
                "../../../../wp_api/tests/wpcom/reader_sites/feed_search.json"
            )),
        ));

        let results = service
            .search_sites("photography".to_string(), Some(20))
            .await
            .unwrap();

        assert!(!results.feeds.is_empty());
        assert_eq!(
            queries(&executor),
            ["q=photography&offset=20&meta=site%2Cfeed"]
        );
    }
}

mod comments {
    use super::*;
    use wp_api::comments::CommentId;

    const SITE: WpComSiteId = WpComSiteId(123);
    const POST: PostId = PostId(7);

    pub(super) fn comment_json(id: i64, hour: u32, parent: Option<i64>) -> Value {
        json!({
            "ID": id,
            "post": { "ID": 7, "type": "post" },
            "author": { "ID": 9, "name": "A commenter", "avatar_URL": "https://example.com/a.png" },
            "date": date(hour),
            "content": format!("<p>Comment {id}</p>"),
            "status": "approved",
            "parent": match parent {
                Some(parent) => json!({ "ID": parent, "type": "comment" }),
                None => json!(false),
            },
            "type": "comment",
            "like_count": 0,
            "i_like": false,
        })
    }

    pub(super) fn comments_page(found: u64, comments: Vec<Value>) -> Value {
        json!({ "found": found, "site_ID": 123, "comments": comments })
    }

    pub(super) async fn ids(collection: &ReaderCommentsCollection) -> Vec<String> {
        collection
            .load_items()
            .await
            .unwrap()
            .iter()
            .map(|comment| match (&comment.comment_id, &comment.local_id) {
                (Some(id), _) => id.0.to_string(),
                (None, _) => "local".to_string(),
            })
            .collect()
    }

    #[tokio::test]
    async fn a_posts_comments_are_fetched_newest_first_and_shown_oldest_first() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        let comments = service.comments(SITE, POST);
        assert!(ids(&comments).await.is_empty());
        assert_eq!(comments.info().unwrap().found, None);
        executor.push(Step::Respond(
            200,
            comments_page(
                2,
                vec![comment_json(2, 11, Some(1)), comment_json(1, 10, None)],
            ),
        ));

        let result = comments.refresh().await.unwrap();

        assert_eq!(
            result,
            ReaderCommentsPageResult {
                comment_count: 2,
                has_more: false,
            }
        );
        assert_eq!(ids(&comments).await, ["1", "2"]);
        let items = comments.load_items().await.unwrap();
        assert_eq!(items[1].parent_id, Some(CommentId(1)));
        assert_eq!(items[0].author_name.as_deref(), Some("A commenter"));
        let info = comments.info().unwrap();
        assert_eq!(info.found, Some(2));
        assert!(!info.has_more);
        assert_eq!(info.state, ListState::Idle);
        assert_eq!(
            executor.sent()[0].url,
            "https://public-api.wordpress.com/rest/v1.1/sites/123/posts/7/replies?number=100&order=DESC&force=wpcom"
        );
    }

    #[tokio::test]
    async fn older_comments_are_loaded_until_the_cache_has_them_all() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        let comments = service.comments(SITE, POST);
        executor.push(Step::Respond(
            200,
            comments_page(
                3,
                vec![comment_json(3, 12, None), comment_json(2, 11, None)],
            ),
        ));
        let result = comments.refresh().await.unwrap();
        assert!(result.has_more);

        executor.push(Step::Respond(
            200,
            comments_page(3, vec![comment_json(1, 10, None)]),
        ));
        let result = comments.load_more().await.unwrap();
        assert!(!result.has_more);
        assert_eq!(ids(&comments).await, ["1", "2", "3"]);
        assert_eq!(
            queries(&executor)[1],
            "number=100&offset=2&order=DESC&force=wpcom"
        );

        // With them all, nothing more is asked for.
        comments.load_more().await.unwrap();
        assert_eq!(executor.sent().len(), 2);
    }

    #[tokio::test]
    async fn a_page_past_the_end_with_nothing_on_it_ends_the_comments() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        let comments = service.comments(SITE, POST);
        // The server says there are five, and only ever returns two.
        executor.push(Step::Respond(
            200,
            comments_page(
                5,
                vec![comment_json(2, 11, None), comment_json(1, 10, None)],
            ),
        ));
        comments.refresh().await.unwrap();
        executor.push(Step::Respond(200, comments_page(5, vec![])));

        let result = comments.load_more().await.unwrap();

        assert!(!result.has_more);
        assert!(!comments.info().unwrap().has_more);
    }

    #[tokio::test]
    async fn a_deleted_comment_goes_when_the_page_has_every_comment() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        let comments = service.comments(SITE, POST);
        executor.push(Step::Respond(
            200,
            comments_page(
                2,
                vec![comment_json(2, 11, None), comment_json(1, 10, None)],
            ),
        ));
        comments.refresh().await.unwrap();
        executor.push(Step::Respond(
            200,
            comments_page(1, vec![comment_json(2, 11, None)]),
        ));

        comments.refresh().await.unwrap();

        assert_eq!(ids(&comments).await, ["2"]);
    }

    #[tokio::test]
    async fn load_more_on_comments_that_have_not_loaded_fetches_the_newest() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        let comments = service.comments(SITE, POST);
        executor.push(Step::Respond(
            200,
            comments_page(1, vec![comment_json(1, 10, None)]),
        ));

        comments.load_more().await.unwrap();

        assert_eq!(queries(&executor), ["number=100&order=DESC&force=wpcom"]);
        assert_eq!(ids(&comments).await, ["1"]);
    }

    #[tokio::test]
    async fn a_failed_fetch_is_recorded_and_keeps_the_cached_comments() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        let comments = service.comments(SITE, POST);
        executor.push(Step::Respond(
            200,
            comments_page(1, vec![comment_json(1, 10, None)]),
        ));
        comments.refresh().await.unwrap();
        executor.push(Step::Offline);

        assert!(matches!(comments.refresh().await, Err(ReaderError::Api(_))));

        let info = comments.info().unwrap();
        assert_eq!(info.state, ListState::Error);
        assert!(info.error_message.is_some());
        assert_eq!(ids(&comments).await, ["1"]);
    }

    #[tokio::test]
    async fn each_posts_comments_are_its_own() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        executor.push(Step::Respond(
            200,
            comments_page(1, vec![comment_json(1, 10, None)]),
        ));
        service.comments(SITE, POST).refresh().await.unwrap();

        assert!(ids(&service.comments(SITE, PostId(8))).await.is_empty());
        assert_eq!(ids(&service.comments(SITE, POST)).await, ["1"]);
    }
}

mod more_actions {
    use super::*;
    use wp_api::comments::CommentId;
    use wp_api::wp_com::reader_saved::ReaderPostSaveIdentity;
    use wp_api::wp_com::reader_subscriptions::ReaderSubscriptionEmailFrequency;

    const SITE: WpComSiteId = WpComSiteId(123);

    fn identity(id: u64) -> ReaderPostSaveIdentity {
        ReaderPostSaveIdentity {
            id,
            site_id: Some(SITE),
            feed_id: None,
            feed_item_id: None,
            is_external: false,
            url: None,
        }
    }

    /// A service whose Following stream has post 1 of site 123, with five likes, not followed.
    async fn reading() -> (
        ReaderService,
        Arc<ScriptedExecutor>,
        Arc<ReaderStreamCollection>,
    ) {
        let (service, executor) = service();
        let stream = service.stream(ReaderStream::Following);
        let mut post = post_json(1, 10);
        post["like_count"] = json!(5);
        post["i_like"] = json!(false);
        post["is_following"] = json!(false);
        post["site_name"] = json!("A site");
        post["site_URL"] = json!("https://site.example");
        executor.push(Step::Respond(200, json!({ "posts": [post] })));
        stream.refresh().await.unwrap();
        (service, executor, stream)
    }

    async fn post(stream: &ReaderStreamCollection) -> ReaderPostRecord {
        stream.load_items().await.unwrap()[0].post.clone()
    }

    fn sent_after(executor: &ScriptedExecutor, skip: usize) -> Vec<(String, Value)> {
        executor
            .sent()
            .iter()
            .skip(skip)
            .map(|sent| {
                (
                    sent.url
                        .trim_start_matches("https://public-api.wordpress.com")
                        .to_string(),
                    sent.body.clone().unwrap_or(Value::Null),
                )
            })
            .collect()
    }

    fn refused(code: &str) -> Step {
        Step::Respond(400, json!({ "error": code, "message": "" }))
    }

    fn target() -> ReaderFollowTarget {
        ReaderFollowTarget {
            site_id: Some(SITE),
            feed_id: None,
            url: Some("https://site.example".to_string()),
            name: Some("A site".to_string()),
        }
    }

    fn subscription_json() -> Value {
        let page: Value = serde_json::from_str(include_str!(
            "../../../../wp_api/tests/wpcom/reader_subscriptions/subscriptions.json"
        ))
        .unwrap();
        let mut subscription = page["subscriptions"][0].clone();
        subscription["blog_ID"] = json!("123");
        subscription["URL"] = json!("https://site.example");
        subscription
    }

    #[tokio::test]
    async fn a_like_moves_the_count_at_once_and_takes_the_servers_when_it_answers() {
        let (service, executor, stream) = reading().await;
        let (release, held) = oneshot::channel();
        executor.push(Step::Hold(
            held,
            200,
            json!({ "success": true, "i_like": true, "like_count": 8 }),
        ));

        let (outcome, ()) = futures::join!(service.set_post_liked(identity(1), true), async {
            let post = post(&stream).await;
            assert_eq!((post.i_like, post.like_count), (Some(true), Some(6)));
            release.send(()).unwrap();
        });

        assert_eq!(outcome.unwrap(), ReaderActionOutcome::Synced);
        // Others liked it meanwhile, and the server's count says so.
        let post = post(&stream).await;
        assert_eq!((post.i_like, post.like_count), (Some(true), Some(8)));
        assert_eq!(
            sent_after(&executor, 1)[0].0,
            "/rest/v1.1/sites/123/posts/1/likes/new"
        );
    }

    #[tokio::test]
    async fn a_refused_like_puts_the_count_back() {
        let (service, executor, stream) = reading().await;
        executor.push(refused("like_failed"));

        let result = service.set_post_liked(identity(1), true).await;

        assert!(matches!(result, Err(ReaderError::Api(_))));
        let post = post(&stream).await;
        assert_eq!((post.i_like, post.like_count), (Some(false), Some(5)));
    }

    #[tokio::test]
    async fn an_unlike_is_sent_to_its_own_endpoint_and_a_refresh_keeps_a_like_that_is_waiting() {
        let (service, executor, stream) = reading().await;
        executor.push(Step::Offline);
        service.set_post_liked(identity(1), true).await.unwrap();

        // The server's page doesn't have the like yet.
        let mut unliked = post_json(1, 10);
        unliked["like_count"] = json!(5);
        unliked["i_like"] = json!(false);
        executor.push(Step::Respond(200, json!({ "posts": [unliked] })));
        stream.refresh().await.unwrap();
        let shown = post(&stream).await;
        assert_eq!((shown.i_like, shown.like_count), (Some(true), Some(6)));

        executor.push(Step::Offline);
        service.set_post_liked(identity(1), false).await.unwrap();
        executor.push(Step::Respond(
            200,
            json!({ "success": true, "i_like": false, "like_count": 5 }),
        ));
        assert_eq!(service.sync_actions().await.unwrap(), 0);
        assert_eq!(
            sent_after(&executor, 4)[0].0,
            "/rest/v1.1/sites/123/posts/1/likes/mine/delete"
        );
    }

    #[tokio::test]
    async fn an_rss_post_cannot_be_liked() {
        let (service, executor) = service();
        let rss = ReaderPostSaveIdentity {
            id: 9,
            site_id: None,
            feed_id: Some(ReaderFeedId(456)),
            feed_item_id: None,
            is_external: true,
            url: None,
        };

        let result = service.set_post_liked(rss, true).await;

        assert!(matches!(result, Err(ReaderError::MissingIds)));
        assert!(executor.sent().is_empty());
        assert_eq!(service.pending_action_count().unwrap(), 0);
    }

    #[tokio::test]
    async fn a_follow_shows_on_the_sites_posts_and_in_the_subscriptions() {
        let (service, executor, stream) = reading().await;
        let (release, held) = oneshot::channel();
        executor.push(Step::Hold(
            held,
            200,
            json!({ "subscribed": true, "subscription": subscription_json() }),
        ));

        let (outcome, ()) = futures::join!(service.set_followed(target(), true), async {
            assert_eq!(post(&stream).await.is_following, Some(true));
            // A stand-in is listed until the server answers.
            let listed = service.subscriptions().unwrap();
            assert_eq!(listed.len(), 1);
            assert_eq!(listed[0].subscription_id, None);
            assert_eq!(listed[0].name.as_deref(), Some("A site"));
            release.send(()).unwrap();
        });

        assert_eq!(outcome.unwrap(), ReaderActionOutcome::Synced);
        let listed = service.subscriptions().unwrap();
        assert_eq!(listed.len(), 1);
        assert!(listed[0].subscription_id.is_some());
        assert_eq!(
            sent_after(&executor, 1),
            [(
                "/rest/v1.1/read/following/mine/new".to_string(),
                json!({ "url": "https://site.example", "source": "reader-ios" })
            )]
        );

        // An unfollow goes by the site's ID.
        executor.push(Step::Respond(
            200,
            json!({ "success": true, "is_following": false }),
        ));
        service.set_followed(target(), false).await.unwrap();
        assert_eq!(post(&stream).await.is_following, Some(false));
        assert!(service.subscriptions().unwrap().is_empty());
        assert_eq!(
            sent_after(&executor, 2)[0].0,
            "/rest/v1.1/sites/123/follows/mine/delete"
        );
    }

    #[tokio::test]
    async fn an_address_that_cannot_be_followed_is_refused_and_put_back() {
        let (service, executor, stream) = reading().await;
        executor.push(Step::Respond(
            200,
            json!({ "info": "invalid_feed", "subscribed": false }),
        ));

        let result = service.set_followed(target(), true).await;

        assert!(
            matches!(&result, Err(ReaderError::Refused { reason }) if reason == "invalid_feed"),
            "{result:?}"
        );
        assert_eq!(post(&stream).await.is_following, Some(false));
        assert!(service.subscriptions().unwrap().is_empty());
    }

    #[tokio::test]
    async fn unfollowing_what_is_not_followed_leaves_it_unfollowed() {
        let (service, executor, stream) = reading().await;
        executor.push(refused("are_not_subscribed"));

        let outcome = service.set_followed(target(), false).await.unwrap();

        assert_eq!(outcome, ReaderActionOutcome::Synced);
        assert_eq!(post(&stream).await.is_following, Some(false));
    }

    #[tokio::test]
    async fn a_feed_is_followed_and_unfollowed_by_its_address() {
        let (service, executor) = service();
        let feed = ReaderFollowTarget {
            site_id: None,
            feed_id: Some(ReaderFeedId(456)),
            url: Some("https://feed.example/rss".to_string()),
            name: None,
        };
        executor.push(Step::Respond(200, json!({ "subscribed": true })));
        executor.push(Step::Respond(200, json!({ "subscribed": false })));

        service.set_followed(feed.clone(), true).await.unwrap();
        service.set_followed(feed, false).await.unwrap();

        assert_eq!(
            sent_after(&executor, 0),
            [
                (
                    "/rest/v1.1/read/following/mine/new".to_string(),
                    json!({ "url": "https://feed.example/rss", "source": "reader-ios" })
                ),
                (
                    "/rest/v1.1/read/following/mine/delete".to_string(),
                    json!({ "url": "https://feed.example/rss" })
                ),
            ]
        );
    }

    #[tokio::test]
    async fn a_follow_with_no_address_to_send_is_refused() {
        let (service, executor) = service();
        let unknown = ReaderFollowTarget {
            site_id: Some(WpComSiteId(999)),
            feed_id: None,
            url: None,
            name: None,
        };

        let result = service.set_followed(unknown, true).await;

        assert!(matches!(result, Err(ReaderError::MissingIds)));
        assert!(executor.sent().is_empty());
        assert!(service.subscriptions().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_refresh_of_the_subscriptions_keeps_a_follow_that_is_waiting() {
        let (service, executor, _) = reading().await;
        executor.push(Step::Offline);
        service.set_followed(target(), true).await.unwrap();
        // The server's list doesn't have it yet.
        executor.push(Step::Respond(
            200,
            json!({ "subscriptions": [], "page": 1, "number": 100, "total_subscriptions": 0 }),
        ));

        service.refresh_subscriptions().await.unwrap();

        let listed = service.subscriptions().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].site_id, Some(SITE));
    }

    #[tokio::test]
    async fn an_account_follows_a_tag_through_the_server() {
        let (service, executor) = service();
        let slug = ReaderTagSlug("art".to_string());
        executor.push(Step::Respond(
            200,
            json!({ "subscribed": true, "tags": [], "added_tags": [] }),
        ));

        let outcome = service
            .set_tag_followed(slug.clone(), Some("Art".to_string()), true)
            .await
            .unwrap();

        assert_eq!(outcome, ReaderActionOutcome::Synced);
        let followed = service.followed_tags().unwrap();
        assert_eq!(followed.len(), 1);
        assert_eq!(followed[0].title.as_deref(), Some("Art"));
        assert_eq!(
            sent_after(&executor, 0),
            [(
                "/rest/v1.2/read/tags/mine/new".to_string(),
                json!({ "tags": ["art"] })
            )]
        );

        // The server says it wasn't followed, which leaves it unfollowed.
        executor.push(refused("not_subscribed"));
        let outcome = service.set_tag_followed(slug, None, false).await.unwrap();
        assert_eq!(outcome, ReaderActionOutcome::Synced);
        assert!(service.followed_tags().unwrap().is_empty());
        assert_eq!(
            sent_after(&executor, 1)[0].0,
            "/rest/v1.1/read/tags/art/mine/delete"
        );
    }

    #[tokio::test]
    async fn a_menu_refresh_keeps_a_tag_follow_that_is_waiting() {
        let (service, executor) = service();
        executor.push(Step::Offline);
        service
            .set_tag_followed(
                ReaderTagSlug("art".to_string()),
                Some("Art".to_string()),
                true,
            )
            .await
            .unwrap();
        // The server's menu has no followed tags.
        executor.push(Step::Respond(
            200,
            json!({ "default": {}, "subscribed": [], "recommended": {} }),
        ));

        service.refresh_menu().await.unwrap();

        assert_eq!(service.followed_tags().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn signed_out_tags_are_followed_on_the_device_and_an_account_takes_them_over() {
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
        let signed_out = new(ReaderOwner::SignedOut);
        for slug in ["art", "travel"] {
            let outcome = signed_out
                .set_tag_followed(ReaderTagSlug(slug.to_string()), None, true)
                .await
                .unwrap();
            assert!(matches!(outcome, ReaderActionOutcome::Pending { .. }));
        }
        signed_out
            .set_tag_followed(ReaderTagSlug("travel".to_string()), None, false)
            .await
            .unwrap();
        assert_eq!(signed_out.followed_tags().unwrap().len(), 1);
        assert!(executor.sent().is_empty());

        let account = new(ACCOUNT);
        executor.push(Step::Respond(
            200,
            json!({ "subscribed": true, "tags": [], "added_tags": [] }),
        ));
        let adopted = account.adopt_signed_out_tags().await.unwrap();

        assert_eq!(adopted, 1);
        assert_eq!(account.followed_tags().unwrap()[0].key, "art");
        assert!(signed_out.followed_tags().unwrap().is_empty());
        assert_eq!(executor.sent().len(), 1);
        // There's nothing left to take over.
        assert_eq!(account.adopt_signed_out_tags().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn a_comment_like_moves_its_count() {
        let (service, executor) = service();
        let comments = service.comments(SITE, PostId(7));
        executor.push(Step::Respond(
            200,
            super::comments::comments_page(1, vec![super::comments::comment_json(4, 10, None)]),
        ));
        comments.refresh().await.unwrap();
        executor.push(Step::Respond(
            200,
            json!({ "success": true, "i_like": true, "like_count": 3 }),
        ));

        let outcome = service
            .set_comment_liked(SITE, CommentId(4), true)
            .await
            .unwrap();

        assert_eq!(outcome, ReaderActionOutcome::Synced);
        let comment = comments.load_items().await.unwrap()[0].clone();
        assert_eq!((comment.i_like, comment.like_count), (Some(true), Some(3)));
        assert_eq!(
            sent_after(&executor, 1)[0].0,
            "/rest/v1.1/sites/123/comments/4/likes/new"
        );

        // A refused unlike puts the like back.
        executor.push(refused("comment_like_failed"));
        let result = service.set_comment_liked(SITE, CommentId(4), false).await;
        assert!(matches!(result, Err(ReaderError::Api(_))));
        let comment = comments.load_items().await.unwrap()[0].clone();
        assert_eq!((comment.i_like, comment.like_count), (Some(true), Some(3)));
    }

    #[tokio::test]
    async fn a_followed_sites_delivery_settings_change() {
        let (service, executor, _) = reading().await;
        executor.push(Step::Respond(
            200,
            json!({ "subscribed": true, "subscription": subscription_json() }),
        ));
        service.set_followed(target(), true).await.unwrap();
        let settings = |service: &ReaderService| service.subscriptions().unwrap()[0].clone();
        let answered = |subscribed: bool| {
            Step::Respond(200, json!({ "success": true, "subscribed": subscribed }))
        };

        executor.push(answered(true));
        service.set_post_emails(SITE, true).await.unwrap();
        executor.push(answered(true));
        service.set_comment_emails(SITE, true).await.unwrap();
        executor.push(answered(true));
        service.set_post_notifications(SITE, true).await.unwrap();
        executor.push(answered(false));
        service.set_post_emails(SITE, false).await.unwrap();
        executor.push(answered(true));
        service
            .set_post_email_frequency(SITE, ReaderSubscriptionEmailFrequency::Weekly)
            .await
            .unwrap();

        let stored = settings(&service);
        assert_eq!(stored.email_posts, Some(false));
        assert_eq!(stored.email_comments, Some(true));
        assert_eq!(stored.notify_posts, Some(true));
        assert_eq!(
            stored.email_frequency,
            Some(ReaderSubscriptionEmailFrequency::Weekly)
        );
        let paths: Vec<_> = sent_after(&executor, 2)
            .into_iter()
            .map(|(path, _)| path)
            .collect();
        assert_eq!(
            paths,
            [
                "/rest/v1.2/read/site/123/post_email_subscriptions/new",
                "/rest/v1.2/read/site/123/comment_email_subscriptions/new",
                "/wpcom/v2/read/sites/123/notification-subscriptions/new",
                "/rest/v1.2/read/site/123/post_email_subscriptions/delete",
                "/rest/v1.2/read/site/123/post_email_subscriptions/update",
            ]
        );
    }

    #[tokio::test]
    async fn the_frequency_of_emails_is_not_kept_to_send_later() {
        let (service, executor, _) = reading().await;
        executor.push(Step::Offline);

        let result = service
            .set_post_email_frequency(SITE, ReaderSubscriptionEmailFrequency::Daily)
            .await;

        assert!(matches!(result, Err(ReaderError::Api(_))));
        assert_eq!(service.pending_action_count().unwrap(), 0);
        let (signed_out, _) = service_for(ReaderOwner::SignedOut);
        assert!(matches!(
            signed_out
                .set_post_email_frequency(SITE, ReaderSubscriptionEmailFrequency::Daily)
                .await,
            Err(ReaderError::NeedsAccount)
        ));
    }
}

mod comment_writing {
    use super::comments::{comment_json, comments_page, ids};
    use super::*;
    use wp_api::comments::CommentId;
    use wp_api::wp_com::reader_saved::ReaderPostSaveIdentity;
    use wp_mobile_cache::repository::reader_comments::ReaderCommentSendState;

    const SITE: WpComSiteId = WpComSiteId(123);
    const POST: PostId = PostId(7);

    /// A service whose post 7 has comment 1.
    async fn with_comments() -> (
        ReaderService,
        Arc<ScriptedExecutor>,
        Arc<ReaderCommentsCollection>,
    ) {
        let (service, executor) = service();
        let comments = service.comments(SITE, POST);
        executor.push(Step::Respond(
            200,
            comments_page(1, vec![comment_json(1, 10, None)]),
        ));
        comments.refresh().await.unwrap();
        (service, executor, comments)
    }

    fn body_and_path(executor: &ScriptedExecutor) -> (String, Value) {
        let sent = executor.sent();
        let last = sent.last().unwrap();
        (
            last.url
                .trim_start_matches("https://public-api.wordpress.com")
                .to_string(),
            last.body.clone().unwrap_or(Value::Null),
        )
    }

    #[tokio::test]
    async fn a_comment_shows_while_it_is_sent_and_becomes_the_servers() {
        let (_service, executor, comments) = with_comments().await;
        let (release, held) = oneshot::channel();
        executor.push(Step::Hold(held, 200, comment_json(2, 23, None)));

        let (posted, ()) = futures::join!(
            comments.post_comment("Nice post!".to_string(), None),
            async {
                let items = comments.load_items().await.unwrap();
                let written = items.last().unwrap();
                assert_eq!(written.send_state, ReaderCommentSendState::Sending);
                assert_eq!(written.content.as_deref(), Some("Nice post!"));
                assert!(written.local_id.is_some());
                release.send(()).unwrap();
            }
        );

        let posted = posted.unwrap();
        assert_eq!(posted.comment_id, Some(CommentId(2)));
        assert_eq!(posted.send_state, ReaderCommentSendState::Sent);
        assert_eq!(ids(&comments).await, ["1", "2"]);
        assert_eq!(comments.info().unwrap().found, Some(2));
        assert_eq!(
            body_and_path(&executor),
            (
                "/rest/v1.1/sites/123/posts/7/replies/new".to_string(),
                json!({ "content": "Nice post!" })
            )
        );
    }

    #[tokio::test]
    async fn a_reply_is_sent_to_the_comment_it_answers() {
        let (_service, executor, comments) = with_comments().await;
        executor.push(Step::Respond(200, comment_json(2, 23, Some(1))));

        let posted = comments
            .post_comment("I agree".to_string(), Some(CommentId(1)))
            .await
            .unwrap();

        assert_eq!(posted.parent_id, Some(CommentId(1)));
        assert_eq!(
            body_and_path(&executor).0,
            "/rest/v1.1/sites/123/comments/1/replies/new"
        );
    }

    #[tokio::test]
    async fn a_comment_that_is_not_taken_stays_and_is_never_sent_again_by_itself() {
        let (service, executor, comments) = with_comments().await;
        executor.push(Step::Offline);

        let result = comments.post_comment("Nice post!".to_string(), None).await;

        assert!(matches!(result, Err(ReaderError::Api(_))));
        let items = comments.load_items().await.unwrap();
        let written = items.last().unwrap().clone();
        assert_eq!(written.send_state, ReaderCommentSendState::Failed);
        assert!(written.send_error.is_some());
        let local_id = written.local_id.clone().unwrap();

        // A refresh and a sync of what's waiting leave it alone.
        executor.push(Step::Respond(
            200,
            comments_page(1, vec![comment_json(1, 10, None)]),
        ));
        comments.refresh().await.unwrap();
        service.sync_actions().await.unwrap();
        assert_eq!(executor.sent().len(), 3);
        assert_eq!(ids(&comments).await, ["1", "local"]);

        // The reader sends it again, and this time it's taken.
        executor.push(Step::Respond(200, comment_json(2, 23, None)));
        let posted = comments.retry_comment(local_id.clone()).await.unwrap();
        assert_eq!(posted.comment_id, Some(CommentId(2)));
        assert_eq!(ids(&comments).await, ["1", "2"]);
        assert_eq!(
            body_and_path(&executor).1,
            json!({ "content": "Nice post!" })
        );
        // It's gone, so there's nothing to send again.
        assert!(matches!(
            comments.retry_comment(local_id).await,
            Err(ReaderError::Refused { .. })
        ));
    }

    #[tokio::test]
    async fn a_refused_comment_can_be_discarded() {
        let (_service, executor, comments) = with_comments().await;
        executor.push(Step::Respond(
            400,
            json!({ "error": "invalid_input", "message": "Invalid request input" }),
        ));
        comments
            .post_comment("Nice post!".to_string(), None)
            .await
            .unwrap_err();
        let local_id = comments
            .load_items()
            .await
            .unwrap()
            .last()
            .unwrap()
            .local_id
            .clone()
            .unwrap();

        assert!(comments.discard_comment(local_id.clone()).unwrap());
        assert!(!comments.discard_comment(local_id).unwrap());
        assert_eq!(ids(&comments).await, ["1"]);
    }

    #[tokio::test]
    async fn an_empty_comment_and_a_signed_out_reader_send_nothing() {
        let (_service, executor, comments) = with_comments().await;
        assert!(matches!(
            comments.post_comment("  \n".to_string(), None).await,
            Err(ReaderError::Refused { .. })
        ));
        assert_eq!(ids(&comments).await, ["1"]);
        assert_eq!(executor.sent().len(), 1);

        let (signed_out, executor) = service_for(ReaderOwner::SignedOut);
        assert!(matches!(
            signed_out
                .comments(SITE, POST)
                .post_comment("Hello".to_string(), None)
                .await,
            Err(ReaderError::NeedsAccount)
        ));
        assert!(executor.sent().is_empty());
    }

    #[tokio::test]
    async fn a_comment_being_sent_cannot_be_sent_again_or_discarded() {
        let (_service, executor, comments) = with_comments().await;
        let (release, held) = oneshot::channel();
        executor.push(Step::Hold(held, 200, comment_json(2, 23, None)));

        let (posted, ()) = futures::join!(
            comments.post_comment("Nice post!".to_string(), None),
            async {
                let local_id = comments
                    .load_items()
                    .await
                    .unwrap()
                    .last()
                    .unwrap()
                    .local_id
                    .clone()
                    .unwrap();
                assert!(matches!(
                    comments.retry_comment(local_id.clone()).await,
                    Err(ReaderError::Refused { .. })
                ));
                assert!(!comments.discard_comment(local_id).unwrap());
                release.send(()).unwrap();
            }
        );

        posted.unwrap();
        assert_eq!(executor.sent().len(), 2);
    }

    #[tokio::test]
    async fn subscribing_to_a_posts_comments_shows_on_the_post() {
        let (service, executor) = service();
        let stream = service.stream(ReaderStream::Following);
        let mut post = post_json(7, 10);
        post["is_subscribed_comments"] = json!(false);
        post["subscribed_comments_notifications"] = json!(false);
        executor.push(Step::Respond(200, json!({ "posts": [post] })));
        stream.refresh().await.unwrap();
        let identity = ReaderPostSaveIdentity {
            id: 7,
            site_id: Some(SITE),
            feed_id: None,
            feed_item_id: None,
            is_external: false,
            url: None,
        };
        let flags = || async {
            let post = stream.load_items().await.unwrap()[0].post.clone();
            (
                post.is_subscribed_comments,
                post.receives_comment_notifications,
            )
        };
        let answered = |subscribed: bool, notified: bool| {
            Step::Respond(
                200,
                json!({ "success": true, "i_subscribe": subscribed, "receives_notifications": notified }),
            )
        };

        executor.push(answered(true, false));
        service
            .set_comments_subscribed(identity.clone(), true)
            .await
            .unwrap();
        executor.push(answered(true, true));
        service
            .set_comment_notifications(identity.clone(), true)
            .await
            .unwrap();
        assert_eq!(flags().await, (Some(true), Some(true)));

        // Unsubscribing ends the notifications too.
        executor.push(answered(false, false));
        service
            .set_comments_subscribed(identity, false)
            .await
            .unwrap();
        assert_eq!(flags().await, (Some(false), Some(false)));

        let sent = executor.sent();
        let paths: Vec<_> = sent[1..]
            .iter()
            .map(|sent| {
                sent.url
                    .trim_start_matches("https://public-api.wordpress.com/rest/v1.1")
            })
            .collect();
        assert_eq!(
            paths,
            [
                "/sites/123/posts/7/subscribers/new",
                "/sites/123/posts/7/subscribers/mine/update",
                "/sites/123/posts/7/subscribers/mine/delete",
            ]
        );
        assert_eq!(sent[2].body, Some(json!({ "receive_notifications": true })));
    }
}
