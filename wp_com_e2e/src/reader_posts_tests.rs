use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::date::WpGmtDateTime;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::client::WpComApiClient;
use wp_api::wp_com::reader_posts::{
    ReaderFreshlyPressedParams, ReaderLikedPostsParams, ReaderListOwner, ReaderListPostsParams,
    ReaderListPostsResponse, ReaderListSlug, ReaderOrganizationPostsParams, ReaderPost,
    ReaderPostsResponse, ReaderSearchPostsParams, ReaderSearchPostsResponse, ReaderSearchSort,
    ReaderStreamParams, ReaderTagPostsParams, ReaderTagSlug,
};
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
        Trial::test("reader_posts::fields", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let page = |fields: Option<&str>| {
                        let params = ReaderStreamParams {
                            number: Some(10),
                            meta: Some("site,likes".to_string()),
                            fields: fields.map(str::to_string),
                            ..Default::default()
                        };
                        let ctx = Arc::clone(&ctx);
                        async move {
                            ctx.client
                                .reader_posts()
                                .site_posts(&SITE, &params)
                                .await
                                .map(|response| response.data)
                                .map_err(|e| e.to_string())
                        }
                    };

                    // Without `fields`, a post has its title and content.
                    let whole = page(None).await?;
                    let post = whole.posts.first().ok_or("The site has no posts")?;
                    if post.title.is_none() || post.content.is_none() {
                        return Err("A whole post is missing its title or content".into());
                    }

                    // A few fields give only those.
                    let few = page(Some("ID,site_ID,date")).await?;
                    let post = few.posts.first().ok_or("The site has no posts")?;
                    if post.identity.site_id != Some(SITE) || post.date.is_none() {
                        return Err("A post is missing a field that was asked for".into());
                    }
                    if post.title.is_some() || post.content.is_some() {
                        return Err("A post has a field that wasn't asked for".into());
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
                    let seen = |page: &[ReaderPost]| {
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
        Trial::test("reader_posts::liked", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let page = |before| {
                        let params = ReaderLikedPostsParams {
                            number: Some(5),
                            before,
                            ..Default::default()
                        };
                        let ctx = Arc::clone(&ctx);
                        async move {
                            ctx.client
                                .reader_posts()
                                .liked(&params)
                                .await
                                .map(|response| response.data)
                                .map_err(|e| e.to_string())
                        }
                    };

                    // An account that has liked nothing gets a page with no posts.
                    let first = page(None).await?;
                    let Some(last) = first.posts.last() else {
                        return Ok(());
                    };
                    if first.posts.iter().any(|post| post.date_liked.is_none()) {
                        return Err("A liked post should say when it was liked".into());
                    }
                    let cursor = first.date_range.as_ref().and_then(|range| range.after);
                    if cursor.is_none() || cursor != last.date_liked {
                        return Err("The range should end at the last post's like".into());
                    }

                    // Paging by the date of the like: the next page is posts liked earlier.
                    let second = page(cursor).await?;
                    if second
                        .posts
                        .iter()
                        .any(|post| !is_before(post.date_liked, cursor))
                    {
                        return Err("The second page should be posts liked earlier".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_posts::automattic", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let page = |before| {
                        let params = ReaderOrganizationPostsParams {
                            number: Some(5),
                            before,
                            ..Default::default()
                        };
                        let ctx = Arc::clone(&ctx);
                        async move {
                            ctx.client
                                .reader_posts()
                                .automattic(&params)
                                .await
                                .map(|response| response.data)
                                .map_err(|e| e.to_string())
                        }
                    };

                    // Only an Automattician gets posts. Anyone else gets a page with none.
                    let first = page(None).await?;
                    let Some(last) = first.posts.last() else {
                        return Ok(());
                    };
                    let cursor = first.date_range.as_ref().and_then(|range| range.after);
                    if cursor.is_none() || cursor != last.date {
                        return Err("The range should end at the last post's date".into());
                    }

                    // Paging by date: the next page is older posts.
                    let second = page(cursor).await?;
                    if second
                        .posts
                        .iter()
                        .any(|post| !is_before(post.date, cursor))
                    {
                        return Err("The second page should be older posts".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_posts::following_p2", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    // An account that follows no P2 gets a page with no posts.
                    ctx.client
                        .reader_posts()
                        .following_p_2(&ReaderOrganizationPostsParams::default())
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(())
                })
            }
        }),
        Trial::test("reader_posts::freshly_pressed", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let first = freshly_pressed(&ctx.client, None).await?;
                    check_freshly_pressed(&first)?;

                    // Paging by the time a post is shown: the next page was shown earlier.
                    let cursor = first.date_range.as_ref().and_then(|range| range.after);
                    let second = freshly_pressed(&ctx.client, cursor).await?;
                    check_freshly_pressed(&second)?;
                    if second
                        .posts
                        .iter()
                        .any(|post| !is_before(post.editorial_displayed_on, cursor))
                    {
                        return Err("The second page should be posts shown earlier".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_posts::search", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    // By relevance: every post has a score, and the handle is the next offset.
                    let first = search(&ctx.client, None, None).await?;
                    if first.posts.is_empty() {
                        return Err("The search has no results".into());
                    }
                    if first.posts.iter().any(|post| post.score.is_none()) {
                        return Err("A result by relevance should have a score".into());
                    }
                    let handle = first
                        .next_page_handle
                        .clone()
                        .ok_or("The first page has no handle for the next")?;
                    let second = search(&ctx.client, None, Some(handle)).await?;
                    if second.posts.is_empty() {
                        return Err("The second page by relevance has no results".into());
                    }

                    // By date: every post says when the search picked it up, and the next page
                    // is other posts.
                    let sort = Some(ReaderSearchSort::Date);
                    let first = search(&ctx.client, sort, None).await?;
                    if first.posts.is_empty() || first.posts.iter().any(|p| p.date_added.is_none())
                    {
                        return Err("A result by date should say when it was picked up".into());
                    }
                    let handle = first
                        .next_page_handle
                        .clone()
                        .ok_or("The first page by date has no handle for the next")?;
                    let second = search(&ctx.client, sort, Some(handle)).await?;
                    let seen = |page: &ReaderSearchPostsResponse| {
                        page.posts
                            .iter()
                            .map(|result| (result.post.identity.site_id, result.post.identity.id))
                            .collect::<std::collections::HashSet<_>>()
                    };
                    if second.posts.is_empty() || !seen(&first).is_disjoint(&seen(&second)) {
                        return Err("The second page by date should be other posts".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_posts::list_posts", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let first = list_posts(&ctx.client, None).await?;
                    if first.posts.is_empty() {
                        return Err("The list has no posts".into());
                    }
                    let handle = first
                        .next_page_handle
                        .clone()
                        .ok_or("The first page has no handle for the next")?;

                    // Paging by handle: the next page is other posts.
                    let second = list_posts(&ctx.client, Some(handle)).await?;
                    let seen = |page: &ReaderListPostsResponse| {
                        page.posts
                            .iter()
                            .map(|post| (post.identity.site_id, post.identity.id))
                            .collect::<std::collections::HashSet<_>>()
                    };
                    if second.posts.is_empty() || !seen(&first).is_disjoint(&seen(&second)) {
                        return Err("The second page should be other posts".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_posts::signed_out", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let reader_posts = ctx.signed_out_client.reader_posts();

                    // Freshly Pressed answers without an account.
                    let page = freshly_pressed(&ctx.signed_out_client, None).await?;
                    check_freshly_pressed(&page)?;

                    // The Automattic stream answers too, with no posts.
                    let page = reader_posts
                        .automattic(&ReaderOrganizationPostsParams::default())
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if !page.posts.is_empty() {
                        return Err("The Automattic stream should be empty signed out".into());
                    }

                    // Liked and the P2 stream are the user's own, so they're refused.
                    if reader_posts
                        .liked(&ReaderLikedPostsParams::default())
                        .await
                        .is_ok()
                    {
                        return Err("The Liked stream should need an account".into());
                    }
                    if reader_posts
                        .following_p_2(&ReaderOrganizationPostsParams::default())
                        .await
                        .is_ok()
                    {
                        return Err("The P2 stream should need an account".into());
                    }

                    // A search and a public list's posts answer without an account.
                    if search(&ctx.signed_out_client, None, None)
                        .await?
                        .posts
                        .is_empty()
                    {
                        return Err("A search should have results signed out".into());
                    }
                    if list_posts(&ctx.signed_out_client, None)
                        .await?
                        .posts
                        .is_empty()
                    {
                        return Err("A public list should have posts signed out".into());
                    }
                    Ok(())
                })
            }
        }),
    ]
}

/// A page of a search for a word that always has results.
async fn search(
    client: &WpComApiClient,
    sort: Option<ReaderSearchSort>,
    page_handle: Option<String>,
) -> Result<ReaderSearchPostsResponse, String> {
    let params = ReaderSearchPostsParams {
        q: "photography".to_string(),
        number: Some(5),
        sort,
        page_handle,
        ..Default::default()
    };
    client
        .reader_posts()
        .search(&params)
        .await
        .map(|response| response.data)
        .map_err(|e| e.to_string())
}

/// A page of "Automattician Blogs", a public list that Automattic has kept for years.
async fn list_posts(
    client: &WpComApiClient,
    page_handle: Option<String>,
) -> Result<ReaderListPostsResponse, String> {
    let params = ReaderListPostsParams {
        number: Some(10),
        page_handle,
        ..Default::default()
    };
    client
        .reader_posts()
        .list_posts(
            &ReaderListOwner("automattic".to_string()),
            &ReaderListSlug("a12s".to_string()),
            &params,
        )
        .await
        .map(|response| response.data)
        .map_err(|e| e.to_string())
}

/// Whether `date` is earlier than `cursor`, the `before` a page was asked for.
fn is_before(date: Option<WpGmtDateTime>, cursor: Option<WpGmtDateTime>) -> bool {
    matches!((date, cursor), (Some(date), Some(cursor)) if date.0 < cursor.0)
}

async fn freshly_pressed(
    client: &WpComApiClient,
    before: Option<WpGmtDateTime>,
) -> Result<ReaderPostsResponse, String> {
    let params = ReaderFreshlyPressedParams {
        number: Some(5),
        before,
        ..Default::default()
    };
    client
        .reader_posts()
        .freshly_pressed(&params)
        .await
        .map(|response| response.data)
        .map_err(|e| e.to_string())
}

/// A page of Freshly Pressed has posts, each with the time it's shown, and a range that ends at
/// the last one's.
fn check_freshly_pressed(page: &ReaderPostsResponse) -> Result<(), String> {
    let last = page.posts.last().ok_or("Freshly Pressed has no posts")?;
    if page
        .posts
        .iter()
        .any(|post| post.editorial_displayed_on.is_none())
    {
        return Err("A Freshly Pressed post should say when it's shown".into());
    }
    let cursor = page.date_range.as_ref().and_then(|range| range.after);
    if cursor.is_none() || cursor != last.editorial_displayed_on {
        return Err("The range should end at the time the last post is shown".into());
    }
    Ok(())
}
