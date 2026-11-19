use libtest_mimic::Trial;
use std::collections::HashSet;
use std::sync::Arc;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_saved::ReaderFeedId;
use wp_api::wp_com::reader_sites::{
    ReaderFeedSearchParams, ReaderFeedSearchResult, ReaderFeedSearchSort, ReaderSiteParams,
};
use wp_api::wp_com::sites::WpComSiteIdentifier;

use crate::context::TestContext;
use crate::reader_post_tests::{expect_error_code, signed_out_client};

/// The WordPress.com Blog, which is public.
const SITE: WpComSiteId = WpComSiteId(3584907);
const SITE_DOMAIN: &str = "en.blog.wordpress.com";
/// The xkcd RSS feed, which is public and belongs to no WordPress.com site.
const FEED: ReaderFeedId = ReaderFeedId(8857199);
const FEED_ADDRESS: &str = "https://xkcd.com/atom.xml";

/// What tells two results apart: a result always has a site ID, a feed ID or both.
fn keys(
    results: &[ReaderFeedSearchResult],
) -> HashSet<(Option<WpComSiteId>, Option<ReaderFeedId>)> {
    results
        .iter()
        .map(|result| (result.site_id, result.feed_id))
        .collect()
}

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    vec![
        Trial::test("reader_sites::site", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let by_id = WpComSiteIdentifier::Id { value: SITE.0 };
                    let site = ctx
                        .client
                        .reader_sites()
                        .site(&by_id, &ReaderSiteParams::default())
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if site.site_id != SITE || site.name.is_none() || site.url.is_none() {
                        return Err("The site is missing its ID, name or address".into());
                    }
                    if site.feed_id.is_none() || site.subscribers_count.is_none() {
                        return Err("The site is missing its feed ID or subscriber count".into());
                    }
                    if site.is_following.is_none() || site.delivery_methods.is_none() {
                        return Err("The site should say whether the user follows it".into());
                    }
                    if site.lang.is_none() {
                        return Err("Signed in, the site should have its language".into());
                    }

                    // By domain, it's the same site.
                    let by_domain = WpComSiteIdentifier::Slug {
                        value: SITE_DOMAIN.to_string(),
                    };
                    let same = ctx
                        .client
                        .reader_sites()
                        .site(&by_domain, &ReaderSiteParams::default())
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if same.site_id != SITE || same.feed_id != site.feed_id {
                        return Err("The domain should find the same site".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_sites::site_signed_out", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let site = signed_out_client(&ctx)
                        .reader_sites()
                        .site(
                            &WpComSiteIdentifier::Id { value: SITE.0 },
                            &ReaderSiteParams::default(),
                        )
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if site.site_id != SITE || site.name.is_none() || site.feed_id.is_none() {
                        return Err("Signed out, the site should still have its details".into());
                    }
                    if site.is_following != Some(false) || site.unseen_count != Some(0) {
                        return Err("Signed out, nobody follows the site".into());
                    }
                    let emails = site.delivery_methods.and_then(|methods| methods.email);
                    if emails.is_some() || site.lang.is_some() || site.post_count.is_some() {
                        return Err("Signed out, the site has a field it shouldn't".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_sites::site_fields", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let params = ReaderSiteParams {
                        fields: Some("ID,feed_ID,is_following".to_string()),
                        ..Default::default()
                    };
                    let site = ctx
                        .client
                        .reader_sites()
                        .site(&WpComSiteIdentifier::Id { value: SITE.0 }, &params)
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if site.site_id != SITE || site.feed_id.is_none() || site.is_following.is_none()
                    {
                        return Err("The site is missing a field that was asked for".into());
                    }
                    if site.name.is_some() || site.owner.is_some() {
                        return Err("The site has a field that wasn't asked for".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_sites::site_that_is_not_there", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let missing = ctx
                        .client
                        .reader_sites()
                        .site(
                            &WpComSiteIdentifier::Id { value: 99999999999 },
                            &ReaderSiteParams::default(),
                        )
                        .await;
                    expect_error_code(missing, "unknown_blog")
                })
            }
        }),
        Trial::test("reader_sites::feed", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    for client in [&ctx.client, &signed_out_client(&ctx)] {
                        let feed = client
                            .reader_sites()
                            .feed(&FEED)
                            .await
                            .map_err(|e| e.to_string())?
                            .data;
                        if feed.feed_id != FEED || feed.name.is_none() || feed.feed_url.is_none() {
                            return Err("The feed is missing its ID, name or address".into());
                        }
                        if feed.site_id.is_some() || feed.owner.is_some() {
                            return Err("An RSS feed from elsewhere belongs to no site".into());
                        }
                        if feed.is_following.is_none() || feed.last_checked.is_none() {
                            return Err("The feed is missing a field every feed has".into());
                        }
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_sites::site_and_feed_link_to_each_other", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let site = ctx
                        .client
                        .reader_sites()
                        .site(
                            &WpComSiteIdentifier::Id { value: SITE.0 },
                            &ReaderSiteParams::default(),
                        )
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    let feed_id = site.feed_id.ok_or("The site should have a feed")?;
                    let feed = ctx
                        .client
                        .reader_sites()
                        .feed(&feed_id)
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if feed.feed_id != feed_id || feed.site_id != Some(SITE) {
                        return Err("The site's feed should lead back to the site".into());
                    }
                    if feed.owner.is_none() {
                        return Err("A site's feed should have the site's owner".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_sites::feed_that_is_not_there", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let missing = ctx
                        .client
                        .reader_sites()
                        .feed(&ReaderFeedId(99999999999))
                        .await;
                    expect_error_code(missing, "no_such_feed")
                })
            }
        }),
        Trial::test("reader_sites::search_feeds", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    // A search busy enough to always have a second page.
                    let search = |offset: Option<u32>| {
                        let params = ReaderFeedSearchParams {
                            q: Some("photography".to_string()),
                            number: Some(5),
                            offset,
                            ..Default::default()
                        };
                        let ctx = Arc::clone(&ctx);
                        async move {
                            ctx.client
                                .reader_sites()
                                .search_feeds(&params)
                                .await
                                .map(|response| response.data)
                                .map_err(|e| e.to_string())
                        }
                    };
                    let first = search(None).await?;
                    if first.feeds.len() != 5 || first.total.is_none_or(|total| total <= 5) {
                        return Err("The search should find more than a page of results".into());
                    }
                    if first.feeds.iter().any(|result| result.title.is_none()) {
                        return Err("A search result should have a title".into());
                    }
                    if first.next_page_handle.is_some() {
                        return Err("Sorted by relevance, a search pages by offset".into());
                    }

                    // Paging by offset: the next page is other results.
                    let offset = first.next_offset.ok_or("There should be a next offset")?;
                    let second = search(Some(offset)).await?;
                    if second.feeds.is_empty()
                        || !keys(&first.feeds).is_disjoint(&keys(&second.feeds))
                    {
                        return Err("The second page should be other results".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_sites::search_feeds_by_last_update", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let search = |page_handle: Option<String>| {
                        let params = ReaderFeedSearchParams {
                            q: Some("photography".to_string()),
                            number: Some(5),
                            sort: Some(ReaderFeedSearchSort::LastUpdated),
                            page_handle,
                            ..Default::default()
                        };
                        let ctx = Arc::clone(&ctx);
                        async move {
                            ctx.client
                                .reader_sites()
                                .search_feeds(&params)
                                .await
                                .map(|response| response.data)
                                .map_err(|e| e.to_string())
                        }
                    };
                    let first = search(None).await?;
                    if first.feeds.is_empty() || first.next_offset.is_some() {
                        return Err("Sorted by last update, a search pages by handle".into());
                    }

                    // Paging by handle: the next page is other results.
                    let handle = first
                        .next_page_handle
                        .clone()
                        .ok_or("There should be a handle for the next page")?;
                    let second = search(Some(handle)).await?;
                    if second.feeds.is_empty()
                        || !keys(&first.feeds).is_disjoint(&keys(&second.feeds))
                    {
                        return Err("The second page should be other results".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_sites::search_feeds_with_meta", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let params = ReaderFeedSearchParams {
                        q: Some("photography".to_string()),
                        number: Some(5),
                        meta: Some("site,feed".to_string()),
                        ..Default::default()
                    };
                    // Signed out too: a search answers with no account.
                    for client in [&ctx.client, &signed_out_client(&ctx)] {
                        let page = client
                            .reader_sites()
                            .search_feeds(&params)
                            .await
                            .map_err(|e| e.to_string())?
                            .data;
                        let with_site = page
                            .feeds
                            .iter()
                            .find_map(|result| result.site.as_ref().map(|site| (result, site)));
                        let (result, site) = with_site.ok_or("No result has its site")?;
                        if Some(site.site_id) != result.site_id {
                            return Err("A result's site should be the result's own".into());
                        }
                        if !page.feeds.iter().any(|result| result.feed.is_some()) {
                            return Err("No result has its feed".into());
                        }
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_sites::search_feeds_by_address", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let params = ReaderFeedSearchParams {
                        url: Some(FEED_ADDRESS.to_string()),
                        meta: Some("feed".to_string()),
                        ..Default::default()
                    };
                    let page = ctx
                        .client
                        .reader_sites()
                        .search_feeds(&params)
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    let result = page
                        .feeds
                        .iter()
                        .find(|result| result.feed_id == Some(FEED))
                        .ok_or("The address should find its feed")?;
                    if result.subscribe_url.is_none() || page.total.is_some() {
                        return Err("A lookup has an address to follow, and no total".into());
                    }
                    let feed = result.feed.as_ref().ok_or("`meta=feed` sends the feed")?;
                    if feed.feed_id != FEED || feed.site_id.is_some() {
                        return Err("The feed should be the one found, with no site".into());
                    }

                    // A site's address finds the site's feed, which leads back to the site.
                    let params = ReaderFeedSearchParams {
                        url: Some(format!("https://{SITE_DOMAIN}")),
                        meta: Some("feed".to_string()),
                        ..Default::default()
                    };
                    let page = signed_out_client(&ctx)
                        .reader_sites()
                        .search_feeds(&params)
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    let leads_back = page.feeds.iter().any(|result| {
                        result
                            .feed
                            .as_ref()
                            .is_some_and(|feed| feed.site_id == Some(SITE))
                    });
                    if !leads_back {
                        return Err("A site's address should find a feed of the site".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_sites::search_feeds_that_cannot_be_answered", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let search = |params: ReaderFeedSearchParams| {
                        let ctx = Arc::clone(&ctx);
                        async move { ctx.client.reader_sites().search_feeds(&params).await }
                    };

                    // Neither a query nor an address.
                    let neither = search(ReaderFeedSearchParams::default()).await;
                    expect_error_code(neither, "missing_param")?;

                    // An address with no feed.
                    let no_feed = search(ReaderFeedSearchParams {
                        url: Some("https://example.com/".to_string()),
                        ..Default::default()
                    })
                    .await;
                    expect_error_code(no_feed, "no_such_feed")?;

                    // More results than a page can hold, and past the last one a search reaches.
                    for (number, offset) in [(Some(21), None), (None, Some(200))] {
                        let too_far = search(ReaderFeedSearchParams {
                            q: Some("photography".to_string()),
                            number,
                            offset,
                            ..Default::default()
                        })
                        .await;
                        expect_error_code(too_far, "Bad Request")?;
                    }
                    Ok(())
                })
            }
        }),
    ]
}
