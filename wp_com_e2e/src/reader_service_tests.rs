//! `ReaderService` against the live API: streams fetched, kept in a cache, and paged.

use libtest_mimic::{Failed, Trial};
use std::collections::HashSet;
use std::sync::Arc;
use wp_api::posts::PostId;
use wp_api::prelude::*;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::reader_posts::{
    ReaderListOwner, ReaderListSlug, ReaderSearchSort, ReaderTagSlug,
};
use wp_api::wp_com::reader_saved::{ReaderSavedKey, ReaderSavedSource};
use wp_mobile::service::reader::{
    ReaderActionOutcome, ReaderError, ReaderService, ReaderStream, ReaderStreamCollection,
    ReaderStreamItem,
};
use wp_mobile_cache::WpApiCache;
use wp_mobile_cache::repository::wordpress_com_accounts::ReaderOwner;

use crate::context::TestContext;

/// The WordPress.com Blog, which is public and has years of posts.
const SITE: WpComSiteId = WpComSiteId(3584907);

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    vec![
        Trial::test("reader_service::following_stream", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    let stream = service.stream(ReaderStream::Following);
                    let page = stream.refresh().await.map_err(|e| e.to_string())?;
                    let items = stream.load_items().await.map_err(|e| e.to_string())?;
                    if items.len() as u64 != page.post_count {
                        return Err("The stream doesn't have the posts the page had".into());
                    }
                    check_newest_first(&items)?;
                    Ok(())
                })
            }
        }),
        Trial::test("reader_service::site_stream_signed_out", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let service = new_service(signed_out(&ctx.delegate), ReaderOwner::SignedOut)?;
                    let stream = service.stream(ReaderStream::Site { site_id: SITE });

                    let first = stream.refresh().await.map_err(|e| e.to_string())?;
                    if first.post_count == 0 || !first.has_more {
                        return Err("The site's first page should have posts, and more".into());
                    }
                    let more = stream.load_more().await.map_err(|e| e.to_string())?;
                    if more.new_post_count == 0 {
                        return Err("The second page should have posts the first didn't".into());
                    }
                    let items = check_stream(&stream).await?;
                    if items.len() as u64 != first.post_count + more.new_post_count {
                        return Err("The stream should have both pages' posts".into());
                    }
                    check_newest_first(&items)?;
                    if items.iter().any(|item| item.post.content.is_some()) {
                        return Err("A stream's items come without their content".into());
                    }
                    if items.iter().any(|item| item.post.title.is_none()) {
                        return Err("A site post is missing its title".into());
                    }

                    // A second refresh reaches the posts the cache has, so it leaves no gap and
                    // keeps both pages.
                    stream.refresh().await.map_err(|e| e.to_string())?;
                    let again = check_stream(&stream).await?;
                    if again.len() < items.len() {
                        return Err("A refresh shouldn't drop the pages already loaded".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_service::tag_stream_signed_out", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let service = new_service(signed_out(&ctx.delegate), ReaderOwner::SignedOut)?;
                    let stream = service.stream(ReaderStream::Tag {
                        slug: ReaderTagSlug("photography".to_string()),
                    });

                    let first = stream.refresh().await.map_err(|e| e.to_string())?;
                    if first.post_count == 0 || !first.has_more {
                        return Err("The tag's first page should have posts, and more".into());
                    }
                    let more = stream.load_more().await.map_err(|e| e.to_string())?;
                    if more.new_post_count == 0 {
                        return Err("The second page should have posts the first didn't".into());
                    }
                    let items = check_stream(&stream).await?;
                    if items.len() as u64 != first.post_count + more.new_post_count {
                        return Err("The stream should have both pages' posts".into());
                    }

                    // A refresh starts the stream again from its first page.
                    let refreshed = stream.refresh().await.map_err(|e| e.to_string())?;
                    let items = check_stream(&stream).await?;
                    if items.len() as u64 != refreshed.post_count {
                        return Err("A refreshed tag stream should have only its first page".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_service::every_public_stream_signed_out", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let service = new_service(signed_out(&ctx.delegate), ReaderOwner::SignedOut)?;
                    let tag = |slug: &str| ReaderTagSlug(slug.to_string());
                    let streams = [
                        ReaderStream::FreshlyPressed,
                        ReaderStream::Tags {
                            slugs: vec![tag("photography"), tag("travel")],
                        },
                        ReaderStream::Search {
                            query: "wordpress".to_string(),
                            sort: ReaderSearchSort::Relevance,
                        },
                        ReaderStream::Search {
                            query: "wordpress".to_string(),
                            sort: ReaderSearchSort::Date,
                        },
                        ReaderStream::ReaderList {
                            owner: ReaderListOwner("automattic".to_string()),
                            slug: ReaderListSlug("a12s".to_string()),
                        },
                        ReaderStream::Discover { tags: vec![] },
                        ReaderStream::Discover {
                            tags: vec![tag("photography")],
                        },
                        ReaderStream::FirstPosts,
                    ];
                    for stream in streams {
                        walk_two_pages(&service, stream).await?;
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_service::a_post_and_its_related_posts_signed_out", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let service = new_service(signed_out(&ctx.delegate), ReaderOwner::SignedOut)?;
                    let stream = service.stream(ReaderStream::Site { site_id: SITE });
                    stream.refresh().await.map_err(|e| e.to_string())?;
                    let items = stream.load_items().await.map_err(|e| e.to_string())?;
                    let listed = items.first().ok_or("The site has no posts")?.post.clone();

                    // Fetched on its own, the post is the one the stream listed, with its content.
                    let post = service
                        .fetch_post(listed.identity.clone())
                        .await
                        .map_err(|e| e.to_string())?;
                    if post.key != listed.key || post.title != listed.title {
                        return Err("The fetched post isn't the one the stream listed".into());
                    }
                    if post.content.as_deref().unwrap_or_default().is_empty() {
                        return Err("A fetched post should have its content".into());
                    }
                    let cached = service
                        .cached_post(post.key.clone())
                        .map_err(|e| e.to_string())?;
                    if cached.as_ref() != Some(&post) {
                        return Err("The fetched post should be what the cache has".into());
                    }

                    let related = service.stream(ReaderStream::Related {
                        site_id: SITE,
                        post_id: PostId(post.identity.id as i64),
                    });
                    let page = related.refresh().await.map_err(|e| e.to_string())?;
                    if page.post_count == 0 || page.has_more {
                        return Err("A post should have related posts, in one page".into());
                    }
                    check_stream(&related).await?;
                    Ok(())
                })
            }
        }),
        Trial::test("reader_service::the_accounts_streams", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    // Any of these can be empty for an account, so only what loads is checked.
                    for stream in [
                        ReaderStream::Liked,
                        ReaderStream::Automattic,
                        ReaderStream::P2,
                    ] {
                        let collection = service.stream(stream.clone());
                        let first = collection
                            .refresh()
                            .await
                            .map_err(|e| format!("{stream:?}: {e}"))?;
                        if first.has_more {
                            collection
                                .load_more()
                                .await
                                .map_err(|e| format!("{stream:?}: {e}"))?;
                        }
                        let items = check_stream(&collection).await.map_err(|e| {
                            format!("{stream:?}: {}", e.message().unwrap_or_default())
                        })?;
                        check_newest_first(&items).map_err(|e| {
                            format!("{stream:?}: {}", e.message().unwrap_or_default())
                        })?;
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_service::seen_is_shown_sent_and_put_back", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    let stream = service.stream(ReaderStream::Following);
                    stream.refresh().await.map_err(|e| e.to_string())?;
                    // The newest followed post that has a seen state. An account that follows
                    // nothing has none, and then there's nothing to try.
                    let items = stream.load_items().await.map_err(|e| e.to_string())?;
                    let Some(post) = items.iter().find(|item| item.post.is_seen.is_some()) else {
                        return Ok(());
                    };
                    let identity = post.post.identity.clone();
                    let key = post.post.key.clone();
                    let was = post.post.is_seen == Some(true);

                    let marked = service.set_post_seen(identity.clone(), !was).await;
                    let shown = seen_in(&stream, &key).await;
                    // What the server has, read by a service with a cache of its own.
                    let other = new_service(ctx.delegate.clone(), account())?;
                    let other_stream = other.stream(ReaderStream::Following);
                    let fetched = match other_stream.refresh().await {
                        Ok(_) => seen_in(&other_stream, &key).await,
                        Err(error) => Err(error.to_string().into()),
                    };
                    // Whatever happened, the post ends as it was.
                    let restored = service
                        .set_post_seen(identity, was)
                        .await
                        .map_err(|e| e.to_string())?;

                    if marked.map_err(|e| e.to_string())? != ReaderActionOutcome::Synced
                        || restored != ReaderActionOutcome::Synced
                    {
                        return Err("Marking the post should reach the server".into());
                    }
                    if shown? != Some(!was) || fetched? != Some(!was) {
                        return Err(
                            "The cache and the server should both show the new state".into()
                        );
                    }
                    if seen_in(&stream, &key).await? != Some(was) {
                        return Err("The post should be back as it was".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_service::a_block_is_sent_and_put_back", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    let site = ctx.site_id;
                    let is_blocked = |service: &ReaderService| {
                        service
                            .blocked_sites()
                            .map(|sites| sites.iter().any(|blocked| blocked.id == site))
                            .map_err(|e| Failed::from(e.to_string()))
                    };
                    service
                        .refresh_blocked_sites()
                        .await
                        .map_err(|e| e.to_string())?;
                    if is_blocked(&service)? {
                        return Err("The test site is blocked already, which isn't the trial's \
                                    doing, so it's left alone"
                            .into());
                    }

                    let blocked = service.set_site_blocked(site, true).await;
                    // What the server has, read by a service with a cache of its own.
                    let other = service_for_blocks(&ctx).await;
                    // Whatever happened, the site ends unblocked.
                    let unblocked = service
                        .set_site_blocked(site, false)
                        .await
                        .map_err(|e| e.to_string())?;

                    if blocked.map_err(|e| e.to_string())? != ReaderActionOutcome::Synced
                        || unblocked != ReaderActionOutcome::Synced
                    {
                        return Err("The block should reach the server".into());
                    }
                    if !is_blocked(&other?)? {
                        return Err(
                            "The server's list should have the site while it's blocked".into()
                        );
                    }
                    service
                        .refresh_blocked_sites()
                        .await
                        .map_err(|e| e.to_string())?;
                    if is_blocked(&service)? {
                        return Err("The site should be unblocked again".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_service::following_needs_an_account", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let service = new_service(signed_out(&ctx.delegate), ReaderOwner::SignedOut)?;
                    let stream = service.stream(ReaderStream::Following);
                    match stream.refresh().await {
                        Err(ReaderError::NeedsAccount) => Ok(()),
                        other => Err(format!("Expected NeedsAccount, got {other:?}").into()),
                    }
                })
            }
        }),
    ]
}

