//! `ReaderService` against the live API: streams fetched, kept in a cache, and paged.

use libtest_mimic::{Failed, Trial};
use std::collections::HashSet;
use std::sync::Arc;
use wp_api::comments::{CommentDeleteParams, CommentId};
use wp_api::posts::PostId;
use wp_api::prelude::*;
use wp_api::wp_com::endpoint::WpComDotOrgApiUrlResolver;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::reader_page_views::ReaderPageView;
use wp_api::wp_com::reader_posts::{
    ReaderListOwner, ReaderListSlug, ReaderSearchSort, ReaderTagSlug,
};
use wp_api::wp_com::reader_saved::{ReaderSavedKey, ReaderSavedSource};
use wp_api::wp_com::{WpComBaseUrl, WpComSiteId};
use wp_mobile::service::reader::{
    ReaderActionOutcome, ReaderError, ReaderFollowTarget, ReaderPageViewOutcome, ReaderService,
    ReaderStream, ReaderStreamCollection, ReaderStreamItem,
};
use wp_mobile::service::reader_saved::ReaderOfflineState;
use wp_mobile_cache::WpApiCache;
use wp_mobile_cache::db_types::reader::ReaderPostRecord;
use wp_mobile_cache::repository::reader_comments::ReaderCommentSendState;
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
        Trial::test("reader_service::menu_sites_subscriptions_and_comments", {
            let ctx = Arc::clone(&ctx);
            move || {
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
                    let other = new_service(ctx.delegate.clone(), account())?;
                    let fetched = match other.refresh_menu().await {
                        Ok(()) => follows(&other),
                        Err(error) => Err(error.to_string().into()),
                    };
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
                    service.refresh_menu().await.map_err(|e| e.to_string())?;
                    if follows(&service)? != was {
                        return Err("The tag should be back as it was".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_service::a_follow_is_sent_and_put_back", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    let site_id = ctx.site_id;
                    let site = service
                        .fetch_site(site_id)
                        .await
                        .map_err(|e| e.to_string())?;
                    let was = site.is_following == Some(true);
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
                    let other = new_service(ctx.delegate.clone(), account())?;
                    let fetched = match other.refresh_subscriptions().await {
                        Ok(_) => follows(&other),
                        Err(error) => Err(error.to_string().into()),
                    };
                    // Whatever happened, the site ends as it was.
                    let restored = service
                        .set_followed(target, was)
                        .await
                        .map_err(|e| e.to_string())?;

                    if changed.map_err(|e| e.to_string())? != ReaderActionOutcome::Synced
                        || restored != ReaderActionOutcome::Synced
                    {
                        return Err("The follow should reach the server".into());
                    }
                    if shown? == was || fetched? == was {
                        return Err("The cache and the server should both show the change".into());
                    }
                    service
                        .refresh_subscriptions()
                        .await
                        .map_err(|e| e.to_string())?;
                    if follows(&service)? != was {
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
                let Some(site_id) = own_site() else {
                    return Err("Set WP_COM_E2E_OWN_SITE_ID to a site the account owns".into());
                };
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    let mut written = Vec::new();
                    let result = write_comments(&service, site_id, &mut written).await;
                    // Whatever happened, the comments it wrote are deleted, replies first.
                    let site = WpApiClient::new(
                        Arc::new(WpComDotOrgApiUrlResolver::new(
                            site_id.0.to_string(),
                            WpComBaseUrl::Production,
                        )),
                        ctx.delegate.clone(),
                    );
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
                let Some(site_id) = own_site() else {
                    return Err("Set WP_COM_E2E_OWN_SITE_ID to a site the account owns".into());
                };
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    let post = first_post_of(&service, site_id).await?;
                    let was = post.i_like == Some(true);

                    let result = like_and_check(&service, &post, !was).await;
                    // Whatever happened, the post is left as it was found.
                    let restored = like_and_check(&service, &post, was).await;
                    result?;
                    restored
                })
            }
        })
        .with_ignored_flag(own_site().is_none()),
        // Counts one view, on a post of a site the account owns.
        Trial::test("reader_service::a_page_view_of_an_own_post", {
            let ctx = Arc::clone(&ctx);
            move || {
                let Some(site_id) = own_site() else {
                    return Err("Set WP_COM_E2E_OWN_SITE_ID to a site the account owns".into());
                };
                ctx.runtime.block_on(async {
                    let service = new_service(ctx.delegate.clone(), account())?;
                    let post = first_post_of(&service, site_id).await?;

                    // The library doesn't count a member's view of a public site, and does count
                    // a private site's.
                    let outcome = service
                        .post_opened(post.key.clone())
                        .await
                        .map_err(|e| e.to_string())?;
                    let expected = if post.site_is_private == Some(true) {
                        ReaderPageViewOutcome::Counted
                    } else {
                        ReaderPageViewOutcome::NotCountedForSiteMember
                    };
                    if outcome != expected {
                        return Err(format!("Expected {expected:?}, got {outcome:?}").into());
                    }

                    // The request itself, which the rule above may not have sent.
                    let view = ReaderPageView {
                        site_id,
                        post_id: PostId(post.identity.id as i64),
                        site_url: post.site_url.clone().ok_or("The post has no site URL")?,
                        site_is_private: post.site_is_private == Some(true),
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
                    Ok(())
                })
            }
        })
        .with_ignored_flag(own_site().is_none()),
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

/// Likes or unlikes a post through the service, and checks the server took it and the cache
/// shows it.
async fn like_and_check(
    service: &ReaderService,
    post: &ReaderPostRecord,
    liked: bool,
) -> Result<(), Failed> {
    let outcome = service
        .set_post_liked(post.identity.clone(), liked)
        .await
        .map_err(|e| format!("Setting the like to {liked} was refused: {e}"))?;
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
