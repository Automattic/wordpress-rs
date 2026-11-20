//! `ReaderService` against the live API: streams fetched, kept in a cache, and paged.

use libtest_mimic::{Failed, Trial};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use wp_api::comments::{CommentDeleteParams, CommentId};
use wp_api::posts::PostId;
use wp_api::prelude::*;
use wp_api::wp_com::endpoint::WpComDotOrgApiUrlResolver;
use wp_api::wp_com::language::WPComLanguage;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::reader_likes::ReaderLikesParams;
use wp_api::wp_com::reader_page_views::ReaderPageView;
use wp_api::wp_com::reader_posts::{
    ReaderListOwner, ReaderListSlug, ReaderSearchSort, ReaderTagSlug,
};
use wp_api::wp_com::reader_saved::{ReaderSavedKey, ReaderSavedSource};
use wp_api::wp_com::reader_subscriptions::ReaderSubscriptionEmailFrequency;
use wp_api::wp_com::stats_post::StatsPostTarget;
use wp_api::wp_com::{WpComBaseUrl, WpComSiteId};
use wp_mobile::service::reader::{
    ReaderActionOutcome, ReaderError, ReaderFollowTarget, ReaderLink, ReaderLinkNotInReader,
    ReaderOpenedPageView, ReaderOpenedSeen, ReaderPageViewChoice, ReaderPageViewOutcome,
    ReaderPostLikeResult, ReaderService, ReaderStream, ReaderStreamCollection, ReaderStreamItem,
};
use wp_mobile::service::reader_saved::ReaderOfflineState;
use wp_mobile_cache::WpApiCache;
use wp_mobile_cache::db_types::reader::ReaderPostRecord;
use wp_mobile_cache::repository::reader_comments::ReaderCommentSendState;
use wp_mobile_cache::repository::wordpress_com_accounts::ReaderOwner;

use crate::context::{TestContext, alone, changing, read_back, reading};

/// The WordPress.com Blog, which is public and has years of posts.
const SITE: WpComSiteId = WpComSiteId(3584907);
/// The WordPress.com Blog's home address.
const SITE_URL: &str = "https://en.blog.wordpress.com/";
/// One of its posts, from 2014, and the post's address.
const POST: PostId = PostId(29614);
const POST_URL: &str =
    "https://en.blog.wordpress.com/2014/12/29/new-year-new-blog-make-it-a-great-one/";
/// xkcd, a site that isn't on WordPress.com or Jetpack and has a feed.
const FEED_ONLY_SITE_URL: &str = "https://xkcd.com/";
/// The first website, which CERN keeps as it was: not on WordPress.com or Jetpack, and with no
/// feed. Not `example.com`: a private WordPress.com site has that domain, so the Reader refuses it.
const NEITHER_URL: &str = "https://info.cern.ch/";