/// The account whose token the trials use. Each trial's cache is new, so any user ID keeps them
/// apart.
fn account() -> ReaderOwner {
    ReaderOwner::Account {
        user_id: WpComUserId(1),
    }
}

/// A service with a cache of its own, holding the server's blocked sites.
async fn service_for_blocks(ctx: &TestContext) -> Result<ReaderService, Failed> {
    let service = new_service(ctx.delegate.clone(), account())?;
    service
        .refresh_blocked_sites()
        .await
        .map_err(|e| e.to_string())?;
    Ok(service)
}

/// The seen state of a post in a stream's cached items.
async fn seen_in(
    stream: &ReaderStreamCollection,
    key: &ReaderSavedKey,
) -> Result<Option<bool>, Failed> {
    let items = stream.load_items().await.map_err(|e| e.to_string())?;
    Ok(items
        .iter()
        .find(|item| &item.post.key == key)
        .and_then(|item| item.post.is_seen))
}

/// A service with a new, empty cache.
fn new_service(delegate: WpApiClientDelegate, owner: ReaderOwner) -> Result<ReaderService, Failed> {
    let cache = WpApiCache::new(None).map_err(|e| e.to_string())?;
    cache.perform_migrations().map_err(|e| e.to_string())?;
    Ok(
        ReaderService::new(delegate, Arc::new(cache), owner, ReaderSavedSource::Ios)
            .map_err(|e| e.to_string())?,
    )
}

