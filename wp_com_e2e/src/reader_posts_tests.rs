use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_posts::{ReaderStreamParams, ReaderTagPostsParams, ReaderTagSlug};
use wp_api::wp_com::reader_saved::ReaderFeedId;

use crate::context::TestContext;

/// The WordPress.com Blog, which is public.
const SITE: WpComSiteId = WpComSiteId(3584907);
/// The xkcd RSS feed, which is public.
const FEED: ReaderFeedId = ReaderFeedId(8857199);

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    vec![
        Trial::test("reader_posts::following", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    // A full page, so every kind of post the account follows has to parse.
                    let params = ReaderStreamParams {
                        number: Some(20),
                        meta: Some("site".to_string()),
                        ..Default::default()
                    };
                    let page = ctx
                        .client
                        .reader_posts()
                        .following(&params)
                        .await
                        .map_err(|e| e.to_string())?
                        .data;

                    // Paging by date: the next page starts before the oldest post of this one.
                    if let Some(before) = page.posts.last().and_then(|post| post.date) {
                        let params = ReaderStreamParams {
                            number: Some(20),
                            before: Some(before),
                            ..Default::default()
                        };
                        ctx.client
                            .reader_posts()
                            .following(&params)
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_posts::site_posts", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let params = ReaderStreamParams {
                        number: Some(5),
                        meta: Some("site,likes".to_string()),
                        ..Default::default()
                    };
                    let page = ctx
                        .client
                        .reader_posts()
                        .site_posts(&SITE, &params)
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    let post = page.posts.first().ok_or("The site has no posts")?;
                    if post.identity.site_id != Some(SITE) {
                        return Err("The post isn't from the site".into());
                    }
                    if post.title.is_none() || post.site_locale.is_none() {
                        return Err("A site post is missing its title or the site's locale".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_posts::feed_posts", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let page = ctx
                        .client
                        .reader_posts()
                        .feed_posts(&FEED, &ReaderStreamParams::default())
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    let post = page.posts.first().ok_or("The feed has no posts")?;
                    if !post.identity.is_external || post.identity.feed_item_id.is_none() {
                        return Err("A feed post should be external, with a feed item ID".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_posts::tag_posts", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    // A tag busy enough to always have a second page.
                    let tag = ReaderTagSlug("photography".to_string());
                    let first = ctx
                        .client
                        .reader_posts()
                        .tag_posts(&tag, &ReaderTagPostsParams::default())
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if first.posts.is_empty() {
                        return Err("The tag has no posts".into());
                    }
                    let handle = first
                        .next_page_handle
                        .ok_or("The first page has no handle for the next")?;

                    // Paging by handle: the next page is other posts.
                    let params = ReaderTagPostsParams {
                        page_handle: Some(handle),
                        ..Default::default()
                    };
                    let second = ctx
                        .client
                        .reader_posts()
                        .tag_posts(&tag, &params)
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    let seen = |page: &[wp_api::wp_com::reader_posts::ReaderPost]| {
                        page.iter()
                            .map(|post| (post.identity.site_id, post.identity.id))
                            .collect::<std::collections::HashSet<_>>()
                    };
                    if second.posts.is_empty()
                        || !seen(&first.posts).is_disjoint(&seen(&second.posts))
                    {
                        return Err("The second page should be other posts".into());
                    }
                    Ok(())
                })
            }
        }),
    ]
}
