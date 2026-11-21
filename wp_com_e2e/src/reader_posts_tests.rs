use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::date::WpGmtDateTime;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::client::WpComApiClient;
use wp_api::wp_com::reader_posts::{
    ReaderCard, ReaderDiscoverParams, ReaderDiscoverResponse, ReaderFirstPostsParams,
    ReaderFirstPostsResponse, ReaderFreshlyPressedParams, ReaderLikedPostsParams, ReaderListOwner,
    ReaderListPostsParams, ReaderListPostsResponse, ReaderListSlug, ReaderOrganizationPostsParams,
    ReaderPost, ReaderPostsResponse, ReaderSearchPostsParams, ReaderSearchPostsResponse,
    ReaderSearchSort, ReaderStreamParams, ReaderTagPostsParams, ReaderTagPostsResponse,
    ReaderTagPostsSort, ReaderTagSlug, ReaderTagsPostsParams,
};
use wp_api::wp_com::reader_saved::ReaderFeedId;

use crate::context::{TestContext, reading};

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
                    // with posts is other posts. A search by date pages by time window, and a
                    // window can be empty without being the last.
                    let by_date =
                        |handle| search(&ctx.client, Some(ReaderSearchSort::Date), handle);
                    let has_posts = |page: &ReaderSearchPostsResponse| !page.posts.is_empty();
                    let next = |page: &ReaderSearchPostsResponse| page.next_page_handle.clone();
                    let first =
                        first_page_with_posts("By date", None, by_date, has_posts, next).await?;
                    if first.posts.iter().any(|p| p.date_added.is_none()) {
                        return Err("A result by date should say when it was picked up".into());
                    }
                    let handle = first
                        .next_page_handle
                        .clone()
                        .ok_or("The first page by date with posts has no handle for the next")?;
                    let second =
                        first_page_with_posts("By date", Some(handle), by_date, has_posts, next)
                            .await?;
                    let seen = |page: &ReaderSearchPostsResponse| {
                        page.posts
                            .iter()
                            .map(|result| (result.post.identity.site_id, result.post.identity.id))
                            .collect::<std::collections::HashSet<_>>()
                    };
                    if !seen(&first).is_disjoint(&seen(&second)) {
                        return Err("The next page by date should be other posts".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_posts::list_posts", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let fetch = |handle| list_posts(&ctx.client, handle);
                    let has_posts = |page: &ReaderListPostsResponse| !page.posts.is_empty();
                    let next = |page: &ReaderListPostsResponse| page.next_page_handle.clone();
                    let first =
                        first_page_with_posts("The list", None, fetch, has_posts, next).await?;
                    let handle = first
                        .next_page_handle
                        .clone()
                        .ok_or("The first page has no handle for the next")?;

                    // Paging by handle: the next page with posts is other posts.
                    let second =
                        first_page_with_posts("The list", Some(handle), fetch, has_posts, next)
                            .await?;
                    let seen = |page: &ReaderListPostsResponse| {
                        page.posts
                            .iter()
                            .map(|post| (post.identity.site_id, post.identity.id))
                            .collect::<std::collections::HashSet<_>>()
                    };
                    if !seen(&first).is_disjoint(&seen(&second)) {
                        return Err("The next page should be other posts".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_posts::discover", {
            let ctx = Arc::clone(&ctx);
            move || {
                // With no tags asked for, Discover is the account's own, which can depend on the tags it
                // follows, and two pages are compared.
                let _tags = reading(&ctx.locks.tags);
                ctx.runtime.block_on(async {
                    let page = |params: ReaderDiscoverParams| {
                        let ctx = Arc::clone(&ctx);
                        async move {
                            let page = ctx
                                .client
                                .reader_posts()
                                .discover(&params)
                                .await
                                .map_err(|e| e.to_string())?
                                .data;
                            check_discover_cards(&page)?;
                            Ok(page)
                        }
                    };

                    let by_handle = |page_handle| {
                        page(ReaderDiscoverParams {
                            page_handle,
                            ..Default::default()
                        })
                    };
                    let has_posts =
                        |page: &ReaderDiscoverResponse| !card_posts(&page.cards).is_empty();
                    let next = |page: &ReaderDiscoverResponse| page.next_page_handle.clone();

                    // A page is posts, and usually a card of recommended sites among them, which
                    // `check_discover_cards` reads. Which cards come depends on the account and
                    // the moment: CI's account once had a page with none.
                    let first =
                        first_page_with_posts("Discover", None, by_handle, has_posts, next).await?;
                    let handle = first
                        .next_page_handle
                        .clone()
                        .ok_or("The first page has no handle for the next")?;

                    // Paging by handle: the next page with posts is other posts.
                    let second =
                        first_page_with_posts("Discover", Some(handle), by_handle, has_posts, next)
                            .await?;
                    if !card_posts(&first.cards).is_disjoint(&card_posts(&second.cards)) {
                        return Err("The next page should be other posts".into());
                    }

                    // Asked for with tags, a page ends with a card of tags the user may like. It
                    // answers the tags asked for, not the account: signed out and signed in got
                    // the same tags.
                    let tagged = page(ReaderDiscoverParams {
                        tags: vec![ReaderTagSlug("photography".to_string())],
                        ..Default::default()
                    })
                    .await?;
                    if !matches!(tagged.cards.last(), Some(ReaderCard::SuggestedTags { .. })) {
                        return Err("A page asked for with tags should end with tags".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_posts::first_posts", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let page = |page_handle| {
                        let params = ReaderFirstPostsParams {
                            page_handle,
                            ..Default::default()
                        };
                        let ctx = Arc::clone(&ctx);
                        async move {
                            let page = ctx
                                .client
                                .reader_posts()
                                .first_posts(&params)
                                .await
                                .map_err(|e| e.to_string())?
                                .data;
                            match page.unreadable_cards.first() {
                                Some(card) => Err(format!("A card couldn't be read: {card:?}")),
                                None => Ok(page),
                            }
                        }
                    };

                    let has_posts =
                        |page: &ReaderFirstPostsResponse| !card_posts(&page.cards).is_empty();
                    let next = |page: &ReaderFirstPostsResponse| page.next_page_handle.clone();

                    // A page is posts, then a card of new sites.
                    let first =
                        first_page_with_posts("The stream", None, page, has_posts, next).await?;
                    if !matches!(first.cards.last(), Some(ReaderCard::NewSites { .. })) {
                        return Err("A page should end with a card of new sites".into());
                    }
                    let handle = first
                        .next_page_handle
                        .clone()
                        .ok_or("The first page has no handle for the next")?;

                    // Paging by handle: the next page with posts is other posts.
                    let second =
                        first_page_with_posts("The stream", Some(handle), page, has_posts, next)
                            .await?;
                    if !card_posts(&first.cards).is_disjoint(&card_posts(&second.cards)) {
                        return Err("The next page should be other posts".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_posts::tags_posts", {
            let ctx = Arc::clone(&ctx);
            move || {
                // The posts of the tags the account follows.
                let _tags = reading(&ctx.locks.tags);
                ctx.runtime.block_on(async {
                    let seen = |page: &ReaderTagPostsResponse| {
                        page.posts
                            .iter()
                            .map(|post| (post.identity.site_id, post.identity.id))
                            .collect::<std::collections::HashSet<_>>()
                    };

                    // By date, the next page is the one the handle names.
                    let by_date = |page_handle| {
                        tags_posts(
                            &ctx.client,
                            ReaderTagsPostsParams {
                                page_handle,
                                ..Default::default()
                            },
                        )
                    };
                    let has_posts = |page: &ReaderTagPostsResponse| !page.posts.is_empty();
                    let next = |page: &ReaderTagPostsResponse| page.next_page_handle.clone();
                    let first =
                        first_page_with_posts("The tags", None, by_date, has_posts, next).await?;
                    let handle = first
                        .next_page_handle
                        .clone()
                        .ok_or("The first page has no handle for the next")?;
                    let second =
                        first_page_with_posts("The tags", Some(handle), by_date, has_posts, next)
                            .await?;
                    if !seen(&first).is_disjoint(&seen(&second)) {
                        return Err("The next page by date should be other posts".into());
                    }

                    // By popularity, the next page is the next page number.
                    let popular = |page| {
                        tags_posts(
                            &ctx.client,
                            ReaderTagsPostsParams {
                                sort: Some(ReaderTagPostsSort::Popularity),
                                page,
                                ..Default::default()
                            },
                        )
                    };
                    let first = popular(None).await?;
                    let second = popular(Some(2)).await?;
                    if first.posts.is_empty()
                        || second.posts.is_empty()
                        || !seen(&first).is_disjoint(&seen(&second))
                    {
                        return Err("The second page by popularity should be other posts".into());
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

                    // So do Discover, the first posts stream and the posts of several tags.
                    let discover = reader_posts
                        .discover(&ReaderDiscoverParams::default())
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if card_posts(&discover.cards).is_empty() {
                        return Err("Discover should have posts signed out".into());
                    }
                    let first_posts = reader_posts
                        .first_posts(&ReaderFirstPostsParams::default())
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if card_posts(&first_posts.cards).is_empty() {
                        return Err("The first posts stream should have posts signed out".into());
                    }
                    if tags_posts(&ctx.signed_out_client, ReaderTagsPostsParams::default())
                        .await?
                        .posts
                        .is_empty()
                    {
                        return Err("The posts of tags should arrive signed out".into());
                    }
                    Ok(())
                })
            }
        }),
    ]
}

/// How many pages a trial asks for, from a handle, to find one with posts.
const MAX_PAGES_TO_POSTS: usize = 12;

/// The first page from `handle` on that has posts. A stream that pages by handle can send a page
/// with none that isn't its last: a search by date pages by time window, and six empty windows in
/// a row have been seen. So an empty page is followed, and the trial fails if one has no handle
/// for the next, or if no page of the first [`MAX_PAGES_TO_POSTS`] has posts.
async fn first_page_with_posts<P, F, Fut>(
    what: &str,
    mut handle: Option<String>,
    fetch: F,
    has_posts: impl Fn(&P) -> bool,
    next_handle: impl Fn(&P) -> Option<String>,
) -> Result<P, String>
where
    F: Fn(Option<String>) -> Fut,
    Fut: std::future::Future<Output = Result<P, String>>,
{
    for _ in 0..MAX_PAGES_TO_POSTS {
        let page = fetch(handle.take()).await?;
        if has_posts(&page) {
            return Ok(page);
        }
        handle =
            Some(next_handle(&page).ok_or_else(|| {
                format!("{what}: a page with no posts has no handle for the next")
            })?);
    }
    Err(format!(
        "{what}: none of {MAX_PAGES_TO_POSTS} pages in a row has posts"
    ))
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

/// The posts on a page of cards.
fn card_posts(cards: &[ReaderCard]) -> std::collections::HashSet<(Option<WpComSiteId>, u64)> {
    cards
        .iter()
        .filter_map(|card| match card {
            ReaderCard::Post { post } => Some((post.identity.site_id, post.identity.id)),
            _ => None,
        })
        .collect()
}

/// Every card on a page of Discover can be read and is of a kind the library knows, and a card of
/// recommended sites names each site, with its address. A card of recommended sites isn't
/// required: the server doesn't promise one on a page, or for an account.
fn check_discover_cards(page: &ReaderDiscoverResponse) -> Result<(), String> {
    if let Some(card) = page.unreadable_cards.first() {
        return Err(format!("A card couldn't be read: {card:?}"));
    }
    for card in &page.cards {
        match card {
            ReaderCard::Unknown { card_type } => {
                return Err(format!(
                    "A card is of a kind the library doesn't know: {card_type}"
                ));
            }
            // A site's name is up to its owner and can be empty, so only the ID and address are
            // required.
            ReaderCard::RecommendedSites { sites } => {
                if let Some(site) = sites
                    .iter()
                    .find(|site| site.site_id.is_none() || site.url.is_none())
                {
                    return Err(format!(
                        "A recommended site is missing its ID or address: {site:?}"
                    ));
                }
            }
            ReaderCard::Post { .. }
            | ReaderCard::NewSites { .. }
            | ReaderCard::SuggestedTags { .. } => {}
        }
    }
    Ok(())
}

/// A page of the posts of two tags busy enough to always have a second page. `params` is the rest
/// of the request.
async fn tags_posts(
    client: &WpComApiClient,
    params: ReaderTagsPostsParams,
) -> Result<ReaderTagPostsResponse, String> {
    let params = ReaderTagsPostsParams {
        tags: vec![
            ReaderTagSlug("photography".to_string()),
            ReaderTagSlug("travel".to_string()),
        ],
        ..params
    };
    client
        .reader_posts()
        .tags_posts(&params)
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