/// The same delegate, sending no authentication.
fn signed_out(delegate: &WpApiClientDelegate) -> WpApiClientDelegate {
    WpApiClientDelegate {
        auth_provider: Arc::new(WpAuthenticationProvider::none()),
        ..delegate.clone()
    }
}

/// Refreshes a stream, loads its next page if it has one, and checks what the cache then holds.
async fn walk_two_pages(service: &ReaderService, stream: ReaderStream) -> Result<(), Failed> {
    let collection = service.stream(stream.clone());
    let first = collection
        .refresh()
        .await
        .map_err(|e| format!("{stream:?}: {e}"))?;
    if first.post_count == 0 {
        return Err(format!("{stream:?} has no posts on its first page").into());
    }
    let mut expected = first.post_count;
    if first.has_more {
        let more = collection
            .load_more()
            .await
            .map_err(|e| format!("{stream:?}: {e}"))?;
        expected += more.new_post_count;
    }
    let items = check_stream(&collection)
        .await
        .map_err(|e| format!("{stream:?}: {}", e.message().unwrap_or_default()))?;
    if items.len() as u64 != expected {
        return Err(format!(
            "{stream:?} has {} posts in the cache, and its pages had {expected}",
            items.len()
        )
        .into());
    }
    // Every card says which post it follows, and that post is in the stream.
    let cards = collection.load_cards().await.map_err(|e| e.to_string())?;
    for card in &cards {
        if let Some(after) = &card.after_post
            && !items.iter().any(|item| &item.post.key == after)
        {
            return Err(format!("{stream:?} has a card after a post that isn't in it").into());
        }
    }
    Ok(())
}

