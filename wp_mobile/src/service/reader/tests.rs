use super::*;
use crate::testing::scripted::{ScriptedExecutor, Step, delegate};
use futures::channel::oneshot;
use serde_json::{Value, json};
use std::time::Duration;
use wp_api::posts::PostId;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::language::{WPComLanguage, WpComLanguageProvider};
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

fn search_by_date() -> ReaderStream {
    ReaderStream::Search {
        query: "cats".to_string(),
        sort: ReaderSearchSort::Date,
    }
}

/// A page of a search, with the handle of the next.
fn search_page(posts: &[u64], next_page: Option<&str>) -> Value {
    let posts: Vec<Value> = posts.iter().map(|id| post_json(*id, 10)).collect();
    json!({ "posts": posts, "next_page": next_page })
}

/// The `page_handle` each request was sent with, or `-` for none.
fn page_handles(executor: &ScriptedExecutor) -> Vec<String> {
    (0..executor.sent().len())
        .map(|index| {
            sent_query(executor, index)
                .into_iter()
                .find(|(name, _)| name == "page_handle")
                .map(|(_, value)| value)
                .unwrap_or_else(|| "-".to_string())
        })
        .collect()
}

#[tokio::test]
async fn empty_search_pages_that_are_not_the_last_are_followed_to_posts() {
    let (service, executor) = service_for(ReaderOwner::SignedOut);
    let stream = service.stream(search_by_date());
    executor.push(Step::Respond(200, search_page(&[], Some("w1"))));
    executor.push(Step::Respond(200, search_page(&[], Some("w2"))));
    executor.push(Step::Respond(200, search_page(&[1, 2], Some("w3"))));

    let result = stream.refresh().await.unwrap();

    assert_eq!(
        result,
        ReaderStreamPageResult {
            post_count: 2,
            new_post_count: 2,
            unreadable_post_count: 0,
            has_more: true,
        }
    );
    assert_eq!(shown(&stream).await, ["1", "2"]);
    assert_eq!(page_handles(&executor), ["-", "w1", "w2"]);

    // A later page does the same, and the next starts where the last one fetched said.
    executor.push(Step::Respond(200, search_page(&[], Some("w4"))));
    executor.push(Step::Respond(200, search_page(&[3], Some("w5"))));
    let result = stream.load_more().await.unwrap();
    assert_eq!(result.post_count, 1);
    assert_eq!(result.new_post_count, 1);
    assert!(result.has_more);
    assert_eq!(shown(&stream).await, ["1", "2", "3"]);
    executor.push(Step::Respond(200, search_page(&[4], None)));
    stream.load_more().await.unwrap();
    assert_eq!(page_handles(&executor), ["-", "w1", "w2", "w3", "w4", "w5"]);
}

#[tokio::test]
async fn a_call_stops_following_empty_search_pages_at_the_limit() {
    let (service, executor) = service_for(ReaderOwner::SignedOut);
    let stream = service.stream(search_by_date());
    // The first page, and the 8 more a call follows, all empty.
    let empty_pages = 9;
    for index in 0..empty_pages {
        let handle = format!("w{index}");
        executor.push(Step::Respond(200, search_page(&[], Some(&handle))));
    }

    let result = stream.refresh().await.unwrap();

    assert_eq!(
        result,
        ReaderStreamPageResult {
            post_count: 0,
            new_post_count: 0,
            unreadable_post_count: 0,
            has_more: true,
        }
    );
    assert_eq!(executor.sent().len(), empty_pages);
    assert_eq!(stream.info().unwrap().has_more, Some(true));
    assert!(shown(&stream).await.is_empty());

    // The next call starts from the handle of the last page fetched.
    executor.push(Step::Respond(200, search_page(&[1], None)));
    let result = stream.load_more().await.unwrap();
    assert_eq!(result.post_count, 1);
    assert!(!result.has_more);
    assert_eq!(
        page_handles(&executor).last().unwrap(),
        &format!("w{}", empty_pages - 1)
    );
}

#[tokio::test]
async fn an_empty_search_page_with_no_handle_is_the_end() {
    let (service, executor) = service_for(ReaderOwner::SignedOut);
    let stream = service.stream(search_by_date());
    executor.push(Step::Respond(200, search_page(&[], None)));
    let result = stream.refresh().await.unwrap();
    assert_eq!(result.post_count, 0);
    assert!(!result.has_more);
    assert_eq!(executor.sent().len(), 1);

    // On a later page too, after an empty page that wasn't the end.
    executor.push(Step::Respond(200, search_page(&[1], Some("w1"))));
    stream.refresh().await.unwrap();
    executor.push(Step::Respond(200, search_page(&[], Some("w2"))));
    executor.push(Step::Respond(200, search_page(&[], None)));
    let result = stream.load_more().await.unwrap();
    assert_eq!(result.post_count, 0);
    assert!(!result.has_more);
    assert_eq!(stream.info().unwrap().has_more, Some(false));
    assert_eq!(page_handles(&executor), ["-", "-", "w1", "w2"]);
    // At the end, nothing more is sent.
    stream.load_more().await.unwrap();
    assert_eq!(executor.sent().len(), 4);
}

/// The query a request was sent with, read back as the server reads it.
fn sent_query(executor: &ScriptedExecutor, index: usize) -> Vec<(String, String)> {
    let url = url::Url::parse(&executor.sent()[index].url).expect("the request has an address");
    url.query_pairs()
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect()
}

