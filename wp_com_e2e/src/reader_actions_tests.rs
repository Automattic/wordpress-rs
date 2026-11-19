//! The Reader's write endpoints against the live API. Each trial puts back what it changed.

use libtest_mimic::{Failed, Trial};
use std::sync::Arc;
use wp_api::posts::PostId;
use wp_api::wp_com::reader_blocks::ReaderBlockedSitesParams;
use wp_api::wp_com::reader_posts::{ReaderPost, ReaderStreamParams};
use wp_api::wp_com::reader_saved::ReaderFeedItemId;
use wp_api::wp_com::reader_seen::{
    ReaderSeenFeedItemsParams, ReaderSeenSitePostsParams, ReaderSeenSource,
};

use crate::context::TestContext;

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    vec![
        Trial::test("reader_blocks::block_and_unblock_a_site", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let site = ctx.site_id;
                    if is_blocked(&ctx).await? {
                        return Err("The test site is blocked already, which isn't the trial's \
                                    doing, so it's left alone"
                            .into());
                    }

                    let blocked = ctx
                        .client
                        .reader_blocks()
                        .block_site(&site)
                        .await
                        .map_err(|e| e.to_string());
                    let listed = is_blocked(&ctx).await;
                    // Whatever happened, the site ends unblocked.
                    let unblocked = ctx
                        .client
                        .reader_blocks()
                        .unblock_site(&site)
                        .await
                        .map_err(|e| e.to_string())?
                        .data;

                    if !blocked?.data.success || !listed? {
                        return Err("Blocking the site should list it as blocked".into());
                    }
                    if !unblocked.success || is_blocked(&ctx).await? {
                        return Err("Unblocking the site should take it off the list".into());
                    }
                    // Unblocking a site that isn't blocked changes nothing, and says so.
                    let again = ctx
                        .client
                        .reader_blocks()
                        .unblock_site(&site)
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if again.success {
                        return Err("Unblocking twice should report no change".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_seen::mark_a_post_and_put_it_back", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    // The newest followed post that has a seen state. An account that follows
                    // nothing has none, and then there's nothing to try.
                    let Some(post) = seen_posts(&ctx).await?.into_iter().next() else {
                        return Ok(());
                    };
                    let was = post.is_seen == Some(true);

                    let flipped = mark(&ctx, &post, !was).await;
                    let after = seen_state(&ctx, &post).await;
                    // Whatever happened, the post ends as it was.
                    mark(&ctx, &post, was).await?;

                    flipped?;
                    if after? != Some(!was) {
                        return Err("The stream should show the post's new seen state".into());
                    }
                    if seen_state(&ctx, &post).await? != Some(was) {
                        return Err("The post should be back as it was".into());
                    }
                    Ok(())
                })
            }
        }),
    ]
}

async fn is_blocked(ctx: &TestContext) -> Result<bool, Failed> {
    let params = ReaderBlockedSitesParams {
        page: None,
        per_page: Some(100),
    };
    let page = ctx
        .client
        .reader_blocks()
        .list_sites(&params)
        .await
        .map_err(|e| e.to_string())?
        .data;
    Ok(page.sites.iter().any(|site| site.id == ctx.site_id))
}

/// The newest followed posts that have a seen state.
async fn seen_posts(ctx: &TestContext) -> Result<Vec<ReaderPost>, Failed> {
    let params = ReaderStreamParams {
        number: Some(20),
        ..Default::default()
    };
    let page = ctx
        .client
        .reader_posts()
        .following(&params)
        .await
        .map_err(|e| e.to_string())?
        .data;
    Ok(page
        .posts
        .into_iter()
        .filter(|post| post.is_seen.is_some())
        .collect())
}

async fn seen_state(ctx: &TestContext, post: &ReaderPost) -> Result<Option<bool>, Failed> {
    Ok(seen_posts(ctx)
        .await?
        .into_iter()
        .find(|other| other.identity == post.identity)
        .and_then(|post| post.is_seen))
}

/// Marks a post as seen or unseen, by its site and post IDs, or by its feed's for an RSS post.
async fn mark(ctx: &TestContext, post: &ReaderPost, seen: bool) -> Result<(), Failed> {
    let source = ReaderSeenSource::Ios;
    let seen_api = ctx.client.reader_seen();
    let identity = &post.identity;
    // Each request has its own response type around the same answer.
    let marked = if identity.is_external {
        let params = ReaderSeenFeedItemsParams {
            feed_id: identity.feed_id.ok_or("An RSS post has no feed ID")?,
            feed_item_ids: vec![
                identity
                    .feed_item_id
                    .unwrap_or(ReaderFeedItemId(identity.id)),
            ],
            source,
        };
        if seen {
            seen_api
                .feed_items_seen(&params)
                .await
                .map(|response| response.data.status)
        } else {
            seen_api
                .feed_items_unseen(&params)
                .await
                .map(|response| response.data.status)
        }
    } else {
        let params = ReaderSeenSitePostsParams {
            blog_id: identity.site_id.ok_or("A site post has no site ID")?,
            post_ids: vec![PostId(identity.id as i64)],
            source,
        };
        if seen {
            seen_api
                .site_posts_seen(&params)
                .await
                .map(|response| response.data.status)
        } else {
            seen_api
                .site_posts_unseen(&params)
                .await
                .map(|response| response.data.status)
        }
    };
    if !marked.map_err(|e| e.to_string())? {
        return Err("The API didn't mark the post".into());
    }
    Ok(())
}