/// The stream's items, checked for a post in it twice and for gaps a page shouldn't have left.
async fn check_stream(stream: &ReaderStreamCollection) -> Result<Vec<ReaderStreamItem>, Failed> {
    let items = stream.load_items().await.map_err(|e| e.to_string())?;
    let mut seen = HashSet::new();
    for item in &items {
        if !seen.insert(item.post.key.clone()) {
            return Err(format!("{:?} is in the stream twice", item.post.key).into());
        }
        if item.gap_before {
            return Err(format!("{:?} has a gap before it", item.post.key).into());
        }
    }
    let info = stream.info().map_err(|e| e.to_string())?;
    if info.unreadable_post_count > 0 {
        return Err(format!(
            "{} of the stream's posts couldn't be read",
            info.unreadable_post_count
        )
        .into());
    }
    Ok(items)
}

fn check_newest_first(items: &[ReaderStreamItem]) -> Result<(), Failed> {
    let dates: Vec<_> = items
        .iter()
        .filter_map(|item| item.sort_date.map(|date| date.0))
        .collect();
    if dates.len() != items.len() {
        return Err("A post in a stream ordered by date has no date".into());
    }
    if dates.windows(2).any(|pair| pair[0] < pair[1]) {
        return Err("The stream isn't newest first".into());
    }
    Ok(())
}