#[tokio::test]
async fn a_search_phrase_reaches_the_server_whole_with_the_readers_language() {
    let (service, executor) = service_for(ReaderOwner::SignedOut);
    service.set_language(Some(WPComLanguage::PortugueseBrazil));
    let stream = service.stream(ReaderStream::Search {
        query: "cats & dogs=friends?".to_string(),
        sort: ReaderSearchSort::Relevance,
    });
    executor.push(Step::Respond(200, json!({ "posts": [post_json(1, 10)] })));
    stream.refresh().await.unwrap();

    let query = sent_query(&executor, 0);
    let value = |name: &str| {
        query
            .iter()
            .filter(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
            .collect::<Vec<_>>()
    };
    // The `&` and `=` are in the phrase, not the start of another parameter.
    assert_eq!(value("q"), ["cats & dogs=friends?"]);
    assert!(value(" dogs").is_empty());
    assert_eq!(value("lang"), ["pt"]);
}

/// What each request was sent as its language: `_locale`, `locale` or `lang`, whichever it has.
fn sent_languages(executor: &ScriptedExecutor) -> Vec<Option<String>> {
    (0..executor.sent().len())
        .map(|index| {
            let languages: Vec<_> = sent_query(executor, index)
                .iter()
                .filter(|(name, _)| ["_locale", "locale", "lang"].contains(&name.as_str()))
                .map(|(name, value)| format!("{name}={value}"))
                .collect();
            assert!(languages.len() <= 1, "{languages:?}");
            languages.into_iter().next()
        })
        .collect()
}

/// Every kind of stream, with the language it's sent in Brazilian Portuguese: none for those whose
/// posts are the same in any language.
fn streams_and_their_language() -> Vec<(ReaderStream, Option<&'static str>)> {
    let slug = |slug: &str| ReaderTagSlug(slug.to_string());
    vec![
        (ReaderStream::Following, None),
        (ReaderStream::Liked, None),
        (ReaderStream::Automattic, None),
        (ReaderStream::P2, None),
        (ReaderStream::FreshlyPressed, None),
        (
            ReaderStream::Site {
                site_id: WpComSiteId(123),
            },
            None,
        ),
        (
            ReaderStream::Feed {
                feed_id: ReaderFeedId(456),
            },
            None,
        ),
        (tag(), Some("_locale=pt")),
        (
            ReaderStream::Tags {
                slugs: vec![slug("art"), slug("travel")],
            },
            Some("_locale=pt"),
        ),
        (
            ReaderStream::Search {
                query: "cats".to_string(),
                sort: ReaderSearchSort::Date,
            },
            Some("lang=pt"),
        ),
        (
            ReaderStream::ReaderList {
                owner: ReaderListOwner("automattic".to_string()),
                slug: ReaderListSlug("design".to_string()),
            },
            None,
        ),
        (
            ReaderStream::Discover {
                tags: vec![slug("art")],
            },
            Some("_locale=pt"),
        ),
        (ReaderStream::FirstPosts, Some("_locale=pt")),
        (
            ReaderStream::Related {
                site_id: WpComSiteId(123),
                post_id: PostId(1),
            },
            None,
        ),
    ]
}

#[tokio::test]
async fn each_stream_is_sent_the_readers_language_where_it_picks_the_posts() {
    let (service, executor) = service();
    service.set_language(Some(WPComLanguage::PortugueseBrazil));
    assert_eq!(service.language(), Some(WPComLanguage::PortugueseBrazil));
    let streams = streams_and_their_language();
    for (stream, sent) in &streams {
        executor.push(Step::Respond(200, page(&[(1, 10)])));
        let collection = service.stream(stream.clone());
        collection.refresh().await.unwrap();
        // A stream reports the language its posts are in: none for one they don't depend on.
        let language = sent.map(|_| WPComLanguage::PortugueseBrazil);
        assert_eq!(collection.language(), language, "{stream:?}");
    }
    let expected: Vec<_> = streams
        .iter()
        .map(|(_, sent)| sent.map(str::to_string))
        .collect();
    assert_eq!(sent_languages(&executor), expected);
}

#[tokio::test]
async fn with_no_language_none_is_sent() {
    let (service, executor) = service();
    assert_eq!(service.language(), None);
    let streams = streams_and_their_language();
    for (stream, _) in &streams {
        executor.push(Step::Respond(200, page(&[(1, 10)])));
        let collection = service.stream(stream.clone());
        collection.refresh().await.unwrap();
        assert_eq!(collection.language(), None);
    }
    executor.push(Step::Respond(200, json!({ "default": [] })));
    service.refresh_menu().await.unwrap();
    executor.push(Step::Respond(200, json!({ "interests": [] })));
    service.interests().await.unwrap();

    assert_eq!(sent_languages(&executor), vec![None; streams.len() + 2]);
}

#[tokio::test]
async fn the_menu_and_the_interests_are_sent_the_readers_locale() {
    let (service, executor) = service_for(ReaderOwner::SignedOut);
    for language in [
        WPComLanguage::PortugueseBrazil,
        WPComLanguage::ChineseTraditional,
        WPComLanguage::SpanishMexico,
    ] {
        service.set_language(Some(language));
        executor.push(Step::Respond(200, json!({ "default": [] })));
        service.refresh_menu().await.unwrap();
        executor.push(Step::Respond(200, json!({ "interests": [] })));
        service.interests().await.unwrap();
    }
    let sent = |name: &str, value: &str| Some(format!("{name}={value}"));
    assert_eq!(
        sent_languages(&executor),
        [
            sent("locale", "pt-br"),
            sent("_locale", "pt-br"),
            sent("locale", "zh-tw"),
            sent("_locale", "zh-tw"),
            // The menu answers `es-mx` in English, and `es` in Spanish.
            sent("locale", "es"),
            sent("_locale", "es"),
        ]
    );
    let sent = executor.sent();
    assert!(sent[0].url.contains("/rest/v1.3/read/menu?"));
    assert!(sent[1].url.contains("/wpcom/v2/read/interests?"));
}

#[tokio::test]
async fn a_stream_in_another_language_is_another_stream() {
    let (service, executor) = service_for(ReaderOwner::SignedOut);
    let search = ReaderStream::Search {
        query: "chat".to_string(),
        sort: ReaderSearchSort::Relevance,
    };
    let discover = ReaderStream::Discover { tags: Vec::new() };
    for stream in [tag(), search, discover, ReaderStream::FirstPosts] {
        // Discover and the first posts send their posts as cards.
        let answer = |id: u64| match stream {
            ReaderStream::Discover { .. } | ReaderStream::FirstPosts => {
                json!({ "cards": [{ "type": "post", "data": post_json(id, 10) }] })
            }
            _ => json!({ "posts": [post_json(id, 10)] }),
        };
        service.set_language(Some(WPComLanguage::French));
        executor.push(Step::Respond(200, answer(1)));
        service.stream(stream.clone()).refresh().await.unwrap();
        service.set_language(None);
        executor.push(Step::Respond(200, answer(2)));
        service.stream(stream.clone()).refresh().await.unwrap();

        assert_eq!(
            shown(&service.stream(stream.clone())).await,
            ["2"],
            "{stream:?}"
        );
        service.set_language(Some(WPComLanguage::French));
        assert_eq!(
            shown(&service.stream(stream.clone())).await,
            ["1"],
            "{stream:?}"
        );
        // The server answers Canadian French with French posts, so the two are one stream.
        service.set_language(Some(WPComLanguage::FrenchCanada));
        assert_eq!(
            shown(&service.stream(stream.clone())).await,
            ["1"],
            "{stream:?}"
        );
        // A language that hasn't been fetched in shows nothing of the others.
        service.set_language(Some(WPComLanguage::Japanese));
        assert!(
            shown(&service.stream(stream.clone())).await.is_empty(),
            "{stream:?}"
        );
        service.set_language(None);
    }
}

#[tokio::test]
async fn a_stream_got_before_a_change_of_language_keeps_its_own() {
    let (service, executor) = service_for(ReaderOwner::SignedOut);
    service.set_language(Some(WPComLanguage::French));
    let french = service.stream(tag());
    executor.push(Step::Respond(200, tag_page(&[1], None)));
    french.refresh().await.unwrap();

    service.set_language(Some(WPComLanguage::German));
    // A refresh of the French stream after the change is still sent in French, and stored as
    // French: German shows none of it.
    executor.push(Step::Respond(200, tag_page(&[3, 1], None)));
    french.refresh().await.unwrap();
    assert_eq!(french.language(), Some(WPComLanguage::French));
    assert_eq!(shown(&french).await, ["3", "1"]);
    let german = service.stream(tag());
    assert_eq!(german.language(), Some(WPComLanguage::German));
    assert!(shown(&german).await.is_empty());
    executor.push(Step::Respond(200, tag_page(&[2], None)));
    german.refresh().await.unwrap();
    assert_eq!(shown(&german).await, ["2"]);
    assert_eq!(shown(&french).await, ["3", "1"]);
    assert_eq!(
        sent_languages(&executor),
        [
            Some("_locale=fr".to_string()),
            Some("_locale=fr".to_string()),
            Some("_locale=de".to_string()),
        ]
    );
}

#[tokio::test]
async fn a_stream_the_language_doesnt_change_is_one_stream_in_any() {
    let (service, executor) = service_for(ReaderOwner::SignedOut);
    let site = ReaderStream::Site {
        site_id: WpComSiteId(123),
    };
    service.set_language(Some(WPComLanguage::French));
    executor.push(Step::Respond(200, page(&[(1, 10)])));
    service.stream(site.clone()).refresh().await.unwrap();

    service.set_language(Some(WPComLanguage::Japanese));
    assert_eq!(shown(&service.stream(site.clone())).await, ["1"]);
    service.set_language(None);
    assert_eq!(shown(&service.stream(site)).await, ["1"]);
}

#[tokio::test]
async fn the_delegates_language_provider_isnt_sent() {
    #[derive(Debug)]
    struct German;
    impl WpComLanguageProvider for German {
        fn current_language(&self) -> Option<WPComLanguage> {
            Some(WPComLanguage::German)
        }
    }
    let executor = Arc::new(ScriptedExecutor::default());
    let service = ReaderService::new(
        WpApiClientDelegate {
            language_provider: Some(Arc::new(German)),
            ..delegate(executor.clone())
        },
        cache(),
        ReaderOwner::SignedOut,
        ReaderSavedSource::Ios,
    )
    .unwrap();
    // A stream cached in no language is sent none, or it would hold German posts.
    executor.push(Step::Respond(200, tag_page(&[1], None)));
    service.stream(tag()).refresh().await.unwrap();
    service.set_language(Some(WPComLanguage::French));
    executor.push(Step::Respond(200, tag_page(&[2], None)));
    service.stream(tag()).refresh().await.unwrap();

    assert_eq!(
        sent_languages(&executor),
        [None, Some("_locale=fr".to_string())]
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
async fn a_post_a_page_has_twice_is_counted_and_shown_once() {
    let (service, executor) = service();
    let stream = service.stream(tag());
    executor.push(Step::Respond(200, tag_page(&[1, 2, 1], None)));

    let result = stream.refresh().await.unwrap();

    assert_eq!(
        result,
        ReaderStreamPageResult {
            post_count: 2,
            new_post_count: 2,
            unreadable_post_count: 0,
            has_more: false,
        }
    );
    assert_eq!(shown(&stream).await, ["1", "2"]);
}

#[tokio::test]
async fn a_post_a_dated_page_has_twice_does_not_hide_a_gap() {
    let (service, executor) = service();
    let stream = service.stream(ReaderStream::Following);
    executor.push(Step::Respond(200, page(&[(1, 10)])));
    stream.refresh().await.unwrap();

    // The newest posts, with 3 twice. None of them is 1, so posts may be missing above it.
    executor.push(Step::Respond(200, page(&[(3, 14), (2, 13), (3, 14)])));
    let result = stream.refresh().await.unwrap();

    assert_eq!(shown(&stream).await, ["3", "2", "*1"]);
    assert_eq!(result.post_count, 2);
    assert_eq!(result.new_post_count, 2);
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
    // Each item says whether it's saved.
    assert!(stream.is_relevant_update(&hook(wp_mobile_cache::DbTable::ReaderSavedPosts)));
    assert!(!stream.is_relevant_update(&hook(wp_mobile_cache::DbTable::ReaderComments)));
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
    async fn a_mark_the_server_answers_no_to_is_refused_and_put_back() {
        let (service, executor, stream) = following().await;
        executor.push(Step::Respond(200, json!({ "status": false })));

        let outcome = service.set_post_seen(identity(2), true).await;

        assert!(
            matches!(&outcome, Err(ReaderError::Refused { reason }) if reason == "The server didn't mark the post as seen."),
            "{outcome:?}"
        );
        assert_eq!(is_seen(&stream, 2).await, Some(false));
        assert_eq!(service.pending_action_count().unwrap(), 0);
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
    async fn an_unblock_waits_for_a_read_of_the_blocked_sites_on_its_way() {
        let (service, executor, stream) = following().await;
        let changed = json!({ "success": true, "modified_sub": true });
        executor.push(Step::Respond(200, changed.clone()));
        service
            .set_site_blocked(WpComSiteId(123), true)
            .await
            .unwrap();
        // The server's list, read before the unblock, still has the site.
        let (release, held) = oneshot::channel();
        executor.push(Step::Hold(
            held,
            200,
            json!({
                "sites": [{ "ID": 123, "name": "A site", "URL": "https://a.example" }],
                "page": 1,
                "count": 1,
            }),
        ));
        executor.push(Step::Respond(200, changed));
        let before = executor.sent().len();

        let (refreshed, (unblocked, sent_while_reading)) =
            futures::join!(service.refresh_blocked_sites(), async {
                futures::join!(service.set_site_blocked(WpComSiteId(123), false), async {
                    // Gives the unblock every chance to be sent before the list arrives.
                    for _ in 0..20 {
                        tokio::task::yield_now().await;
                    }
                    let sent = executor.sent().len() - before;
                    release.send(()).unwrap();
                    sent
                })
            });

        refreshed.unwrap();
        assert_eq!(unblocked.unwrap(), ReaderActionOutcome::Synced);
        // The unblock isn't sent while the list is being read, and the list read before it
        // doesn't block the site again.
        assert_eq!(sent_while_reading, 1);
        let paths: Vec<_> = sent_after(&executor, before)
            .into_iter()
            .map(|(path, _)| path)
            .collect();
        assert_eq!(
            paths,
            [
                "/wpcom/v2/me/blocks/sites?page=1&per_page=100",
                "/rest/v1.1/me/block/sites/123/delete"
            ]
        );
        assert!(service.blocked_sites().unwrap().is_empty());
        assert_eq!(shown(&stream).await, ["2", "1"]);
    }

    fn blocked_list(ids: &[u64]) -> Step {
        let sites: Vec<Value> = ids
            .iter()
            .map(|id| json!({ "ID": id, "name": "A site", "URL": "https://a.example" }))
            .collect();
        Step::Respond(
            200,
            json!({ "sites": sites, "page": 1, "count": ids.len() }),
        )
    }

    fn site_is_blocked(blocked: bool) -> Step {
        Step::Respond(200, json!({ "ID": 123, "is_blocked": blocked }))
    }

    /// Blocks site 123, then unblocks it if `unblock`, both taken by the server.
    async fn blocked_then(service: &ReaderService, executor: &ScriptedExecutor, unblock: bool) {
        let changed = json!({ "success": true, "modified_sub": true });
        executor.push(Step::Respond(200, changed.clone()));
        service
            .set_site_blocked(WpComSiteId(123), true)
            .await
            .unwrap();
        if unblock {
            executor.push(Step::Respond(200, changed));
            service
                .set_site_blocked(WpComSiteId(123), false)
                .await
                .unwrap();
        }
    }

    fn blocked_ids(service: &ReaderService) -> Vec<u64> {
        service
            .blocked_sites()
            .unwrap()
            .iter()
            .map(|site| site.id.0)
            .collect()
    }

    /// The whole site, with no `fields`: the server keeps an answer for each value of `fields`.
    const SITE_CHECK: &str = "/rest/v1.2/read/sites/123?";
    const BLOCKED_LIST: &str = "/wpcom/v2/me/blocks/sites?page=1&per_page=100";

    fn paths_after(executor: &ScriptedExecutor, skip: usize) -> Vec<String> {
        sent_after(executor, skip)
            .into_iter()
            .map(|(path, _)| path)
            .collect()
    }

    #[tokio::test]
    async fn a_list_still_listing_a_site_after_its_unblock_does_not_block_it_again() {
        let (service, executor, stream) = following().await;
        blocked_then(&service, &executor, true).await;
        let before = executor.sent().len();

        // The server's list is stale, and the site itself says it isn't blocked.
        executor.push(blocked_list(&[123, 7]));
        executor.push(site_is_blocked(false));
        service.refresh_blocked_sites().await.unwrap();

        assert_eq!(blocked_ids(&service), [7]);
        assert_eq!(shown(&stream).await, ["2", "1"]);
        assert_eq!(paths_after(&executor, before), [BLOCKED_LIST, SITE_CHECK]);

        // While the list stays stale, the site is asked about again.
        executor.push(blocked_list(&[123, 7]));
        executor.push(site_is_blocked(false));
        service.refresh_blocked_sites().await.unwrap();
        assert_eq!(blocked_ids(&service), [7]);
        assert_eq!(
            paths_after(&executor, before + 2),
            [BLOCKED_LIST, SITE_CHECK]
        );

        // Once the list agrees, the change is forgotten: a later block made elsewhere is taken
        // from the list, with nothing more asked.
        executor.push(blocked_list(&[7]));
        service.refresh_blocked_sites().await.unwrap();
        executor.push(blocked_list(&[123, 7]));
        service.refresh_blocked_sites().await.unwrap();
        assert_eq!(blocked_ids(&service), [7, 123]);
        assert_eq!(
            paths_after(&executor, before + 4),
            [BLOCKED_LIST, BLOCKED_LIST]
        );
    }

    #[tokio::test]
    async fn a_list_leaving_out_a_site_after_its_block_does_not_unblock_it() {
        let (service, executor, stream) = following().await;
        blocked_then(&service, &executor, false).await;
        let before = executor.sent().len();

        executor.push(blocked_list(&[]));
        executor.push(site_is_blocked(true));
        service.refresh_blocked_sites().await.unwrap();

        assert_eq!(blocked_ids(&service), [123]);
        assert!(shown(&stream).await.is_empty());
        assert_eq!(paths_after(&executor, before), [BLOCKED_LIST, SITE_CHECK]);
    }

    #[tokio::test]
    async fn a_site_blocked_again_elsewhere_after_an_unblock_is_taken_from_the_list() {
        let (service, executor, stream) = following().await;
        blocked_then(&service, &executor, true).await;

        executor.push(blocked_list(&[123]));
        executor.push(site_is_blocked(true));
        service.refresh_blocked_sites().await.unwrap();

        assert_eq!(blocked_ids(&service), [123]);
        assert!(shown(&stream).await.is_empty());
    }

    #[tokio::test]
    async fn a_site_that_cannot_be_asked_about_is_taken_from_the_list() {
        let (service, executor, _) = following().await;
        blocked_then(&service, &executor, true).await;

        // A deleted site.
        executor.push(blocked_list(&[123]));
        executor.push(Step::Respond(
            410,
            json!({ "error": "site_gone", "message": "The site has been deleted" }),
        ));
        service.refresh_blocked_sites().await.unwrap();

        assert_eq!(blocked_ids(&service), [123]);
    }

    #[tokio::test]
    async fn a_site_check_that_gets_no_answer_fails_the_refresh_and_changes_nothing() {
        let (service, executor, stream) = following().await;
        blocked_then(&service, &executor, true).await;

        executor.push(blocked_list(&[123, 7]));
        executor.push(Step::Offline);
        let result = service.refresh_blocked_sites().await;

        assert!(result.is_err());
        assert!(blocked_ids(&service).is_empty());
        assert_eq!(shown(&stream).await, ["2", "1"]);
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

    /// A `User-Agent` an app sends a view with.
    const APP: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148 wp-iphone/26.4";

    /// Opens a post, counting a view, and returns what became of the view.
    async fn view(
        service: &ReaderService,
        key: ReaderSavedKey,
        user_agent: &str,
    ) -> Result<ReaderPageViewOutcome, ReaderError> {
        let choice = ReaderPageViewChoice::Count {
            user_agent: user_agent.to_string(),
        };
        match service.post_opened(key, choice).await.unwrap().page_view {
            ReaderOpenedPageView::Reported { outcome } => Ok(outcome),
            ReaderOpenedPageView::Failed { error } => Err(error),
            ReaderOpenedPageView::NotAsked => panic!("A view was asked for"),
        }
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

        let outcome = view(&service, key(1), APP).await.unwrap();

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
        // The view goes out as the app, not as the library.
        assert_eq!(sent.last().unwrap().user_agent.as_deref(), Some(APP));
        // Nothing else was asked for: a signed-out reader is no site's member.
        assert_eq!(sent.len(), 2);
    }

    #[tokio::test]
    async fn a_view_is_not_sent_with_a_user_agent_that_stats_dont_count() {
        let (service, executor) = reading(ReaderOwner::SignedOut, vec![site_post(1, false)]).await;
        let library = wp_api::request::user_agent::default_user_agent("URLSession");

        for user_agent in ["", " ", library.as_str()] {
            let outcome = view(&service, key(1), user_agent).await.unwrap();
            assert_eq!(
                outcome,
                ReaderPageViewOutcome::NotCountedForUserAgent,
                "{user_agent:?}"
            );
        }

        // Only the stream was fetched: no view left the device.
        assert_eq!(executor.sent().len(), 1);
    }

    #[tokio::test]
    async fn a_view_by_a_member_of_a_public_site_is_not_counted() {
        let (service, executor) =
            reading(ACCOUNT, vec![site_post(1, false), site_post(2, false)]).await;
        // The account's sites are fetched the first time a post is opened.
        executor.push(sites(&[123]));

        let outcome = view(&service, key(1), APP).await.unwrap();

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
        let outcome = view(&service, key(2), APP).await.unwrap();
        assert_eq!(outcome, ReaderPageViewOutcome::NotCountedForSiteMember);
        assert_eq!(executor.sent().len(), 2);
    }

    #[tokio::test]
    async fn a_view_by_someone_who_is_not_a_member_is_counted() {
        let (service, executor) = reading(ACCOUNT, vec![site_post(1, false)]).await;
        service.set_member_sites(vec![WpComSiteId(999)]).unwrap();
        executor.push(pixel());

        let outcome = view(&service, key(1), APP).await.unwrap();

        assert_eq!(outcome, ReaderPageViewOutcome::Counted);
        // The app said which sites the account belongs to, so they weren't fetched.
        assert_eq!(executor.sent().len(), 2);
    }

    #[tokio::test]
    async fn a_view_of_a_private_site_is_counted_even_by_a_member() {
        let (service, executor) = reading(ACCOUNT, vec![site_post(1, true)]).await;
        service.set_member_sites(vec![WpComSiteId(123)]).unwrap();
        executor.push(pixel());

        let outcome = view(&service, key(1), APP).await.unwrap();

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

        let outcome = view(&service, key, APP).await.unwrap();

        assert_eq!(outcome, ReaderPageViewOutcome::NotCountedForRssPost);
        assert_eq!(executor.sent().len(), 1);
    }

    #[tokio::test]
    async fn a_view_of_a_feed_item_with_no_post_id_is_not_counted() {
        let item = feed_items_with_no_post_id::page()["posts"][0].clone();
        let (service, executor) = reading(ReaderOwner::SignedOut, vec![item]).await;

        let outcome = view(&service, feed_items_with_no_post_id::key(100000003), APP)
            .await
            .unwrap();

        assert_eq!(outcome, ReaderPageViewOutcome::NotCountedForRssPost);
        assert_eq!(executor.sent().len(), 1);
    }

    #[tokio::test]
    async fn a_post_the_cache_lacks_or_knows_too_little_of_is_not_counted() {
        // Post 1 has no site URL.
        let (service, executor) = reading(ReaderOwner::SignedOut, vec![post_json(1, 10)]).await;

        for post in [1, 99] {
            let outcome = view(&service, key(post), APP).await.unwrap();
            assert_eq!(outcome, ReaderPageViewOutcome::NotCountedForMissingDetails);
        }
        assert_eq!(executor.sent().len(), 1);
    }

    #[tokio::test]
    async fn a_view_that_cannot_be_sent_is_an_error_and_is_not_kept() {
        let (service, executor) = reading(ReaderOwner::SignedOut, vec![site_post(1, false)]).await;
        executor.push(Step::Offline);

        let result = view(&service, key(1), APP).await;

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
        let outcome = view(&service, key(1), APP).await.unwrap();
        assert_eq!(outcome, ReaderPageViewOutcome::NotCountedForSiteMember);

        // Back online, the list says the account has left the site.
        executor.push(sites(&[999]));
        executor.push(pixel());
        let outcome = view(&service, key(1), APP).await.unwrap();
        assert_eq!(outcome, ReaderPageViewOutcome::Counted);
    }

    #[tokio::test]
    async fn with_no_list_of_sites_a_failed_fetch_is_the_error() {
        let (service, executor) = reading(ACCOUNT, vec![site_post(1, false)]).await;
        executor.push(Step::Offline);

        let result = view(&service, key(1), APP).await;

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
            ["/rest/v1.2/read/site/123/post/7/related?size_global=2&size_local=2&meta=site%2Cfeed"]
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

mod opening {
    use super::*;

    const APP: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148 wp-iphone/26.4";

    /// A followed post of site 123, with a seen state and the site's address.
    fn followed_post(id: u64, seen: bool) -> Value {
        let mut post = post_json(id, 10);
        post["is_seen"] = json!(seen);
        post["site_URL"] = json!("https://example.wordpress.com");
        post["site_is_private"] = json!(false);
        post
    }

    /// A service whose Following stream has `posts`, and whose account is a member of no site.
    async fn following(
        owner: ReaderOwner,
        posts: Vec<Value>,
    ) -> (ReaderService, Arc<ScriptedExecutor>) {
        let (service, executor) = service_for(owner);
        executor.push(Step::Respond(200, json!({ "posts": posts.clone() })));
        if owner == ReaderOwner::SignedOut {
            // Signed out there's no Following stream: the posts come from a tag.
            service.stream(tag()).refresh().await.unwrap();
        } else {
            service
                .stream(ReaderStream::Following)
                .refresh()
                .await
                .unwrap();
            service.set_member_sites(vec![WpComSiteId(999)]).unwrap();
        }
        (service, executor)
    }

    fn is_seen(service: &ReaderService, id: i64) -> Option<bool> {
        service.cached_post(key(id)).unwrap().unwrap().is_seen
    }

    fn marked() -> Step {
        Step::Respond(200, json!({ "status": true }))
    }

    fn pixel() -> Step {
        Step::Respond(200, json!(null))
    }

    fn count() -> ReaderPageViewChoice {
        ReaderPageViewChoice::Count {
            user_agent: APP.to_string(),
        }
    }

    /// The paths of the requests sent after the stream's.
    fn sent_paths(executor: &ScriptedExecutor) -> Vec<String> {
        executor
            .sent()
            .iter()
            .skip(1)
            .map(|sent| {
                let url = sent
                    .url
                    .trim_start_matches("https://public-api.wordpress.com");
                url.split('?').next().unwrap_or(url).to_string()
            })
            .collect()
    }

    #[tokio::test]
    async fn opening_a_post_marks_it_seen_in_every_stream_that_shows_it() {
        let (service, executor) = following(ACCOUNT, vec![followed_post(1, false)]).await;
        // The same post in its site's stream.
        executor.push(Step::Respond(
            200,
            json!({ "posts": [followed_post(1, false)] }),
        ));
        let site = service.stream(ReaderStream::Site {
            site_id: WpComSiteId(123),
        });
        site.refresh().await.unwrap();
        executor.push(marked());

        let opened = service
            .post_opened(key(1), ReaderPageViewChoice::DontCount)
            .await
            .unwrap();

        assert!(
            matches!(
                opened.seen,
                ReaderOpenedSeen::Marked {
                    outcome: ReaderActionOutcome::Synced
                }
            ),
            "{opened:?}"
        );
        assert!(matches!(opened.page_view, ReaderOpenedPageView::NotAsked));
        for stream in [service.stream(ReaderStream::Following), site] {
            let items = stream.load_items().await.unwrap();
            assert_eq!(items[0].post.is_seen, Some(true));
        }
        // Only the mark was sent: no view, as asked.
        assert_eq!(executor.sent().len(), 3);
        assert!(
            executor.sent()[2]
                .url
                .ends_with("/wpcom/v2/seen-posts/seen/blog/new")
        );
    }

    #[tokio::test]
    async fn the_mark_and_the_view_are_both_done_and_reported() {
        let (service, executor) = following(ACCOUNT, vec![followed_post(1, false)]).await;
        executor.push(marked());
        executor.push(pixel());

        let opened = service.post_opened(key(1), count()).await.unwrap();

        assert!(matches!(opened.seen, ReaderOpenedSeen::Marked { .. }));
        assert!(matches!(
            opened.page_view,
            ReaderOpenedPageView::Reported {
                outcome: ReaderPageViewOutcome::Counted
            }
        ));
        let paths = sent_paths(&executor);
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"/wpcom/v2/seen-posts/seen/blog/new".to_string()));
        assert!(
            paths
                .iter()
                .any(|path| path.starts_with("https://pixel.wp.com/g.gif"))
        );
    }

    #[tokio::test]
    async fn a_seen_post_and_one_with_no_seen_state_send_no_mark() {
        let mut no_state = post_json(2, 10);
        no_state["site_URL"] = json!("https://example.wordpress.com");
        let (service, executor) = following(ACCOUNT, vec![followed_post(1, true), no_state]).await;

        let seen = service
            .post_opened(key(1), ReaderPageViewChoice::DontCount)
            .await
            .unwrap();
        let without = service
            .post_opened(key(2), ReaderPageViewChoice::DontCount)
            .await
            .unwrap();
        let missing = service
            .post_opened(key(99), ReaderPageViewChoice::DontCount)
            .await
            .unwrap();

        assert!(matches!(seen.seen, ReaderOpenedSeen::AlreadySeen));
        assert!(matches!(without.seen, ReaderOpenedSeen::NoSeenState));
        assert!(matches!(missing.seen, ReaderOpenedSeen::NotCached));
        assert!(sent_paths(&executor).is_empty());
    }

    #[tokio::test]
    async fn a_signed_out_reader_opens_a_post_without_a_mark() {
        let (service, executor) =
            following(ReaderOwner::SignedOut, vec![followed_post(1, false)]).await;
        executor.push(pixel());

        let opened = service.post_opened(key(1), count()).await.unwrap();

        assert!(matches!(opened.seen, ReaderOpenedSeen::NoSeenState));
        assert!(matches!(
            opened.page_view,
            ReaderOpenedPageView::Reported {
                outcome: ReaderPageViewOutcome::Counted
            }
        ));
        assert_eq!(sent_paths(&executor).len(), 1);
        assert_eq!(service.pending_action_count().unwrap(), 0);
    }

    #[tokio::test]
    async fn offline_the_mark_waits_and_the_view_fails_and_both_are_said() {
        let (service, executor) = following(ACCOUNT, vec![followed_post(1, false)]).await;
        executor.push(Step::Offline);
        executor.push(Step::Offline);

        let opened = service.post_opened(key(1), count()).await.unwrap();

        assert!(
            matches!(
                opened.seen,
                ReaderOpenedSeen::Marked {
                    outcome: ReaderActionOutcome::Pending { .. }
                }
            ),
            "{opened:?}"
        );
        assert!(
            matches!(
                opened.page_view,
                ReaderOpenedPageView::Failed {
                    error: ReaderError::Api(_)
                }
            ),
            "{opened:?}"
        );
        // It shows as seen, and is sent once the device is back online. The view isn't.
        assert_eq!(is_seen(&service, 1), Some(true));
        assert_eq!(service.pending_action_count().unwrap(), 1);
        executor.push(marked());
        assert_eq!(service.sync_actions().await.unwrap(), 0);
        assert_eq!(
            sent_paths(&executor).last().map(String::as_str),
            Some("/wpcom/v2/seen-posts/seen/blog/new")
        );
    }

    #[tokio::test]
    async fn a_refused_mark_is_the_seen_parts_error_and_the_view_still_counts() {
        let (service, executor) = following(ACCOUNT, vec![followed_post(1, false)]).await;
        executor.push(Step::Respond(
            403,
            json!({ "code": "rest_forbidden", "message": "Sorry, you are not allowed to do that.", "data": { "status": 403 } }),
        ));
        executor.push(pixel());

        let opened = service.post_opened(key(1), count()).await.unwrap();

        assert!(
            matches!(
                opened.seen,
                ReaderOpenedSeen::Failed {
                    error: ReaderError::Api(_)
                }
            ),
            "{opened:?}"
        );
        assert!(matches!(
            opened.page_view,
            ReaderOpenedPageView::Reported {
                outcome: ReaderPageViewOutcome::Counted
            }
        ));
        assert_eq!(is_seen(&service, 1), Some(false));
        assert_eq!(service.pending_action_count().unwrap(), 0);
    }
}

mod links {
    use super::*;

    const POST_URL: &str = "https://example.wordpress.com/2026/11/18/hello-world/";

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

    fn error(status: u32, code: &str) -> Step {
        Step::Respond(status, json!({ "error": code, "message": "" }))
    }

    /// The server's answer to a post address on a site that isn't on WordPress.com or Jetpack.
    fn server_error() -> Step {
        Step::Respond(500, json!(null))
    }

    fn site(id: u64) -> Step {
        Step::Respond(
            200,
            json!({ "ID": id, "name": "A site", "URL": "https://example.com", "feed_ID": 456 }),
        )
    }

    /// The feeds found at an address: one of site `site`, or of no site.
    fn feeds(feed_id: u64, site: Option<u64>) -> Step {
        let mut feed = json!({ "feed_ID": feed_id, "name": "A feed", "URL": "https://example.com", "feed_URL": "https://example.com/feed/" });
        if let Some(site) = site {
            feed["blog_ID"] = json!(site);
        }
        Step::Respond(
            200,
            json!({ "feeds": [{ "subscribe_URL": "https://example.com/feed/", "feed_ID": feed_id.to_string(), "meta": { "data": { "feed": feed } } }] }),
        )
    }

    fn not_in_reader(reason: ReaderLinkNotInReader) -> ReaderLink {
        ReaderLink::NotInReader { reason }
    }

    #[tokio::test]
    async fn a_post_link_resolves_to_the_post_for_an_account() {
        let (service, executor) = service();
        executor.push(Step::Respond(200, json!({ "site_id": 123, "post_id": 7 })));
        executor.push(Step::Respond(200, post_json(7, 10)));

        let link = service.resolve_link(POST_URL.to_string()).await.unwrap();

        let ReaderLink::Post { post } = link else {
            panic!("{link:?}")
        };
        assert_eq!(post.identity.id, 7);
        assert!(service.cached_post(key(7)).unwrap().is_some());
        assert_eq!(
            paths(&executor),
            [
                "/wpcom/v2/mobile/resolve-reader-url?url=https%3A%2F%2Fexample.wordpress.com%2F2026%2F11%2F18%2Fhello-world%2F",
                "/rest/v1.2/read/sites/123/posts/7?meta=site%2Cfeed",
            ]
        );
    }

    #[tokio::test]
    async fn signed_out_a_post_link_opens_the_post_found_by_its_slug() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        executor.push(Step::Respond(
            200,
            json!({ "ID": 7, "site_ID": 123, "slug": "hello-world", "title": "Post 7" }),
        ));
        executor.push(Step::Respond(200, post_json(7, 10)));

        let link = service.resolve_link(POST_URL.to_string()).await.unwrap();

        let ReaderLink::Post { post } = link else {
            panic!("{link:?}")
        };
        assert_eq!(post.identity.id, 7);
        assert!(service.cached_post(key(7)).unwrap().is_some());
        // The resolver isn't asked: it needs an account.
        assert_eq!(
            paths(&executor),
            [
                "/rest/v1.1/sites/example.wordpress.com/posts/slug:hello-world?",
                "/rest/v1.2/read/sites/123/posts/7?meta=site%2Cfeed",
            ]
        );
    }

    #[tokio::test]
    async fn signed_out_the_slug_is_the_last_part_of_the_path_decoded() {
        for (link, sent) in [
            (
                "https://example.com/a-post/",
                "/rest/v1.1/sites/example.com/posts/slug:a-post?",
            ),
            (
                "https://Example.com/blog/2026/11/a-post?share=1#comments",
                "/rest/v1.1/sites/example.com/posts/slug:a-post?",
            ),
            (
                "https://example.com/2026/11/19/%e5%86%99%e7%9c%9f/",
                "/rest/v1.1/sites/example.com/posts/slug:%E5%86%99%E7%9C%9F?",
            ),
        ] {
            let (service, executor) = service_for(ReaderOwner::SignedOut);
            executor.push(error(404, "unknown_post"));
            let _ = service.resolve_link(link.to_string()).await;
            assert_eq!(paths(&executor), [sent], "{link}");
        }
    }

    #[tokio::test]
    async fn signed_out_a_post_link_the_slug_does_not_find_needs_an_account() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        // No post with the slug, a host that's no site's, and a private site: the resolver, with
        // an account, might still find a post there.
        for answer in [
            error(404, "unknown_post"),
            error(404, "unknown_blog"),
            error(403, "unauthorized"),
        ] {
            executor.push(answer);
            assert!(matches!(
                service.resolve_link(POST_URL.to_string()).await,
                Err(ReaderError::NeedsAccount)
            ));
        }
        assert_eq!(executor.sent().len(), 3);

        // A request that fails isn't a refusal, so it's the error.
        for answer in [Step::Offline, Step::Respond(500, json!(null))] {
            executor.push(answer);
            assert!(matches!(
                service.resolve_link(POST_URL.to_string()).await,
                Err(ReaderError::Api(_))
            ));
        }
        // So is a failure to fetch the post the slug found.
        executor.push(Step::Respond(
            200,
            json!({ "ID": 7, "site_ID": 123, "slug": "hello-world" }),
        ));
        executor.push(Step::Offline);
        assert!(matches!(
            service.resolve_link(POST_URL.to_string()).await,
            Err(ReaderError::Api(_))
        ));
    }

    #[tokio::test]
    async fn signed_out_a_post_link_with_no_slug_needs_an_account_and_sends_nothing() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        for link in [
            // Archives, a feed, a page of posts and a file: WordPress gives no post such a slug.
            "https://example.com/2026/11/",
            "https://example.com/2026/11/18/",
            "https://example.com/feed/",
            "https://example.com/tag/cats/RSS2",
            "https://example.com/page/2",
            "https://example.com/page2",
            "https://example.com/index.php",
            // Escapes that don't spell UTF-8.
            "https://example.com/%ff%fe/",
        ] {
            assert!(
                matches!(
                    service.resolve_link(link.to_string()).await,
                    Err(ReaderError::NeedsAccount)
                ),
                "{link}"
            );
        }
        assert!(executor.sent().is_empty());
    }

    #[tokio::test]
    async fn a_sites_home_address_opens_the_site_signed_out_too() {
        for owner in [ACCOUNT, ReaderOwner::SignedOut] {
            let (service, executor) = service_for(owner);
            executor.push(site(123));

            // Typed without a scheme, and with no slash.
            let link = service
                .resolve_link("example.wordpress.com".to_string())
                .await
                .unwrap();

            let ReaderLink::Site { site_id, site } = link else {
                panic!("{link:?}")
            };
            assert_eq!(site_id, WpComSiteId(123));
            assert_eq!(site.name.as_deref(), Some("A site"));
            assert_eq!(
                service
                    .cached_site(Some(WpComSiteId(123)), None)
                    .unwrap()
                    .and_then(|site| site.name),
                Some("A site".to_string())
            );
            assert_eq!(
                paths(&executor),
                ["/rest/v1.2/read/sites/example.wordpress.com?"]
            );
        }
    }

    #[tokio::test]
    async fn the_home_address_of_a_site_that_is_only_a_feed_opens_the_feed() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        executor.push(error(404, "unknown_blog"));
        executor.push(feeds(456, None));

        let link = service
            .resolve_link("https://example.com/".to_string())
            .await
            .unwrap();

        let ReaderLink::Feed { feed_id, site } = link else {
            panic!("{link:?}")
        };
        assert_eq!(feed_id, ReaderFeedId(456));
        assert_eq!(site.name.as_deref(), Some("A feed"));
        assert_eq!(
            paths(&executor),
            [
                "/rest/v1.2/read/sites/example.com?",
                "/rest/v1.1/read/feed?url=https%3A%2F%2Fexample.com%2F&meta=feed",
            ]
        );
    }

    #[tokio::test]
    async fn a_home_address_with_no_site_or_feed_says_so() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        executor.push(error(404, "unknown_blog"));
        executor.push(error(404, "no_such_feed"));

        let link = service
            .resolve_link("https://example.com".to_string())
            .await
            .unwrap();

        assert_eq!(
            link,
            not_in_reader(ReaderLinkNotInReader::NotWordPressOrFeed)
        );
        assert_eq!(executor.sent().len(), 2);
    }

    #[tokio::test]
    async fn an_address_on_a_site_with_no_post_or_feed_there_says_so() {
        let (service, executor) = service();
        executor.push(error(404, "post_not_found"));
        executor.push(error(404, "no_such_feed"));

        let link = service.resolve_link(POST_URL.to_string()).await.unwrap();

        assert_eq!(link, not_in_reader(ReaderLinkNotInReader::NothingThere));
        assert!(paths(&executor)[1].starts_with("/rest/v1.1/read/feed?url=https"));
    }

    #[tokio::test]
    async fn a_sites_address_with_a_path_opens_the_site_of_its_feed() {
        let (service, executor) = service();
        executor.push(error(404, "post_not_found"));
        executor.push(feeds(456, Some(123)));

        let link = service
            .resolve_link("https://wordpress.com/blog/".to_string())
            .await
            .unwrap();

        let ReaderLink::Site { site_id, .. } = link else {
            panic!("{link:?}")
        };
        assert_eq!(site_id, WpComSiteId(123));
    }

    #[tokio::test]
    async fn an_address_on_another_site_is_its_feed_or_nothing() {
        let (service, executor) = service();
        // The server fails on it, and no WordPress.com or Jetpack site has the domain.
        executor.push(server_error());
        executor.push(error(404, "unknown_blog"));
        executor.push(error(404, "no_such_feed"));
        let link = service
            .resolve_link("https://example.com/learn".to_string())
            .await
            .unwrap();
        assert_eq!(
            link,
            not_in_reader(ReaderLinkNotInReader::NotWordPressOrFeed)
        );
        assert_eq!(
            paths(&executor)[1..],
            [
                "/rest/v1.2/read/sites/example.com?",
                "/rest/v1.1/read/feed?url=https%3A%2F%2Fexample.com%2Flearn&meta=feed",
            ]
        );

        // A post of a site with a feed opens the feed.
        executor.push(server_error());
        executor.push(error(404, "unknown_blog"));
        executor.push(feeds(456, None));
        let link = service
            .resolve_link("https://example.com/2026/11/18/a-post/".to_string())
            .await
            .unwrap();
        assert!(
            matches!(
                link,
                ReaderLink::Feed {
                    feed_id: ReaderFeedId(456),
                    ..
                }
            ),
            "{link:?}"
        );
    }

    #[tokio::test]
    async fn the_servers_failure_on_a_sites_address_is_the_error() {
        let (service, executor) = service();
        executor.push(server_error());
        executor.push(site(123));

        let result = service.resolve_link(POST_URL.to_string()).await;

        assert!(
            matches!(&result, Err(ReaderError::Api(error)) if error.status_code() == Some(500)),
            "{result:?}"
        );
        // Nothing more was asked once the domain turned out to be a site's.
        assert_eq!(executor.sent().len(), 2);
    }

    #[tokio::test]
    async fn a_refusal_is_the_error() {
        let (service, executor) = service();
        executor.push(error(403, "unauthorized"));
        assert!(matches!(
            service
                .resolve_link("https://private.example.com/".to_string())
                .await,
            Err(ReaderError::Api(_))
        ));

        executor.push(Step::Respond(
            401,
            json!({ "code": "rest_forbidden", "message": "Sorry, you are not allowed to do that.", "data": { "status": 401 } }),
        ));
        assert!(matches!(
            service.resolve_link(POST_URL.to_string()).await,
            Err(ReaderError::Api(_))
        ));
        executor.push(Step::Offline);
        assert!(matches!(
            service.resolve_link(POST_URL.to_string()).await,
            Err(ReaderError::Api(_))
        ));
        assert_eq!(executor.sent().len(), 3);
    }

    #[tokio::test]
    async fn a_short_link_and_what_is_not_an_address_send_nothing() {
        let (service, executor) = service();
        for (link, reason) in [
            ("https://wp.me/pf2B5-oJs", ReaderLinkNotInReader::ShortLink),
            ("WP.ME/pf2B5-oJs", ReaderLinkNotInReader::ShortLink),
            ("not a link", ReaderLinkNotInReader::NotAnAddress),
            ("", ReaderLinkNotInReader::NotAnAddress),
            (
                "mailto:someone@example.com",
                ReaderLinkNotInReader::NotAnAddress,
            ),
            (
                "ftp://example.com/file",
                ReaderLinkNotInReader::NotAnAddress,
            ),
        ] {
            assert_eq!(
                service.resolve_link(link.to_string()).await.unwrap(),
                not_in_reader(reason),
                "{link}"
            );
        }
        assert!(executor.sent().is_empty());
        assert!(!reader_link_not_in_reader_message(ReaderLinkNotInReader::ShortLink).is_empty());
    }

    /// A feed, of site `site` or of none (`0`).
    fn feed(feed_id: u64, site: u64) -> Step {
        Step::Respond(
            200,
            json!({ "feed_ID": feed_id.to_string(), "blog_ID": site.to_string(), "name": "A feed", "URL": "https://example.com", "feed_URL": "https://example.com/feed/" }),
        )
    }

    #[tokio::test]
    async fn the_readers_own_address_of_a_site_opens_the_site_signed_out() {
        for link in [
            "https://wordpress.com/read/blogs/123",
            "https://wordpress.com/reader/blogs/123/",
            "https://wordpress.com/read/blogs/123/posts",
            "https://www.wordpress.com/READ/Blogs/123",
            "https://wordpress.com/read/blog/id/123",
            "wordpress.com/reader/blogs/123?ref=share#top",
        ] {
            let (service, executor) = service_for(ReaderOwner::SignedOut);
            executor.push(site(123));

            let opened = service.resolve_link(link.to_string()).await.unwrap();

            let ReaderLink::Site { site_id, site } = opened else {
                panic!("{link}: {opened:?}")
            };
            assert_eq!(site_id, WpComSiteId(123), "{link}");
            assert_eq!(site.name.as_deref(), Some("A site"), "{link}");
            assert_eq!(paths(&executor), ["/rest/v1.2/read/sites/123?"], "{link}");
        }
    }

    #[tokio::test]
    async fn the_readers_own_address_of_a_feed_opens_the_feed_or_its_site_signed_out() {
        for link in [
            "https://wordpress.com/read/feeds/456",
            "https://wordpress.com/reader/feeds/456/posts/",
            "https://wordpress.com/read/blog/feed/456",
        ] {
            let (service, executor) = service_for(ReaderOwner::SignedOut);
            executor.push(feed(456, 0));

            let opened = service.resolve_link(link.to_string()).await.unwrap();

            let ReaderLink::Feed { feed_id, site } = opened else {
                panic!("{link}: {opened:?}")
            };
            assert_eq!(feed_id, ReaderFeedId(456), "{link}");
            assert_eq!(site.name.as_deref(), Some("A feed"), "{link}");
            assert_eq!(paths(&executor), ["/rest/v1.1/read/feed/456"], "{link}");
        }

        // A WordPress.com or Jetpack site's feed opens the site, as the web Reader opens it.
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        executor.push(feed(456, 123));
        let opened = service
            .resolve_link("https://wordpress.com/reader/feeds/456".to_string())
            .await
            .unwrap();
        assert!(
            matches!(
                opened,
                ReaderLink::Site {
                    site_id: WpComSiteId(123),
                    ..
                }
            ),
            "{opened:?}"
        );
    }

    #[tokio::test]
    async fn the_readers_own_address_of_a_sites_post_opens_the_post_signed_out() {
        for link in [
            "https://wordpress.com/read/blogs/123/posts/7",
            "https://wordpress.com/reader/blogs/123/posts/7/",
            "https://wordpress.com/read/post/id/123/7",
        ] {
            let (service, executor) = service_for(ReaderOwner::SignedOut);
            executor.push(Step::Respond(200, post_json(7, 10)));

            let opened = service.resolve_link(link.to_string()).await.unwrap();

            let ReaderLink::Post { post } = opened else {
                panic!("{link}: {opened:?}")
            };
            assert_eq!(post.identity.id, 7, "{link}");
            assert!(service.cached_post(key(7)).unwrap().is_some(), "{link}");
            assert_eq!(
                paths(&executor),
                ["/rest/v1.2/read/sites/123/posts/7?meta=site%2Cfeed"],
                "{link}"
            );
        }
    }

    #[tokio::test]
    async fn the_readers_own_address_of_a_feeds_post_opens_the_post_signed_out() {
        for link in [
            "https://wordpress.com/read/feeds/456/posts/9",
            "https://wordpress.com/reader/feeds/456/posts/9",
            "https://wordpress.com/read/post/feed/456/9",
        ] {
            let (service, executor) = service_for(ReaderOwner::SignedOut);
            executor.push(Step::Respond(
                200,
                json!({ "ID": 9, "feed_ID": 456, "feed_item_ID": 9, "is_external": true, "title": "From a feed" }),
            ));

            let opened = service.resolve_link(link.to_string()).await.unwrap();

            let ReaderLink::Post { post } = opened else {
                panic!("{link}: {opened:?}")
            };
            assert_eq!(post.title.as_deref(), Some("From a feed"), "{link}");
            assert_eq!(
                paths(&executor),
                ["/rest/v1.3/read/feed/456/posts/9?meta=site%2Cfeed"],
                "{link}"
            );
        }
    }

    #[tokio::test]
    async fn a_readers_own_address_that_names_nothing_says_so_and_sends_nothing() {
        for owner in [ACCOUNT, ReaderOwner::SignedOut] {
            let (service, executor) = service_for(owner);
            for link in [
                "https://wordpress.com/read",
                "https://wordpress.com/reader/",
                "https://wordpress.com/read/blogs",
                "https://wordpress.com/read/blogs/example.com",
                "https://wordpress.com/read/blogs/0",
                "https://wordpress.com/read/blogs/+123",
                "https://wordpress.com/read/blogs/123/posts/a-post",
                "https://wordpress.com/read/blogs/123/posts/7/likes",
                "https://wordpress.com/read/feeds/456/items/9",
                // The older addresses are only under `/read/`.
                "https://wordpress.com/reader/blog/id/123",
                // Streams that aren't a site or a feed.
                "https://wordpress.com/read/tag/photography",
                "https://wordpress.com/reader/list/someone/a-list",
                "https://wordpress.com/read/search?q=cats",
                "https://wordpress.com/reader/a8c",
            ] {
                assert_eq!(
                    service.resolve_link(link.to_string()).await.unwrap(),
                    not_in_reader(ReaderLinkNotInReader::NothingThere),
                    "{link}"
                );
            }
            assert!(executor.sent().is_empty());
        }
    }

    #[tokio::test]
    async fn a_readers_own_address_with_ids_the_server_does_not_have_says_so() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        for (link, answer) in [
            (
                "https://wordpress.com/read/blogs/123",
                error(404, "unknown_blog"),
            ),
            (
                "https://wordpress.com/read/blogs/123",
                error(410, "site_gone"),
            ),
            (
                "https://wordpress.com/read/feeds/456",
                error(404, "no_such_feed"),
            ),
            (
                "https://wordpress.com/read/blogs/123/posts/7",
                error(404, "unknown_post"),
            ),
            (
                "https://wordpress.com/read/feeds/456/posts/9",
                error(404, "no_such_item"),
            ),
        ] {
            executor.push(answer);
            assert_eq!(
                service.resolve_link(link.to_string()).await.unwrap(),
                not_in_reader(ReaderLinkNotInReader::NothingThere),
                "{link}"
            );
        }

        // A refusal, such as for a private site, and a request that fails are the error.
        executor.push(error(403, "unauthorized"));
        assert!(matches!(
            service
                .resolve_link("https://wordpress.com/read/blogs/123".to_string())
                .await,
            Err(ReaderError::Api(_))
        ));
        executor.push(Step::Offline);
        assert!(matches!(
            service
                .resolve_link("https://wordpress.com/read/blogs/123/posts/7".to_string())
                .await,
            Err(ReaderError::Api(_))
        ));
    }

    #[tokio::test]
    async fn a_read_address_on_another_host_is_not_the_readers() {
        let (service, executor) = service();
        executor.push(error(404, "post_not_found"));
        executor.push(error(404, "no_such_feed"));

        let link = service
            .resolve_link("https://example.com/read/blogs/123".to_string())
            .await
            .unwrap();

        assert_eq!(link, not_in_reader(ReaderLinkNotInReader::NothingThere));
        assert!(paths(&executor)[0].starts_with("/wpcom/v2/mobile/resolve-reader-url"));
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
            "https://public-api.wordpress.com/rest/v1.3/read/menu?"
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
        assert!(sent[0].url.ends_with("/wpcom/v2/read/interests?"));
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
        // The sites the reader follows are asked for too: the server leaves them out by default.
        assert_eq!(
            queries(&executor),
            ["q=photography&offset=20&exclude_followed=false&meta=site%2Cfeed"]
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

    /// A comment by user `author`.
    fn comment_by(id: i64, hour: u32, parent: Option<i64>, author: u64) -> Value {
        let mut comment = comment_json(id, hour, parent);
        comment["author"]["ID"] = json!(author);
        comment
    }

    #[tokio::test]
    async fn a_blocked_users_comments_and_the_replies_under_them_are_hidden_until_unblocked() {
        let (service, executor) = service();
        let comments = service.comments(SITE, POST);
        // 2 is by user 8, 3 answers it and 4 answers 3; 1 and 5 aren't under it.
        executor.push(Step::Respond(
            200,
            comments_page(
                5,
                vec![
                    comment_by(5, 14, Some(1), 9),
                    comment_by(4, 13, Some(3), 9),
                    comment_by(3, 12, Some(2), 9),
                    comment_by(2, 11, None, 8),
                    comment_by(1, 10, None, 9),
                ],
            ),
        ));
        comments.refresh().await.unwrap();

        service.set_user_blocked(WpComUserId(8), true).unwrap();

        assert_eq!(ids(&comments).await, ["1", "5"]);
        let info = comments.info().unwrap();
        assert_eq!(info.hidden, 3);
        // The server's count still has them, and the cache has every comment.
        assert_eq!(info.found, Some(5));
        assert!(!info.has_more);
        assert!(
            comments
                .load_more()
                .await
                .is_ok_and(|page| page.comment_count == 0)
        );

        // Unblocking shows them again, with nothing fetched.
        service.set_user_blocked(WpComUserId(8), false).unwrap();
        assert_eq!(ids(&comments).await, ["1", "2", "3", "4", "5"]);
        assert_eq!(comments.info().unwrap().hidden, 0);
        assert_eq!(executor.sent().len(), 1);
    }

    #[tokio::test]
    async fn a_page_fetched_while_a_user_is_blocked_leaves_out_their_comments() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        // Blocking works signed out: the list is the device's.
        service.set_user_blocked(WpComUserId(8), true).unwrap();
        let comments = service.comments(SITE, POST);
        executor.push(Step::Respond(
            200,
            comments_page(
                2,
                vec![comment_by(2, 11, None, 8), comment_by(1, 10, None, 9)],
            ),
        ));

        comments.refresh().await.unwrap();

        assert_eq!(ids(&comments).await, ["1"]);
        assert_eq!(comments.info().unwrap().hidden, 1);
    }

    #[tokio::test]
    async fn the_comments_reload_when_a_user_is_blocked_or_unblocked() {
        let (service, _) = service();
        let comments = service.comments(SITE, POST);
        let hook = |table| wp_mobile_cache::UpdateHook {
            action: wp_mobile_cache::HookAction::Insert,
            db_name: "main".to_string(),
            table,
            row_id: 1,
        };
        for table in [
            wp_mobile_cache::DbTable::ReaderComments,
            wp_mobile_cache::DbTable::ReaderCommentThreads,
            wp_mobile_cache::DbTable::ReaderBlockedUsers,
        ] {
            assert!(comments.is_relevant_update(&hook(table)), "{table:?}");
        }
        assert!(!comments.is_relevant_update(&hook(wp_mobile_cache::DbTable::ReaderBlockedSites)));
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

        assert!(matches!(result, Err(ReaderError::FeedPost)));
        assert!(executor.sent().is_empty());
        assert_eq!(service.pending_action_count().unwrap(), 0);
    }

    #[tokio::test]
    async fn a_site_post_with_no_post_id_cannot_be_liked_for_missing_its_ids() {
        let (service, executor) = service();
        let post = ReaderPostSaveIdentity {
            id: 0,
            site_id: Some(WpComSiteId(123)),
            feed_id: None,
            feed_item_id: None,
            is_external: false,
            url: None,
        };

        let result = service.set_post_liked(post, true).await;

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

    /// Site 123 as the server describes it: followed or not, by so many subscribers.
    fn site_json(following: bool, subscribers: u64) -> Value {
        let mut site: Value = serde_json::from_str(include_str!(
            "../../../../wp_api/tests/wpcom/reader_sites/site_followed.json"
        ))
        .unwrap();
        site["ID"] = json!(123);
        site["URL"] = json!("https://site.example");
        site["is_following"] = json!(following);
        site["subscribers_count"] = json!(subscribers);
        site
    }

    /// Whether the cached site says it's followed, and by how many.
    fn site_shown(service: &ReaderService) -> (Option<bool>, Option<u64>) {
        let site = service.cached_site(Some(SITE), None).unwrap().unwrap();
        (site.is_following, site.subscribers_count)
    }

    #[tokio::test]
    async fn a_follow_and_an_unfollow_move_the_sites_subscriber_count() {
        let (service, executor) = service();
        executor.push(Step::Respond(200, site_json(false, 4)));
        service.fetch_site(SITE).await.unwrap();
        assert_eq!(site_shown(&service), (Some(false), Some(4)));

        executor.push(Step::Respond(
            200,
            json!({ "subscribed": true, "subscription": subscription_json() }),
        ));
        service.set_followed(target(), true).await.unwrap();
        assert_eq!(site_shown(&service), (Some(true), Some(5)));

        // Following what's followed already doesn't count the reader twice.
        executor.push(Step::Respond(
            200,
            json!({ "subscribed": true, "subscription": subscription_json() }),
        ));
        service.set_followed(target(), true).await.unwrap();
        assert_eq!(site_shown(&service), (Some(true), Some(5)));

        executor.push(Step::Respond(
            200,
            json!({ "success": true, "is_following": false }),
        ));
        service.set_followed(target(), false).await.unwrap();
        assert_eq!(site_shown(&service), (Some(false), Some(4)));

        // Nor does unfollowing what isn't followed.
        executor.push(refused("are_not_subscribed"));
        service.set_followed(target(), false).await.unwrap();
        assert_eq!(site_shown(&service), (Some(false), Some(4)));
    }

    #[tokio::test]
    async fn a_refused_follow_puts_the_sites_subscriber_count_back() {
        let (service, executor) = service();
        executor.push(Step::Respond(200, site_json(false, 4)));
        service.fetch_site(SITE).await.unwrap();
        let (release, held) = oneshot::channel();
        executor.push(Step::Hold(
            held,
            400,
            json!({ "error": "unauthorized", "message": "" }),
        ));

        let (result, ()) = futures::join!(service.set_followed(target(), true), async {
            assert_eq!(site_shown(&service), (Some(true), Some(5)));
            release.send(()).unwrap();
        });

        assert!(matches!(result, Err(ReaderError::Api(_))));
        assert_eq!(site_shown(&service), (Some(false), Some(4)));
    }

    #[tokio::test]
    async fn a_fetch_of_the_site_keeps_a_follow_that_is_waiting_counted_once() {
        let (service, executor) = service();
        executor.push(Step::Respond(200, site_json(false, 4)));
        service.fetch_site(SITE).await.unwrap();
        executor.push(Step::Offline);
        service.set_followed(target(), true).await.unwrap();
        assert_eq!(site_shown(&service), (Some(true), Some(5)));

        // The server doesn't have the follow yet: its count is without the reader.
        executor.push(Step::Respond(200, site_json(false, 4)));
        service.fetch_site(SITE).await.unwrap();
        assert_eq!(site_shown(&service), (Some(true), Some(5)));

        // Once it has it, its count is the one shown.
        executor.push(Step::Respond(
            200,
            json!({ "subscribed": true, "subscription": subscription_json() }),
        ));
        assert_eq!(service.sync_actions().await.unwrap(), 0);
        executor.push(Step::Respond(200, site_json(true, 5)));
        service.fetch_site(SITE).await.unwrap();
        assert_eq!(site_shown(&service), (Some(true), Some(5)));
    }

    #[tokio::test]
    async fn an_unblock_shows_a_subscribed_site_as_followed_again() {
        let (service, executor, stream) = reading().await;
        executor.push(Step::Respond(
            200,
            json!({
                "subscriptions": [subscription_json()],
                "page": 1,
                "number": 100,
                "total_subscriptions": 1
            }),
        ));
        service.refresh_subscriptions().await.unwrap();
        let blocked = json!({ "success": true, "modified_sub": true });
        executor.push(Step::Respond(200, blocked.clone()));
        service.set_site_blocked(SITE, true).await.unwrap();

        // While it's blocked, the server says the site isn't followed, without the reader in its
        // count, as the post it sent says too.
        executor.push(Step::Respond(200, site_json(false, 0)));
        service.fetch_site(SITE).await.unwrap();
        assert_eq!(site_shown(&service), (Some(false), Some(0)));

        // The server kept the subscription, and the unblock brings the follow back.
        executor.push(Step::Respond(200, blocked));
        service.set_site_blocked(SITE, false).await.unwrap();
        assert_eq!(site_shown(&service), (Some(true), Some(1)));
        assert_eq!(post(&stream).await.is_following, Some(true));
        assert_eq!(service.subscriptions().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn an_unblock_leaves_a_site_that_is_not_subscribed_unfollowed() {
        let (service, executor, stream) = reading().await;
        executor.push(Step::Respond(200, site_json(false, 4)));
        service.fetch_site(SITE).await.unwrap();
        let blocked = json!({ "success": true, "modified_sub": true });
        executor.push(Step::Respond(200, blocked.clone()));
        executor.push(Step::Respond(200, blocked));

        service.set_site_blocked(SITE, true).await.unwrap();
        service.set_site_blocked(SITE, false).await.unwrap();

        assert_eq!(site_shown(&service), (Some(false), Some(4)));
        assert_eq!(post(&stream).await.is_following, Some(false));
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
    async fn a_tag_the_servers_menu_lists_is_unfollowed_by_its_slug() {
        let (service, executor) = service();
        // The server's menu keys a followed tag by the tag's ID, not its slug.
        executor.push(Step::Respond(
            200,
            json!({
                "default": {},
                "subscribed": {
                    "9002": {
                        "ID": "9002",
                        "title": "Knitting",
                        "slug": "knitting",
                        "display_name": "knitting",
                        "type": "tag"
                    }
                },
                "recommended": {}
            }),
        ));
        service.refresh_menu().await.unwrap();
        let slug = ReaderTagSlug("knitting".to_string());
        let followed = |service: &ReaderService| {
            service
                .followed_tags()
                .unwrap()
                .into_iter()
                .map(|tag| (tag.key, tag.slug))
                .collect::<Vec<_>>()
        };
        let listed = vec![("9002".to_string(), Some("knitting".to_string()))];
        assert_eq!(followed(&service), listed);

        // A refused unfollow puts it back as it was.
        executor.push(refused("unauthorized"));
        let result = service.set_tag_followed(slug.clone(), None, false).await;
        assert!(matches!(result, Err(ReaderError::Api(_))));
        assert_eq!(
            followed(&service),
            [("knitting".to_string(), Some("knitting".to_string()))]
        );

        // Following a tag that's listed already doesn't list it twice.
        executor.push(Step::Respond(
            200,
            json!({ "subscribed": true, "tags": [], "added_tags": [] }),
        ));
        service
            .set_tag_followed(slug.clone(), None, true)
            .await
            .unwrap();
        assert_eq!(followed(&service).len(), 1);

        // An unfollow the server takes removes it.
        executor.push(Step::Respond(
            200,
            json!({ "subscribed": false, "tags": [], "removed_tag": 9002 }),
        ));
        let outcome = service.set_tag_followed(slug, None, false).await.unwrap();
        assert_eq!(outcome, ReaderActionOutcome::Synced);
        assert!(followed(&service).is_empty());
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
    async fn emails_started_again_take_the_frequency_the_server_answers_with() {
        let (service, executor, _) = reading().await;
        executor.push(Step::Respond(
            200,
            json!({ "subscribed": true, "subscription": subscription_json() }),
        ));
        service.set_followed(target(), true).await.unwrap();
        executor.push(Step::Respond(
            200,
            json!({ "success": true, "subscribed": true }),
        ));
        service
            .set_post_email_frequency(SITE, ReaderSubscriptionEmailFrequency::Weekly)
            .await
            .unwrap();
        let frequency =
            |service: &ReaderService| service.subscriptions().unwrap()[0].email_frequency.clone();

        executor.push(Step::Respond(
            200,
            json!({
                "success": true,
                "subscribed": true,
                "subscription": { "blog_ID": "123", "delivery_frequency": "daily", "status": "active" }
            }),
        ));
        service.set_post_emails(SITE, true).await.unwrap();
        assert_eq!(
            frequency(&service),
            Some(ReaderSubscriptionEmailFrequency::Daily)
        );

        // An answer without the emails' settings leaves the frequency as it was.
        executor.push(Step::Respond(
            200,
            json!({ "success": true, "subscribed": true }),
        ));
        service.set_post_emails(SITE, true).await.unwrap();
        assert_eq!(
            frequency(&service),
            Some(ReaderSubscriptionEmailFrequency::Daily)
        );

        // Stopping the emails doesn't take a frequency from the answer.
        executor.push(Step::Respond(
            200,
            json!({ "success": true, "subscribed": false, "subscription": null }),
        ));
        service.set_post_emails(SITE, false).await.unwrap();
        assert_eq!(
            frequency(&service),
            Some(ReaderSubscriptionEmailFrequency::Daily)
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
    async fn a_comment_the_server_takes_adds_one_to_the_posts_count() {
        let (service, executor, comments) = with_comments().await;
        let stream = service.stream(ReaderStream::Following);
        let mut post = post_json(7, 10);
        post["discussion"] = json!({ "comment_count": 1, "comments_open": true });
        executor.push(Step::Respond(200, json!({ "posts": [post] })));
        stream.refresh().await.unwrap();
        let count = || async { stream.load_items().await.unwrap()[0].post.comment_count };
        assert_eq!(count().await, Some(1));

        executor.push(Step::Respond(200, comment_json(2, 23, None)));
        comments
            .post_comment("Nice post!".to_string(), None)
            .await
            .unwrap();
        assert_eq!(count().await, Some(2));

        // One held for a moderator isn't in the count yet.
        let mut held = comment_json(3, 23, Some(2));
        held["status"] = json!("unapproved");
        executor.push(Step::Respond(200, held));
        comments
            .post_comment("I agree".to_string(), Some(CommentId(2)))
            .await
            .unwrap();
        assert_eq!(count().await, Some(2));

        // One that isn't taken isn't counted, and is counted once it's taken on "Send again".
        executor.push(Step::Offline);
        let result = comments.post_comment("Me too".to_string(), None).await;
        assert!(matches!(result, Err(ReaderError::Api(_))));
        assert_eq!(count().await, Some(2));
        let local_id = comments
            .load_items()
            .await
            .unwrap()
            .into_iter()
            .find_map(|comment| comment.local_id)
            .unwrap();
        executor.push(Step::Respond(200, comment_json(4, 23, None)));
        comments.retry_comment(local_id).await.unwrap();
        assert_eq!(count().await, Some(3));
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

/// What the example apps had to work around, and no longer do.
mod for_the_apps {
    use super::*;
    use rstest::rstest;
    use std::sync::Mutex;
    use std::time::SystemTime;
    use wp_api::wp_com::reader_saved::ReaderPostSaveIdentity;
    use wp_api::wp_com::reader_tags::{ReaderMenuItem, ReaderMenuItemKind};

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

    #[tokio::test]
    async fn a_stream_says_which_of_its_posts_are_saved() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        let stream = service.stream(tag());
        executor.push(Step::Respond(200, tag_page(&[1, 2], None)));
        stream.refresh().await.unwrap();
        let saved_ids = |items: Vec<ReaderStreamItem>| -> Vec<u64> {
            items
                .into_iter()
                .filter(|item| item.is_saved)
                .map(|item| item.post.identity.id)
                .collect()
        };
        assert_eq!(
            saved_ids(stream.load_items().await.unwrap()),
            Vec::<u64>::new()
        );

        service.saved().save(identity(2)).await.unwrap();
        assert_eq!(saved_ids(stream.load_items().await.unwrap()), [2]);

        service.saved().unsave(identity(2)).await.unwrap();
        assert_eq!(
            saved_ids(stream.load_items().await.unwrap()),
            Vec::<u64>::new()
        );
    }

    /// A post fetched on its own doesn't say when it was featured, which is what Freshly Pressed
    /// is ordered by. It used to leave the unreadable list and never reach the stream.
    #[tokio::test]
    async fn a_freshly_pressed_post_that_reads_on_its_own_takes_the_place_of_its_publish_date() {
        let executor = Arc::new(ScriptedExecutor::default());
        let now = Arc::new(Mutex::new(SystemTime::now()));
        let clock = {
            let now = now.clone();
            Arc::new(move || *now.lock().unwrap())
        };
        let service = ReaderService::with_clock(
            delegate(executor.clone()),
            cache(),
            ReaderOwner::SignedOut,
            ReaderSavedSource::Ios,
            clock,
        )
        .unwrap();
        let stream = service.stream(ReaderStream::FreshlyPressed);
        let featured = |id: u64, written: u32, displayed: u32| {
            let mut post = post_json(id, written);
            post["editorial"] = json!({ "displayed_on": date(displayed) });
            post
        };
        let mut unreadable = featured(2, 11, 21);
        unreadable["date"] = json!("2026-11-18T11:00:00+91:00");
        executor.push(Step::Respond(
            200,
            json!({ "posts": [featured(3, 12, 22), unreadable, featured(1, 10, 20)] }),
        ));
        stream.refresh().await.unwrap();
        assert_eq!(shown(&stream).await, ["3", "1"]);
        assert_eq!(stream.info().unwrap().unreadable_post_count, 1);

        *now.lock().unwrap() += Duration::from_secs(61 * 60);
        // On its own the post reads, and comes without `editorial`.
        executor.push(Step::Respond(200, post_json(2, 11)));
        assert_eq!(stream.retry_unreadable_posts().await.unwrap(), 1);

        assert_eq!(stream.info().unwrap().unreadable_post_count, 0);
        // Written at 11:00, it sorts under the posts featured at 22:00 and 20:00.
        assert_eq!(shown(&stream).await, ["3", "1", "2"]);
    }

    #[tokio::test]
    async fn a_signed_out_readers_posts_have_no_seen_status() {
        let seen_of = |owner: ReaderOwner| async move {
            let (service, executor) = service_for(owner);
            let stream = service.stream(tag());
            let mut post = post_json(1, 10);
            post["is_seen"] = json!(false);
            executor.push(Step::Respond(200, json!({ "posts": [post] })));
            stream.refresh().await.unwrap();
            stream.load_items().await.unwrap()[0].post.is_seen
        };

        assert_eq!(seen_of(ACCOUNT).await, Some(false));
        assert_eq!(seen_of(ReaderOwner::SignedOut).await, None);
    }

    #[tokio::test]
    async fn signed_out_a_follow_is_kept_and_an_unfollow_is_removed() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        let slug = ReaderTagSlug("knitting".to_string());
        let reason = |outcome: ReaderActionOutcome| match outcome {
            ReaderActionOutcome::Pending { reason } => reason,
            other => panic!("expected a pending outcome, got {other:?}"),
        };

        let followed = service
            .set_tag_followed(slug.clone(), None, true)
            .await
            .unwrap();
        let unfollowed = service.set_tag_followed(slug, None, false).await.unwrap();

        assert!(reason(followed).contains("kept on this device"));
        assert!(reason(unfollowed).contains("removed from this device"));
        assert!(executor.sent().is_empty());
    }

    fn menu_item(
        kind: ReaderMenuItemKind,
        key: &str,
        slug: Option<&str>,
        url: Option<&str>,
    ) -> ReaderMenuItem {
        ReaderMenuItem {
            key: key.to_string(),
            kind,
            title: None,
            url: url.map(str::to_string),
            slug: slug.map(str::to_string),
            display_name: None,
            tag_id: None,
            organization_id: None,
        }
    }

    const LIST_URL: &str =
        "https://public-api.wordpress.com/rest/v1.2/read/list/someone/good-reads/posts";

    #[rstest]
    #[case(
        menu_item(ReaderMenuItemKind::Stream, "following", None, None),
        Some(ReaderStream::Following)
    )]
    #[case(
        menu_item(ReaderMenuItemKind::Stream, "liked", None, None),
        Some(ReaderStream::Liked)
    )]
    #[case(
        menu_item(ReaderMenuItemKind::Stream, "discover", None, None),
        Some(ReaderStream::Discover { tags: Vec::new() })
    )]
    #[case(
        menu_item(ReaderMenuItemKind::Stream, "something-new", None, None),
        None
    )]
    #[case(
        menu_item(ReaderMenuItemKind::Tag, "1234", Some("photography"), None),
        Some(tag())
    )]
    #[case(menu_item(ReaderMenuItemKind::Tag, "1234", None, None), None)]
    #[case(
        menu_item(ReaderMenuItemKind::List, "list-good-reads", None, Some(LIST_URL)),
        Some(ReaderStream::ReaderList {
            owner: ReaderListOwner("someone".to_string()),
            slug: ReaderListSlug("good-reads".to_string()),
        })
    )]
    #[case(
        menu_item(
            ReaderMenuItemKind::List,
            "list-good-reads",
            None,
            Some("https://example.com/read/list/someone")
        ),
        None
    )]
    #[case(
        menu_item(ReaderMenuItemKind::List, "list-good-reads", None, None),
        None
    )]
    #[case(
        menu_item(ReaderMenuItemKind::Organization, "a8c", Some("a8c"), None),
        Some(ReaderStream::Automattic)
    )]
    #[case(
        menu_item(ReaderMenuItemKind::Organization, "p2", Some("p2"), None),
        Some(ReaderStream::P2)
    )]
    #[case(menu_item(ReaderMenuItemKind::Other("widget".to_string()), "w", None, None), None)]
    fn a_menu_entry_opens_its_stream(
        #[case] item: ReaderMenuItem,
        #[case] expected: Option<ReaderStream>,
    ) {
        assert_eq!(reader_stream_for_menu_item(&item), expected);
    }

    #[test]
    fn an_error_has_words_for_an_app_to_show() {
        assert_eq!(
            reader_error_message(&ReaderError::NeedsAccount),
            "Sign in to WordPress.com to use this."
        );
        assert_eq!(
            reader_error_message(&ReaderError::Refused {
                reason: "no feed there".to_string()
            }),
            "The server didn't accept it: no feed there"
        );
    }

    #[rstest]
    #[case(ReaderData::Menu, wp_mobile_cache::DbTable::ReaderMenuItems, true)]
    #[case(ReaderData::Menu, wp_mobile_cache::DbTable::ReaderPosts, false)]
    #[case(
        ReaderData::Subscriptions,
        wp_mobile_cache::DbTable::ReaderSubscriptions,
        true
    )]
    #[case(ReaderData::Blocks, wp_mobile_cache::DbTable::ReaderBlockedUsers, true)]
    #[case(
        ReaderData::PendingActions,
        wp_mobile_cache::DbTable::ReaderActions,
        true
    )]
    #[case(
        ReaderData::PendingActions,
        wp_mobile_cache::DbTable::ReaderSavedPosts,
        true
    )]
    #[case(ReaderData::Post, wp_mobile_cache::DbTable::ReaderPosts, true)]
    #[case(ReaderData::Site, wp_mobile_cache::DbTable::ReaderSites, true)]
    #[case(ReaderData::Site, wp_mobile_cache::DbTable::ReaderSubscriptions, false)]
    fn a_change_is_relevant_to_what_shows_it(
        #[case] data: ReaderData,
        #[case] table: wp_mobile_cache::DbTable,
        #[case] expected: bool,
    ) {
        let hook = wp_mobile_cache::UpdateHook {
            action: wp_mobile_cache::HookAction::Update,
            db_name: "main".to_string(),
            table,
            row_id: 1,
        };
        assert_eq!(reader_data_is_relevant_update(data, &hook), expected);
    }
}