/// The `User-Agent` a page view is sent with here. An app passes its web view's.
const PAGE_VIEW_USER_AGENT: &str = "wp-rs-e2e/1.0 reqwest";

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
        Trial::test("reader_service::tag_and_discover_in_french_signed_out", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    // One cache, so a French stream showing English posts would be a leak.
                    let service = new_service(signed_out(&ctx.delegate), ReaderOwner::SignedOut)?;
                    let streams = [
                        ReaderStream::Tag {
                            slug: ReaderTagSlug("photography".to_string()),
                        },
                        ReaderStream::Discover { tags: vec![] },
                    ];
                    for stream in streams {
                        service.set_language(None);
                        let english = refreshed_post_ids(&service, &stream).await?;
                        service.set_language(Some(WPComLanguage::French));
                        let french_stream = service.stream(stream.clone());
                        if french_stream.language() != Some(WPComLanguage::French) {
                            return Err(format!("{stream:?} isn't in French").into());
                        }
                        let french = refreshed_post_ids(&service, &stream).await?;
                        if english.is_empty() || french.is_empty() {
                            return Err(format!("{stream:?} has no posts in a language").into());
                        }
                        // The server picks other posts in French. Live data moves, so a few in
                        // both are allowed: yesterday's probes had none.
                        let shared = french.intersection(&english).count();
                        if shared * 2 > french.len() {
                            return Err(format!(
                                "{stream:?}: {shared} of the {} French posts are English ones",
                                french.len()
                            )
                            .into());
                        }
                        // Each language's posts are cached apart.
                        service.set_language(None);
                        if cached_post_ids(&service, &stream).await? != english {
                            return Err(format!("{stream:?} in English changed").into());
                        }
                        service.set_language(Some(WPComLanguage::FrenchCanada));
                        if cached_post_ids(&service, &stream).await? != french {
                            return Err(format!("{stream:?} isn't French for Canada").into());
                        }
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
                let _follows = reading(&ctx.locks.follows);
                let _seen = alone(&ctx.locks.seen);
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
                    let fetched = seen_on_the_server(&ctx, &key, Some(!was)).await;
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
                let _follows = changing(&ctx.locks.follows);
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
                    settle_after_a_block().await;
                    let listed = read_back(
                        async || is_blocked(&service_for_blocks(&ctx).await?),
                        |listed| *listed,
                    )
                    .await;
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
                    if !listed? {
                        return Err(
                            "The server's list should have the site while it's blocked".into()
                        );
                    }
                    settle_after_a_block().await;
                    let listed = read_back(
                        async || {
                            service
                                .refresh_blocked_sites()
                                .await
                                .map_err(|e| e.to_string())?;
                            is_blocked(&service)
                        },
                        |listed| !*listed,
                    )
                    .await?;
                    if listed {
                        return Err("The site should be unblocked again".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_service::menu_sites_subscriptions_and_comments", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _follows = reading(&ctx.locks.follows);
                ctx.runtime.block_on(async {
                    // Signed out: the menu's default streams and recommended tags, a site's info,
                    // and a post's comments.
                    let signed_out =
                        new_service(signed_out(&ctx.delegate), ReaderOwner::SignedOut)?;
                    signed_out.refresh_menu().await.map_err(|e| e.to_string())?;
                    let menu = signed_out.menu().map_err(|e| e.to_string())?;
                    if menu.items.is_empty() || menu.recommended_tags.is_empty() {
                        return Err("Signed out, the menu should have streams and tags".into());
                    }
                    if !menu.followed_tags.is_empty() {
                        return Err("A new signed-out reader follows no tags".into());
                    }
                    let site = signed_out
                        .fetch_site(SITE)
                        .await
                        .map_err(|e| e.to_string())?;
                    if site.name.is_none() || site.url.is_none() {
                        return Err("A site should have its name and address".into());
                    }
                    let cached = signed_out
                        .cached_site(Some(SITE), None)
                        .map_err(|e| e.to_string())?;
                    if cached != Some(site) {
                        return Err("The fetched site should be what the cache has".into());
                    }
                    if signed_out
                        .interests()
                        .await
                        .map_err(|e| e.to_string())?
                        .is_empty()
                    {
                        return Err("There should be interests to pick from".into());
                    }

                    // A post of the site that has comments.
                    let stream = signed_out.stream(ReaderStream::Site { site_id: SITE });
                    stream.refresh().await.map_err(|e| e.to_string())?;
                    let items = stream.load_items().await.map_err(|e| e.to_string())?;
                    let commented = items
                        .iter()
                        .find(|item| item.post.comment_count.unwrap_or(0) > 0)
                        .ok_or("None of the site's newest posts has comments")?;
                    let comments =
                        signed_out.comments(SITE, PostId(commented.post.identity.id as i64));
                    let page = comments.refresh().await.map_err(|e| e.to_string())?;
                    let loaded = comments.load_items().await.map_err(|e| e.to_string())?;
                    if page.comment_count == 0 || loaded.len() as u64 != page.comment_count {
                        return Err("The post's comments should be in the cache".into());
                    }
                    if loaded.iter().any(|comment| comment.comment_id.is_none()) {
                        return Err("A comment from the server has its ID".into());
                    }
                    if loaded
                        .windows(2)
                        .any(|pair| match (pair[0].date, pair[1].date) {
                            (Some(first), Some(second)) => first.0 > second.0,
                            _ => false,
                        })
                    {
                        return Err("Comments should be listed oldest first".into());
                    }

                    // Signed in: the menu with the account's tags, and its subscriptions.
                    let service = new_service(ctx.delegate.clone(), account())?;
                    service.refresh_menu().await.map_err(|e| e.to_string())?;
                    let menu = service.menu().map_err(|e| e.to_string())?;
                    if !menu.items.iter().any(|item| item.key == "following") {
                        return Err("Signed in, the menu should have the Following stream".into());
                    }
                    let count = service
                        .refresh_subscriptions()
                        .await
                        .map_err(|e| e.to_string())?;
                    let subscriptions = service.subscriptions().map_err(|e| e.to_string())?;
                    if subscriptions.len() as u64 != count {
                        return Err("The subscriptions should be in the cache".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_service::a_tag_follow_is_sent_and_put_back", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    let slug = ReaderTagSlug("knitting".to_string());
                    let follows = |service: &ReaderService| {
                        service
                            .followed_tags()
                            .map(|tags| {
                                tags.iter()
                                    .any(|tag| tag.slug.as_deref() == Some("knitting"))
                            })
                            .map_err(|e| Failed::from(e.to_string()))
                    };
                    service.refresh_menu().await.map_err(|e| e.to_string())?;
                    let was = follows(&service)?;

                    let changed = service.set_tag_followed(slug.clone(), None, !was).await;
                    // What the server has, read by a service with a cache of its own.
                    let fetched = read_back(
                        async || {
                            let other = new_service(ctx.delegate.clone(), account())?;
                            other.refresh_menu().await.map_err(|e| e.to_string())?;
                            follows(&other)
                        },
                        |follows| *follows != was,
                    )
                    .await;
                    // Whatever happened, the tag ends as it was.
                    let restored = service
                        .set_tag_followed(slug, None, was)
                        .await
                        .map_err(|e| e.to_string())?;

                    if changed.map_err(|e| e.to_string())? != ReaderActionOutcome::Synced
                        || restored != ReaderActionOutcome::Synced
                    {
                        return Err("The tag follow should reach the server".into());
                    }
                    if fetched? == was {
                        return Err("The server's menu should show the change".into());
                    }
                    let back = read_back(
                        async || {
                            service.refresh_menu().await.map_err(|e| e.to_string())?;
                            follows(&service)
                        },
                        |follows| *follows == was,
                    )
                    .await?;
                    if back != was {
                        return Err("The tag should be back as it was".into());
                    }
                    Ok(())
                })
            }
        }),
        // Follows and unfollows the site in `WP_COM_E2E_OWN_SITE_ID` when it's set, and the
        // trial's site otherwise.
        Trial::test("reader_service::a_follow_is_sent_and_put_back", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _follows = changing(&ctx.locks.follows);
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    let site_id = own_site().unwrap_or(ctx.site_id);
                    let site = service
                        .fetch_site(site_id)
                        .await
                        .map_err(|e| e.to_string())?;
                    let was = site.is_following == Some(true);
                    // A followed site's emails and notifications, to put back after it's
                    // followed again.
                    let delivery = if was {
                        Some(delivery_on_the_server(&ctx, site_id).await?)
                    } else {
                        None
                    };
                    let target = ReaderFollowTarget {
                        site_id: Some(site_id),
                        feed_id: site.feed_id,
                        url: site.url.clone(),
                        name: site.name.clone(),
                    };
                    let follows = |service: &ReaderService| {
                        service
                            .subscriptions()
                            .map(|all| all.iter().any(|one| one.site_id == Some(site_id)))
                            .map_err(|e| Failed::from(e.to_string()))
                    };

                    let changed = service.set_followed(target.clone(), !was).await;
                    let shown = follows(&service);
                    // What the server has, read by a service with a cache of its own.
                    let fetched = read_back(
                        async || {
                            let other = new_service(ctx.delegate.clone(), account())?;
                            other
                                .refresh_subscriptions()
                                .await
                                .map_err(|e| e.to_string())?;
                            follows(&other)
                        },
                        |follows| *follows != was,
                    )
                    .await;
                    // Whatever happened, the site ends as it was.
                    let restored = service
                        .set_followed(target, was)
                        .await
                        .map_err(|e| e.to_string())?;
                    // A follow can come back with the server's default emails and
                    // notifications, so those are put back as they were found too.
                    if let Some(delivery) = &delivery {
                        service
                            .refresh_subscriptions()
                            .await
                            .map_err(|e| e.to_string())?;
                        restore_delivery(&ctx, &service, site_id, delivery).await?;
                    }

                    if changed.map_err(|e| e.to_string())? != ReaderActionOutcome::Synced
                        || restored != ReaderActionOutcome::Synced
                    {
                        return Err("The follow should reach the server".into());
                    }
                    if shown? == was || fetched? == was {
                        return Err("The cache and the server should both show the change".into());
                    }
                    let back = read_back(
                        async || {
                            service
                                .refresh_subscriptions()
                                .await
                                .map_err(|e| e.to_string())?;
                            follows(&service)
                        },
                        |follows| *follows == was,
                    )
                    .await?;
                    if back != was {
                        return Err("The site should be back as it was".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_service::saved_posts_are_downloaded_for_offline", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    let directory = std::env::temp_dir()
                        .join(format!("reader-offline-{}", uuid::Uuid::new_v4()));
                    let result = download_saved_posts(&service, &directory).await;
                    // The trial's files go whether it passed or not.
                    let removed = std::fs::remove_dir_all(&directory);
                    result?;
                    removed.map_err(|e| format!("Couldn't remove {}: {e}", directory.display()))?;
                    Ok(())
                })
            }
        }),
        // Writes a public comment, so it only runs against a site the account owns, named in
        // `WP_COM_E2E_OWN_SITE_ID`. It deletes what it writes.
        Trial::test("reader_service::a_comment_is_written_shown_and_deleted", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _own_post = alone(&ctx.locks.own_post);
                let Some(site_id) = own_site() else {
                    return Err("Set WP_COM_E2E_OWN_SITE_ID to a site the account owns".into());
                };
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    let mut written = Vec::new();
                    let result = write_comments(&service, site_id, &mut written).await;
                    // Whatever happened, the comments it wrote are deleted, replies first.
                    let site = own_site_client(&ctx, site_id);
                    for comment in written.iter().rev() {
                        site.comments()
                            .delete(comment, &CommentDeleteParams::new(None))
                            .await
                            .map_err(|e| format!("Comment {} wasn't deleted: {e}", comment.0))?;
                    }
                    result
                })
            }
        })
        .with_ignored_flag(own_site().is_none()),
        // A like notifies the post's author, so it only runs against a site the account owns.
        Trial::test("reader_service::a_like_is_sent_and_put_back", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _own_post = alone(&ctx.locks.own_post);
                let Some(site_id) = own_site() else {
                    return Err("Set WP_COM_E2E_OWN_SITE_ID to a site the account owns".into());
                };
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    let post = first_post_of(&service, site_id).await?;
                    let was = server_says_post_liked(&ctx, &post).await?;

                    let result = like_and_check(&ctx, &service, &post, !was).await;
                    // Whatever happened, the post is left as it was found.
                    let restored = like_and_check(&ctx, &service, &post, was).await;
                    result?;
                    restored
                })
            }
        })
        .with_ignored_flag(own_site().is_none()),
        // Likes a comment the trial writes, so it only runs against a site the account owns.
        Trial::test("reader_service::a_comment_like_is_sent_and_put_back", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _own_post = alone(&ctx.locks.own_post);
                let Some(site_id) = own_site() else {
                    return Err("Set WP_COM_E2E_OWN_SITE_ID to a site the account owns".into());
                };
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    let post = first_post_of(&service, site_id).await?;
                    let comments = service.comments(site_id, PostId(post.identity.id as i64));
                    let comment = comments
                        .post_comment(
                            "A test comment from wordpress-rs, to like. It deletes itself."
                                .to_string(),
                            None,
                        )
                        .await
                        .map_err(|e| format!("The comment wasn't taken: {e}"))?;
                    let comment_id = comment.comment_id.ok_or("The comment has no ID")?;

                    let result = like_comment_and_check(&ctx, &service, site_id, comment_id).await;
                    // Whatever happened, the comment is deleted, and its like with it.
                    own_site_client(&ctx, site_id)
                        .comments()
                        .delete(&comment_id, &CommentDeleteParams::new(None))
                        .await
                        .map_err(|e| format!("Comment {} wasn't deleted: {e}", comment_id.0))?;
                    result
                })
            }
        })
        .with_ignored_flag(own_site().is_none()),
        // Subscribing to comments emails the account, so it only runs against a site it owns.
        Trial::test(
            "reader_service::a_comment_subscription_is_sent_and_put_back",
            {
                let ctx = Arc::clone(&ctx);
                move || {
                    let _own_post = alone(&ctx.locks.own_post);
                    let Some(site_id) = own_site() else {
                        return Err("Set WP_COM_E2E_OWN_SITE_ID to a site the account owns".into());
                    };
                    ctx.runtime.block_on(async {
                        let service = new_service(ctx.delegate.clone(), account())?;
                        let post = first_post_of(&service, site_id).await?;
                        let was = comment_subscription(&ctx, &post).await?;

                        let result = subscribe_to_comments_and_check(&ctx, &service, &post).await;
                        // Whatever happened, the subscription is left as it was found.
                        let restored =
                            restore_comment_subscription(&ctx, &service, &post, was).await;
                        result?;
                        restored
                    })
                }
            },
        )
        .with_ignored_flag(own_site().is_none()),
        // A site's emails and notifications are the account's own settings for a site it
        // follows, so this only runs against a site it owns and follows.
        Trial::test("reader_service::delivery_settings_are_sent_and_put_back", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _follows = changing(&ctx.locks.follows);
                let Some(site_id) = own_site() else {
                    return Err("Set WP_COM_E2E_OWN_SITE_ID to a site the account owns".into());
                };
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    service
                        .refresh_subscriptions()
                        .await
                        .map_err(|e| e.to_string())?;
                    let was = delivery_on_the_server(&ctx, site_id).await?;

                    let result = change_delivery_and_check(&ctx, &service, site_id, &was).await;
                    // Whatever happened, the settings are left as they were found.
                    let restored = restore_delivery(&ctx, &service, site_id, &was).await;
                    result?;
                    restored
                })
            }
        })
        .with_ignored_flag(own_site().is_none()),
        // The requests that follow and unfollow a site by its ID, which the service doesn't use
        // to follow: it follows by address. Only against a site the account owns.
        Trial::test("reader_service::a_site_is_followed_and_unfollowed_by_id", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _follows = changing(&ctx.locks.follows);
                let Some(site_id) = own_site() else {
                    return Err("Set WP_COM_E2E_OWN_SITE_ID to a site the account owns".into());
                };
                ctx.runtime.block_on(async {
                    let was = follows_by_id(&ctx, site_id).await?;

                    let result = async {
                        set_followed_by_id(&ctx, site_id, !was).await?;
                        set_followed_by_id(&ctx, site_id, was).await
                    }
                    .await;
                    // Whatever happened, the site is left as it was found.
                    if follows_by_id(&ctx, site_id).await? != was {
                        set_followed_by_id(&ctx, site_id, was).await?;
                    }
                    result
                })
            }
        })
        .with_ignored_flag(own_site().is_none()),
        // Counts one view, on a post of a site the account owns, and checks that the site's stats
        // show it.
        Trial::test("reader_service::a_page_view_of_an_own_post", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _follows = reading(&ctx.locks.follows);
                let _seen = alone(&ctx.locks.seen);
                let Some(site_id) = own_site() else {
                    return Err("Set WP_COM_E2E_OWN_SITE_ID to a site the account owns".into());
                };
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    let post = first_post_of(&service, site_id).await?;
                    let post_id = PostId(post.identity.id as i64);
                    let views_before = views_of(&ctx, site_id, post_id).await?;

                    // Opening the post also marks it as seen when it isn't: what it was is put
                    // back below.
                    let was_seen = post.is_seen;
                    let opened = service
                        .post_opened(
                            post.key.clone(),
                            ReaderPageViewChoice::Count {
                                user_agent: PAGE_VIEW_USER_AGENT.to_string(),
                            },
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                    if was_seen == Some(false) {
                        service
                            .set_post_seen(post.identity.clone(), false)
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                    if let ReaderOpenedSeen::Failed { error } = &opened.seen {
                        return Err(
                            format!("Opening the post didn't mark it as seen: {error}").into()
                        );
                    }

                    // The library doesn't count a member's view of a public site, and does count
                    // a private site's.
                    let outcome = page_view_outcome(opened.page_view)?;
                    let expected = if post.site_is_private == Some(true) {
                        ReaderPageViewOutcome::Counted
                    } else {
                        ReaderPageViewOutcome::NotCountedForSiteMember
                    };
                    if outcome != expected {
                        return Err(format!("Expected {expected:?}, got {outcome:?}").into());
                    }

                    // Nor does it send a view as the library, which stats wouldn't count.
                    let library = wp_api::request::user_agent::default_user_agent("reqwest");
                    let opened = service
                        .post_opened(
                            post.key.clone(),
                            ReaderPageViewChoice::Count {
                                user_agent: library,
                            },
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                    let outcome = page_view_outcome(opened.page_view)?;
                    if outcome != ReaderPageViewOutcome::NotCountedForUserAgent {
                        return Err(format!("As the library, got {outcome:?}").into());
                    }

                    // The request itself, which the rule above may not have sent.
                    let view = ReaderPageView {
                        site_id,
                        post_id,
                        site_url: post.site_url.clone().ok_or("The post has no site URL")?,
                        site_is_private: post.site_is_private == Some(true),
                        user_agent: PAGE_VIEW_USER_AGENT.to_string(),
                    };
                    let request = view.request().ok_or("The view has no request")?;
                    let response = ctx
                        .delegate
                        .request_executor
                        .execute(Arc::new(request))
                        .await
                        .map_err(|e| e.to_string())?;
                    if response.status_code != 200 {
                        return Err(format!("The pixel answered {}", response.status_code).into());
                    }

                    // The pixel answers 200 whether or not the view counts, so the site's stats
                    // are what says. They take a few seconds to show a view.
                    for _ in 0..30 {
                        tokio::time::sleep(Duration::from_secs(3)).await;
                        if views_of(&ctx, site_id, post_id).await? > views_before {
                            return Ok(());
                        }
                    }
                    Err(format!(
                        "The view was sent, and after 90 seconds the site's stats still show \
                         {views_before} views of the post"
                    )
                    .into())
                })
            }
        })
        .with_ignored_flag(own_site().is_none()),
        Trial::test("reader_service::resolve_link_to_a_post", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    let link = service
                        .resolve_link(POST_URL.to_string())
                        .await
                        .map_err(|e| e.to_string())?;
                    let ReaderLink::Post { post } = link else {
                        return Err(
                            format!("The post's address should open the post: {link:?}").into()
                        );
                    };
                    if post.identity.site_id != Some(SITE) || post.identity.id != POST.0 as u64 {
                        return Err(format!(
                            "It isn't the post at the address: {:?}",
                            post.identity
                        )
                        .into());
                    }
                    if post.content.is_none() {
                        return Err("The post should come with its content".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_service::resolve_link_to_a_site", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    // A home address needs no account, so it's checked both ways.
                    let services = [
                        new_service(ctx.delegate.clone(), account())?,
                        new_service(signed_out(&ctx.delegate), ReaderOwner::SignedOut)?,
                    ];
                    for service in services {
                        let link = service
                            .resolve_link(SITE_URL.to_string())
                            .await
                            .map_err(|e| e.to_string())?;
                        let ReaderLink::Site { site_id, site } = link else {
                            return Err(format!(
                                "A site's home address should open the site: {link:?}"
                            )
                            .into());
                        };
                        if site_id != SITE || site.site_id != Some(SITE) || site.name.is_none() {
                            return Err(
                                format!("It isn't the site at the address: {site:?}").into()
                            );
                        }
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_service::resolve_link_to_a_feed", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    let link = service
                        .resolve_link(FEED_ONLY_SITE_URL.to_string())
                        .await
                        .map_err(|e| e.to_string())?;
                    let ReaderLink::Feed { feed_id, site } = link else {
                        return Err(format!(
                            "A feed-only site's home address should open its feed: {link:?}"
                        )
                        .into());
                    };
                    if site.feed_id != Some(feed_id) || site.site_id.is_some() {
                        return Err(format!(
                            "It should be a feed of no WordPress.com site: {site:?}"
                        )
                        .into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_service::resolve_link_that_is_not_in_the_reader", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    // A home address, looked up as a site and then for a feed, and an address with
                    // a path, which the server answers with a 500 before the same lookups.
                    let neither_with_a_path = format!("{NEITHER_URL}some/page/");
                    for address in [NEITHER_URL, neither_with_a_path.as_str()] {
                        let link = service
                            .resolve_link(address.to_string())
                            .await
                            .map_err(|e| format!("{address}: {e}"))?;
                        let expected = ReaderLink::NotInReader {
                            reason: ReaderLinkNotInReader::NotWordPressOrFeed,
                        };
                        if link != expected {
                            return Err(format!(
                                "{address} should be neither a site nor a feed: {link:?}"
                            )
                            .into());
                        }
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_service::resolve_link_to_a_post_signed_out", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    // Signed out, a post's address opens by its slug on the address's site.
                    let service = new_service(signed_out(&ctx.delegate), ReaderOwner::SignedOut)?;
                    let link = service
                        .resolve_link(POST_URL.to_string())
                        .await
                        .map_err(|e| e.to_string())?;
                    let ReaderLink::Post { post } = link else {
                        return Err(format!(
                            "Signed out, the post's address should open the post by its slug: \
                             {link:?}"
                        )
                        .into());
                    };
                    if post.identity.site_id != Some(SITE) || post.identity.id != POST.0 as u64 {
                        return Err(format!(
                            "It isn't the post at the address: {:?}",
                            post.identity
                        )
                        .into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_service::resolve_link_to_a_post_needs_an_account", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let executor = Arc::new(CountingExecutor {
                        inner: Arc::clone(&ctx.delegate.request_executor),
                        sent: AtomicU64::new(0),
                    });
                    let delegate = WpApiClientDelegate {
                        request_executor: executor.clone(),
                        ..signed_out(&ctx.delegate)
                    };
                    let service = new_service(delegate, ReaderOwner::SignedOut)?;
                    // Signed out, an address whose path ends in no slug (a day's archive) sends
                    // nothing, and a slug the site doesn't have is refused after one request. Only
                    // the resolver, which needs an account, could still find either.
                    let no_slug = format!("{SITE_URL}2014/12/29/");
                    let unknown_slug = format!("{SITE_URL}2014/12/29/no-post-has-this-slug-wp-rs-e2e/");
                    for (address, expected_sent) in [(no_slug, 0), (unknown_slug, 1)] {
                        let sent_before = executor.sent.load(Ordering::SeqCst);
                        match service.resolve_link(address.clone()).await {
                            Err(ReaderError::NeedsAccount) => {}
                            other => {
                                return Err(
                                    format!("{address}: expected NeedsAccount, got {other:?}")
                                        .into(),
                                );
                            }
                        }
                        let sent = executor.sent.load(Ordering::SeqCst) - sent_before;
                        if sent != expected_sent {
                            return Err(format!(
                                "{address}: {expected_sent} requests should be sent, but {sent} were"
                            )
                            .into());
                        }
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

/// How long a trial waits after a block or an unblock before it reads the list of blocked sites.
///
/// The server keeps its answer to the list, for each page and page size, until the next block or
/// unblock clears it. Once, in a parallel run, the list still had a site 14 minutes after it was
/// unblocked, and another unblock cleared it. Measured since, a read on its way while the server
/// takes an unblock leaves it stale like that, and a read that starts after the unblock is
/// answered doesn't (`ReaderService::refresh_blocked_sites` has the details); what overlapped the
/// unblock in that run wasn't found, so the wait stays.
pub(crate) async fn settle_after_a_block() {
    tokio::time::sleep(Duration::from_secs(3)).await;
}

/// A post's seen state in the Following stream, as the server answers a service with a cache of
/// its own, read back until it's `expected` (see [`read_back`]).
async fn seen_on_the_server(
    ctx: &TestContext,
    key: &ReaderSavedKey,
    expected: Option<bool>,
) -> Result<Option<bool>, Failed> {
    read_back(
        async || {
            let other = new_service(ctx.delegate.clone(), account())?;
            let stream = other.stream(ReaderStream::Following);
            stream.refresh().await.map_err(|e| e.to_string())?;
            seen_in(&stream, key).await
        },
        |seen| *seen == expected,
    )
    .await
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

/// A site the account owns, for the trials that write something public. Only the environment
/// names one, so no run writes a comment by accident.
fn own_site() -> Option<WpComSiteId> {
    std::env::var("WP_COM_E2E_OWN_SITE_ID")
        .ok()?
        .trim()
        .parse()
        .ok()
        .map(WpComSiteId)
}

/// How many views a site's stats have for a post.
async fn views_of(ctx: &TestContext, site_id: WpComSiteId, post_id: PostId) -> Result<u64, Failed> {
    Ok(ctx
        .client
        .stats_post()
        .get_stats_post(&site_id, &StatsPostTarget::Post { id: post_id })
        .await
        .map_err(|e| e.to_string())?
        .data
        .views)
}

/// The newest post of a site, as the cache has it after a refresh of the site's stream.
async fn first_post_of(
    service: &ReaderService,
    site_id: WpComSiteId,
) -> Result<ReaderPostRecord, Failed> {
    let stream = service.stream(ReaderStream::Site { site_id });
    stream.refresh().await.map_err(|e| e.to_string())?;
    let items = stream.load_items().await.map_err(|e| e.to_string())?;
    Ok(items
        .into_iter()
        .next()
        .ok_or("The site has no posts")?
        .post)
}

/// A client for the WordPress REST API of a site the account owns, which is what deletes a
/// comment.
fn own_site_client(ctx: &TestContext, site_id: WpComSiteId) -> WpApiClient {
    WpApiClient::new(
        Arc::new(WpComDotOrgApiUrlResolver::new(
            site_id.0.to_string(),
            WpComBaseUrl::Production,
        )),
        ctx.delegate.clone(),
    )
}

/// Whether the server has the account's like on a post, asked of the post's likes directly:
/// nothing of the service or its cache is in the answer.
async fn server_says_post_liked(
    ctx: &TestContext,
    post: &ReaderPostRecord,
) -> Result<bool, Failed> {
    let site_id = post.identity.site_id.ok_or("The post has no site")?;
    let likes = ctx
        .client
        .reader_likes()
        .post_likes(
            &site_id,
            &PostId(post.identity.id as i64),
            &ReaderLikesParams::default(),
        )
        .await
        .map_err(|e| e.to_string())?
        .data;
    likes
        .i_like
        .ok_or_else(|| "The post's likes don't say whether the account likes it".into())
}

/// Likes or unlikes a post through the service, and checks that the server has it and the cache
/// shows it.
async fn like_and_check(
    ctx: &TestContext,
    service: &ReaderService,
    post: &ReaderPostRecord,
    liked: bool,
) -> Result<(), Failed> {
    // A trial counts no view: one on someone's post can't be taken back.
    let outcome = service
        .set_post_liked(
            post.identity.clone(),
            liked,
            ReaderPageViewChoice::DontCount,
        )
        .await
        .map_err(|e| format!("Setting the like to {liked} was refused: {e}"))?;
    let outcome = match outcome.like {
        ReaderPostLikeResult::Done { outcome } => outcome,
        ReaderPostLikeResult::Failed { error } => {
            return Err(format!("Setting the like to {liked} was refused: {error}").into());
        }
    };
    if outcome != ReaderActionOutcome::Synced {
        return Err(format!("Setting the like to {liked} wasn't sent: {outcome:?}").into());
    }
    let cached = service
        .cached_post(post.key.clone())
        .map_err(|e| e.to_string())?
        .ok_or("The post left the cache")?;
    if cached.i_like != Some(liked) {
        return Err(format!("The cache says i_like is {:?}", cached.i_like).into());
    }
    let has = read_back(
        async || server_says_post_liked(ctx, post).await,
        |has| *has == liked,
    )
    .await?;
    if has != liked {
        return Err(format!("The server doesn't have the like set to {liked}").into());
    }
    Ok(())
}

/// Whether the server has the account's like on a comment.
async fn server_says_comment_liked(
    ctx: &TestContext,
    site_id: WpComSiteId,
    comment_id: CommentId,
) -> Result<bool, Failed> {
    let likes = ctx
        .client
        .reader_likes()
        .comment_likes(&site_id, &comment_id, &ReaderLikesParams::default())
        .await
        .map_err(|e| e.to_string())?
        .data;
    likes
        .i_like
        .ok_or_else(|| "The comment's likes don't say whether the account likes it".into())
}

/// Likes then unlikes a comment through the service, and checks the server after each.
async fn like_comment_and_check(
    ctx: &TestContext,
    service: &ReaderService,
    site_id: WpComSiteId,
    comment_id: CommentId,
) -> Result<(), Failed> {
    for liked in [true, false] {
        let outcome = service
            .set_comment_liked(site_id, comment_id, liked)
            .await
            .map_err(|e| format!("Setting the comment's like to {liked} was refused: {e}"))?;
        if outcome != ReaderActionOutcome::Synced {
            return Err(format!("The comment's like wasn't sent: {outcome:?}").into());
        }
        let has = read_back(
            async || server_says_comment_liked(ctx, site_id, comment_id).await,
            |has| *has == liked,
        )
        .await?;
        if has != liked {
            return Err(
                format!("The server doesn't have the comment's like set to {liked}").into(),
            );
        }
    }
    Ok(())
}

/// The account's subscription to a post's comments, as the server has it: whether it's
/// subscribed, and whether new comments notify it.
async fn comment_subscription(
    ctx: &TestContext,
    post: &ReaderPostRecord,
) -> Result<(bool, bool), Failed> {
    let site_id = post.identity.site_id.ok_or("The post has no site")?;
    let subscription = ctx
        .client
        .reader_comments()
        .subscription(&site_id, &PostId(post.identity.id as i64))
        .await
        .map_err(|e| e.to_string())?
        .data;
    Ok((
        subscription.i_subscribe,
        subscription.receives_notifications == Some(true),
    ))
}

/// Sends one change to a post's comment subscription, and checks what the server then has.
async fn expect_comment_subscription(
    ctx: &TestContext,
    post: &ReaderPostRecord,
    sent: Result<ReaderActionOutcome, ReaderError>,
    expected: (bool, bool),
) -> Result<(), Failed> {
    let outcome = sent.map_err(|e| format!("The change was refused: {e}"))?;
    if outcome != ReaderActionOutcome::Synced {
        return Err(format!("The change wasn't sent: {outcome:?}").into());
    }
    let has = read_back(
        async || comment_subscription(ctx, post).await,
        |has| *has == expected,
    )
    .await?;
    if has != expected {
        return Err(format!(
            "The server has (subscribed, notified) as {has:?}, expected {expected:?}"
        )
        .into());
    }
    Ok(())
}

/// Subscribes to a post's comments, turns their notifications on and off, and unsubscribes,
/// checking the server after each.
async fn subscribe_to_comments_and_check(
    ctx: &TestContext,
    service: &ReaderService,
    post: &ReaderPostRecord,
) -> Result<(), Failed> {
    let identity = post.identity.clone();
    if comment_subscription(ctx, post).await?.0 {
        let sent = service
            .set_comments_subscribed(identity.clone(), false)
            .await;
        expect_comment_subscription(ctx, post, sent, (false, false)).await?;
    }
    let sent = service
        .set_comments_subscribed(identity.clone(), true)
        .await;
    expect_synced("Subscribing", sent)?;
    // Whether a new subscription notifies is the server's to choose, so only the subscription is
    // checked here.
    let subscribed = read_back(async || comment_subscription(ctx, post).await, |has| has.0).await?;
    if !subscribed.0 {
        return Err("The server doesn't have the subscription".into());
    }
    let sent = service
        .set_comment_notifications(identity.clone(), true)
        .await;
    expect_comment_subscription(ctx, post, sent, (true, true)).await?;
    let sent = service
        .set_comment_notifications(identity.clone(), false)
        .await;
    expect_comment_subscription(ctx, post, sent, (true, false)).await?;
    let sent = service.set_comments_subscribed(identity, false).await;
    expect_comment_subscription(ctx, post, sent, (false, false)).await
}

/// Puts a post's comment subscription back to what it was.
async fn restore_comment_subscription(
    ctx: &TestContext,
    service: &ReaderService,
    post: &ReaderPostRecord,
    was: (bool, bool),
) -> Result<(), Failed> {
    let identity = post.identity.clone();
    let now = comment_subscription(ctx, post).await?;
    if now.0 != was.0 {
        service
            .set_comments_subscribed(identity.clone(), was.0)
            .await
            .map_err(|e| e.to_string())?;
    }
    if was.0 && comment_subscription(ctx, post).await?.1 != was.1 {
        service
            .set_comment_notifications(identity, was.1)
            .await
            .map_err(|e| e.to_string())?;
    }
    let now = read_back(
        async || comment_subscription(ctx, post).await,
        |now| *now == was,
    )
    .await?;
    if now != was {
        return Err(format!("The subscription is {now:?}, and was {was:?}").into());
    }
    Ok(())
}

/// How a followed site reaches the account, as the server has it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Delivery {
    email_posts: bool,
    email_comments: bool,
    notify_posts: bool,
    frequency: Option<ReaderSubscriptionEmailFrequency>,
}

/// A followed site's delivery settings, read by a service with a cache of its own, so they're
/// the server's.
async fn delivery_on_the_server(
    ctx: &TestContext,
    site_id: WpComSiteId,
) -> Result<Delivery, Failed> {
    let other = new_service(ctx.delegate.clone(), account())?;
    other
        .refresh_subscriptions()
        .await
        .map_err(|e| e.to_string())?;
    let subscription = other
        .subscriptions()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|one| one.site_id == Some(site_id))
        .ok_or("The account doesn't follow the site, so it has no delivery settings")?;
    Ok(Delivery {
        email_posts: subscription.email_posts == Some(true),
        email_comments: subscription.email_comments == Some(true),
        notify_posts: subscription.notify_posts == Some(true),
        frequency: subscription.email_frequency,
    })
}

fn expect_synced(what: &str, sent: Result<ReaderActionOutcome, ReaderError>) -> Result<(), Failed> {
    match sent {
        Ok(ReaderActionOutcome::Synced) => Ok(()),
        Ok(outcome) => Err(format!("{what} wasn't sent: {outcome:?}").into()),
        Err(error) => Err(format!("{what} was refused: {error}").into()),
    }
}

/// Flips each of a site's delivery settings and flips it back, checking the server after each
/// change.
async fn change_delivery_and_check(
    ctx: &TestContext,
    service: &ReaderService,
    site_id: WpComSiteId,
    was: &Delivery,
) -> Result<(), Failed> {
    for on in [!was.email_comments, was.email_comments] {
        expect_synced(
            "Comment emails",
            service.set_comment_emails(site_id, on).await,
        )?;
        let has = read_back(
            async || delivery_on_the_server(ctx, site_id).await,
            |has| has.email_comments == on,
        )
        .await?;
        if has.email_comments != on {
            return Err(format!("The server doesn't have comment emails set to {on}").into());
        }
    }
    for on in [!was.notify_posts, was.notify_posts] {
        expect_synced(
            "Notifications",
            service.set_post_notifications(site_id, on).await,
        )?;
        let has = read_back(
            async || delivery_on_the_server(ctx, site_id).await,
            |has| has.notify_posts == on,
        )
        .await?;
        if has.notify_posts != on {
            return Err(format!("The server doesn't have notifications set to {on}").into());
        }
    }
    // The frequency is only kept while posts are emailed.
    if was.email_posts {
        let other = if was.frequency == Some(ReaderSubscriptionEmailFrequency::Daily) {
            ReaderSubscriptionEmailFrequency::Weekly
        } else {
            ReaderSubscriptionEmailFrequency::Daily
        };
        let original = was
            .frequency
            .clone()
            .unwrap_or(ReaderSubscriptionEmailFrequency::Instantly);
        for frequency in [other, original] {
            service
                .set_post_email_frequency(site_id, frequency.clone())
                .await
                .map_err(|e| format!("The frequency was refused: {e}"))?;
            let has = read_back(
                async || delivery_on_the_server(ctx, site_id).await,
                |has| has.frequency.as_ref() == Some(&frequency),
            )
            .await?
            .frequency;
            if has != Some(frequency.clone()) {
                return Err(
                    format!("The server has the frequency as {has:?}, not {frequency:?}").into(),
                );
            }
        }
    }
    for on in [!was.email_posts, was.email_posts] {
        expect_synced("Post emails", service.set_post_emails(site_id, on).await)?;
        let has = read_back(
            async || delivery_on_the_server(ctx, site_id).await,
            |has| has.email_posts == on,
        )
        .await?;
        if has.email_posts != on {
            return Err(format!("The server doesn't have post emails set to {on}").into());
        }
    }
    Ok(())
}

/// Puts a site's delivery settings back to what they were.
async fn restore_delivery(
    ctx: &TestContext,
    service: &ReaderService,
    site_id: WpComSiteId,
    was: &Delivery,
) -> Result<(), Failed> {
    let now = delivery_on_the_server(ctx, site_id).await?;
    if now.email_posts != was.email_posts {
        expect_synced(
            "Post emails",
            service.set_post_emails(site_id, was.email_posts).await,
        )?;
    }
    if now.email_comments != was.email_comments {
        expect_synced(
            "Comment emails",
            service
                .set_comment_emails(site_id, was.email_comments)
                .await,
        )?;
    }
    if now.notify_posts != was.notify_posts {
        expect_synced(
            "Notifications",
            service
                .set_post_notifications(site_id, was.notify_posts)
                .await,
        )?;
    }
    if was.email_posts
        && let Some(frequency) = &was.frequency
        && delivery_on_the_server(ctx, site_id).await?.frequency != was.frequency
    {
        service
            .set_post_email_frequency(site_id, frequency.clone())
            .await
            .map_err(|e| e.to_string())?;
    }
    let now = read_back(
        async || delivery_on_the_server(ctx, site_id).await,
        |now| now == was,
    )
    .await?;
    if &now != was {
        return Err(format!("The settings are {now:?}, and were {was:?}").into());
    }
    Ok(())
}

/// Whether the account follows a site, as the server answers for the site's ID.
async fn follows_by_id(ctx: &TestContext, site_id: WpComSiteId) -> Result<bool, Failed> {
    Ok(ctx
        .client
        .reader_subscriptions()
        .is_following_site(&site_id)
        .await
        .map_err(|e| e.to_string())?
        .data
        .is_following)
}

/// Follows or unfollows a site by its ID, and checks the answer and what the server then says.
async fn set_followed_by_id(
    ctx: &TestContext,
    site_id: WpComSiteId,
    followed: bool,
) -> Result<(), Failed> {
    let subscriptions = ctx.client.reader_subscriptions();
    let refused = |e: WpApiError| format!("Setting the follow to {followed} was refused: {e}");
    let answer = if followed {
        subscriptions
            .follow_site(&site_id)
            .await
            .map_err(refused)?
            .data
    } else {
        subscriptions
            .unfollow_site(&site_id)
            .await
            .map_err(refused)?
            .data
    };
    if answer.is_following != followed || answer.success != Some(true) {
        return Err(format!("The answer to setting the follow to {followed} is {answer:?}").into());
    }
    let has = read_back(
        async || follows_by_id(ctx, site_id).await,
        |has| *has == followed,
    )
    .await?;
    if has != followed {
        return Err(format!("The server doesn't have the follow set to {followed}").into());
    }
    Ok(())
}

/// Writes a comment and a reply to it through the service, and checks what the cache then
/// shows. Every comment the server took is added to `written`, for the caller to delete.
async fn write_comments(
    service: &ReaderService,
    site_id: WpComSiteId,
    written: &mut Vec<CommentId>,
) -> Result<(), Failed> {
    let post = first_post_of(service, site_id).await?;
    let comments = service.comments(site_id, PostId(post.identity.id as i64));
    comments.refresh().await.map_err(|e| e.to_string())?;
    let before = comments
        .load_items()
        .await
        .map_err(|e| e.to_string())?
        .len();

    let text = "A test comment from wordpress-rs. It deletes itself.";
    let comment = comments
        .post_comment(text.to_string(), None)
        .await
        .map_err(|e| format!("The comment wasn't taken: {e}"))?;
    let comment_id = comment.comment_id.ok_or("The comment has no ID")?;
    written.push(comment_id);
    if comment.send_state != ReaderCommentSendState::Sent || comment.local_id.is_some() {
        return Err(format!("The comment isn't the server's: {comment:?}").into());
    }
    if !comment
        .content
        .as_deref()
        .unwrap_or_default()
        .contains("deletes itself")
    {
        return Err(format!("The comment's content is {:?}", comment.content).into());
    }

    let reply = comments
        .post_comment(
            "A test reply. It deletes itself too.".to_string(),
            Some(comment_id),
        )
        .await
        .map_err(|e| format!("The reply wasn't taken: {e}"))?;
    let reply_id = reply.comment_id.ok_or("The reply has no ID")?;
    written.push(reply_id);
    if reply.parent_id != Some(comment_id) {
        return Err(format!("The reply's parent is {:?}", reply.parent_id).into());
    }

    // The cache shows each once, and a refresh from the server keeps both.
    for refreshed in [false, true] {
        if refreshed {
            comments.refresh().await.map_err(|e| e.to_string())?;
        }
        let items = comments.load_items().await.map_err(|e| e.to_string())?;
        if items.len() != before + 2 {
            return Err(format!(
                "Expected {} comments, the cache has {} (refreshed: {refreshed})",
                before + 2,
                items.len()
            )
            .into());
        }
        for id in [comment_id, reply_id] {
            let found = items
                .iter()
                .filter(|item| item.comment_id == Some(id))
                .count();
            if found != 1 {
                return Err(format!("Comment {} is in the cache {found} times", id.0).into());
            }
        }
    }
    Ok(())
}

/// Room for a few posts' images, so that running out of room is part of the trial.
const OFFLINE_IMAGE_BYTES: u64 = 3_000_000;

/// Downloads the account's saved posts for reading with no connection, into a new cache, and
/// checks what the device then has. It only reads from the server.
async fn download_saved_posts(
    service: &ReaderService,
    directory: &std::path::Path,
) -> Result<(), Failed> {
    let saved = service.saved();
    saved
        .set_offline_image_store(directory.display().to_string(), OFFLINE_IMAGE_BYTES)
        .map_err(|e| e.to_string())?;

    let report = saved
        .download_for_offline(true)
        .await
        .map_err(|e| e.to_string())?;
    eprintln!("Saved posts for offline: {report:?}");
    if let Some(reason) = &report.stopped_because {
        return Err(format!("The download stopped: {reason}").into());
    }
    let count = saved.saved_count().map_err(|e| e.to_string())?;
    if count.is_none_or(|count| count == 0) {
        // An account with nothing saved has nothing to check.
        return Ok(());
    }
    if report.ready == 0 {
        return Err(format!("No saved post has its content: {report:?}").into());
    }
    if report.not_downloaded > 0 {
        return Err(format!("Saves were left without being tried: {report:?}").into());
    }
    if report.image_bytes > OFFLINE_IMAGE_BYTES {
        return Err(format!("The images take more room than allowed: {report:?}").into());
    }

    // The files in the directory are the stored images, and nothing else.
    let mut on_disk = 0;
    for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
        on_disk += entry
            .and_then(|entry| entry.metadata())
            .map_err(|e| e.to_string())?
            .len();
    }
    if on_disk != report.image_bytes {
        return Err(format!(
            "The directory holds {on_disk} bytes, and the report says {}",
            report.image_bytes
        )
        .into());
    }

    // A post with its images reads from the stored copies.
    let items = saved
        .clone()
        .saved_posts()
        .load_items()
        .await
        .map_err(|e| e.to_string())?;
    let with_images = items
        .iter()
        .filter(|item| item.offline == ReaderOfflineState::Ready { has_images: true })
        .count() as u64;
    if with_images > report.with_images {
        return Err("The list has more posts with images than the report".into());
    }
    for item in items
        .iter()
        .filter(|item| item.offline == ReaderOfflineState::Ready { has_images: true })
    {
        let post = saved
            .offline_post(item.key.clone(), "reader-file://".to_string())
            .map_err(|e| e.to_string())?
            .ok_or("A saved post has no offline copy")?;
        let content = post.post.content.unwrap_or_default();
        for image in &post.images {
            if !directory.join(&image.file_name).is_file() {
                return Err(format!("{} isn't in the directory", image.file_name).into());
            }
            let pointed_at = content.contains(&format!("reader-file://{}", image.file_name))
                || post.post.featured_image.as_deref()
                    == Some(format!("reader-file://{}", image.file_name).as_str())
                || post.post.display_image_url.as_deref()
                    == Some(format!("reader-file://{}", image.file_name).as_str());
            if !pointed_at {
                return Err(format!("Nothing points at the stored copy of {}", image.url).into());
            }
        }
    }

    // A second download has nothing left to fetch but the list's first page.
    let again = saved
        .download_for_offline(false)
        .await
        .map_err(|e| e.to_string())?;
    if again.ready < report.ready {
        return Err(format!("A second download lost content: {again:?}").into());
    }
    Ok(())
}

/// A request executor that counts the requests it sends.
struct CountingExecutor {
    inner: Arc<dyn RequestExecutor>,
    sent: AtomicU64,
}

#[async_trait::async_trait]
impl RequestExecutor for CountingExecutor {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        self.sent.fetch_add(1, Ordering::SeqCst);
        self.inner.execute(request).await
    }

    async fn upload(
        &self,
        request: Arc<wp_api::request::WpMultipartFormRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        self.sent.fetch_add(1, Ordering::SeqCst);
        self.inner.upload(request).await
    }

    async fn sleep(&self, millis: u64) {
        self.inner.sleep(millis).await
    }

    fn cancel(&self, context: Arc<wp_api::request::RequestContext>) {
        self.inner.cancel(context)
    }
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

/// Refreshes a stream in the service's language, and returns the IDs of the posts it then has.
async fn refreshed_post_ids(
    service: &ReaderService,
    stream: &ReaderStream,
) -> Result<HashSet<String>, Failed> {
    service
        .stream(stream.clone())
        .refresh()
        .await
        .map_err(|e| e.to_string())?;
    cached_post_ids(service, stream).await
}

/// The IDs of the posts a stream has in the cache, in the service's language.
async fn cached_post_ids(
    service: &ReaderService,
    stream: &ReaderStream,
) -> Result<HashSet<String>, Failed> {
    let items = service
        .stream(stream.clone())
        .load_items()
        .await
        .map_err(|e| e.to_string())?;
    Ok(items
        .iter()
        .map(|item| format!("{:?}", item.post.key))
        .collect())
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

/// What became of an opened post's view, or the error it failed with.
fn page_view_outcome(page_view: ReaderOpenedPageView) -> Result<ReaderPageViewOutcome, Failed> {
    match page_view {
        ReaderOpenedPageView::Reported { outcome } => Ok(outcome),
        ReaderOpenedPageView::NotAsked => Err("The view wasn't asked for".into()),
        ReaderOpenedPageView::Failed { error } => Err(error.to_string().into()),
    }
}