/// The items of a feed whose site the API no longer serves: `ID: 0`, `is_external: false`, the
/// site's ID, and a feed item ID.
mod feed_items_with_no_post_id {
    use super::*;
    use wp_api::wp_com::reader_saved::{ReaderFeedItemId, ReaderPostSaveIdentity};

    const FEED: ReaderFeedId = ReaderFeedId(100000002);

    /// A page of the feed, with items 100000003 and 100000004.
    pub(super) fn page() -> Value {
        serde_json::from_str(include_str!(
            "../../../../wp_api/tests/wpcom/reader_posts/feed_posts_of_a_site_gone.json"
        ))
        .unwrap()
    }

    pub(super) fn key(feed_item_id: u64) -> ReaderSavedKey {
        ReaderSavedKey::FeedItem {
            feed_item_id: ReaderFeedItemId(feed_item_id),
        }
    }

    fn identity(feed_item_id: u64) -> ReaderPostSaveIdentity {
        ReaderPostSaveIdentity {
            id: 0,
            site_id: Some(WpComSiteId(100000001)),
            feed_id: Some(FEED),
            feed_item_id: Some(ReaderFeedItemId(feed_item_id)),
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
    async fn a_stream_keeps_them_by_their_feed_items() {
        let (service, executor) = service();
        let stream = service.stream(ReaderStream::Feed { feed_id: FEED });
        executor.push(Step::Respond(200, page()));

        let result = stream.refresh().await.unwrap();

        assert_eq!(result.post_count, 2);
        assert_eq!(result.unreadable_post_count, 0);
        let keys: Vec<_> = stream
            .load_items()
            .await
            .unwrap()
            .into_iter()
            .map(|item| item.post.key)
            .collect();
        assert_eq!(keys, [key(100000003), key(100000004)]);
        assert!(service.cached_post(key(100000003)).unwrap().is_some());
    }

    #[tokio::test]
    async fn one_is_fetched_from_its_feed() {
        let (service, executor) = service_for(ReaderOwner::SignedOut);
        executor.push(Step::Respond(200, page()["posts"][0].clone()));

        let post = service.fetch_post(identity(100000003)).await.unwrap();

        assert_eq!(post.key, key(100000003));
        assert_eq!(
            paths(&executor),
            ["/rest/v1.3/read/feed/100000002/posts/100000003?meta=site%2Cfeed"]
        );
    }

    #[tokio::test]
    async fn one_is_marked_as_seen_by_its_feed() {
        let (service, executor) = service();
        executor.push(Step::Respond(200, json!({ "status": true })));

        service
            .set_post_seen(identity(100000003), true)
            .await
            .unwrap();

        let sent = executor.sent();
        assert_eq!(
            sent[0]
                .url
                .trim_start_matches("https://public-api.wordpress.com"),
            "/wpcom/v2/seen-posts/seen/new"
        );
        assert_eq!(
            sent[0].body,
            Some(
                json!({ "feed_id": 100000002, "feed_item_ids": [100000003], "source": "reader-ios" })
            )
        );
    }

    #[tokio::test]
    async fn a_seen_mark_the_server_answers_no_to_is_refused_and_put_back() {
        let (service, executor) = service();
        let stream = service.stream(ReaderStream::Feed { feed_id: FEED });
        executor.push(Step::Respond(200, page()));
        stream.refresh().await.unwrap();
        executor.push(Step::Respond(200, json!({ "status": false })));

        let outcome = service.set_post_seen(identity(100000003), true).await;

        assert!(
            matches!(&outcome, Err(ReaderError::Refused { reason }) if reason == "The server didn't mark the post as seen."),
            "{outcome:?}"
        );
        let post = service.cached_post(key(100000003)).unwrap().unwrap();
        assert_eq!(post.is_seen, Some(false));
        assert_eq!(service.pending_action_count().unwrap(), 0);
    }

    #[tokio::test]
    async fn one_cannot_be_liked_or_have_its_comments_followed() {
        let (service, executor) = service();

        let liked = service.set_post_liked(identity(100000003), true).await;
        let followed = service
            .set_comments_subscribed(identity(100000003), true)
            .await;
        let notified = service
            .set_comment_notifications(identity(100000003), true)
            .await;

        for outcome in [&liked, &followed, &notified] {
            let Err(error) = outcome else {
                panic!("{outcome:?}");
            };
            assert!(matches!(error, ReaderError::FeedPost), "{error:?}");
            assert_eq!(
                reader_error_message(error),
                "A feed's post can't be liked, and its comments can't be followed."
            );
        }
        assert!(executor.sent().is_empty());
        assert_eq!(service.pending_action_count().unwrap(), 0);
    }
}
