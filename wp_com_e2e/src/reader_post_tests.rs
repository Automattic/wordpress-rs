use libtest_mimic::{Failed, Trial};
use std::sync::Arc;
use wp_api::api_error::{WpApiError, WpErrorCode};
use wp_api::posts::PostId;
use wp_api::prelude::*;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::client::WpComApiClient;
use wp_api::wp_com::reader_post::{
    ReaderPostBySlugParams, ReaderPostParams, ReaderPostSlug, ReaderRelatedPostsParams,
    ReaderResolveUrlParams,
};
use wp_api::wp_com::reader_posts::ReaderStreamParams;
use wp_api::wp_com::reader_saved::ReaderFeedId;
use wp_api::wp_com::reader_sites::ReaderSiteParams;
use wp_api::wp_com::sites::WpComSiteIdentifier;

use crate::context::TestContext;

/// The WordPress.com Blog, which is public.
const SITE: WpComSiteId = WpComSiteId(3584907);
const SITE_DOMAIN: &str = "en.blog.wordpress.com";
/// One of its posts, from 2014.
const POST: PostId = PostId(29614);
const POST_SLUG: &str = "new-year-new-blog-make-it-a-great-one";
const POST_URL: &str =
    "https://en.blog.wordpress.com/2014/12/29/new-year-new-blog-make-it-a-great-one/";
/// The xkcd RSS feed, which is public.
const FEED: ReaderFeedId = ReaderFeedId(8857199);
/// WordPress.org News: a self-hosted site, connected to WordPress.com with Jetpack.
const JETPACK_SITE: WpComSiteId = WpComSiteId(14607090);
/// Variety: a site on WordPress VIP, which is connected to WordPress.com with Jetpack too.
const VIP_SITE: WpComSiteId = WpComSiteId(179338616);
/// The WordPress.com account that a VIP site belongs to. Nothing in a VIP site's post says it's
/// one: the site's owner does.
const VIP_OWNER: &str = "wpcomvip";

/// A client that sends no token, to check what answers signed out.
pub(crate) fn signed_out_client(ctx: &TestContext) -> WpComApiClient {
    WpComApiClient::new(WpApiClientDelegate {
        auth_provider: WpAuthenticationProvider::none().into(),
        ..ctx.delegate.clone()
    })
}

/// Checks that a request failed with a WordPress.com error code, such as `unknown_post`.
pub(crate) fn expect_error_code<T>(
    result: Result<T, WpApiError>,
    expected_code: &str,
) -> Result<(), Failed> {
    match result {
        Err(WpApiError::WpError { error_code, .. })
            if error_code == WpErrorCode::CustomError(expected_code.to_string()) =>
        {
            Ok(())
        }
        Err(other) => Err(format!("Expected `{expected_code}`, got: {other:?}").into()),
        Ok(_) => Err(format!("Expected `{expected_code}`, but the request succeeded").into()),
    }
}

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    vec![
        Trial::test("reader_post::site_post", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let params = ReaderPostParams {
                        meta: Some("site,likes".to_string()),
                        ..Default::default()
                    };
                    let post = ctx
                        .client
                        .reader_post()
                        .site_post(&SITE, &POST, &params)
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if post.identity.site_id != Some(SITE) || post.identity.id != POST.0 as u64 {
                        return Err("It isn't the post that was asked for".into());
                    }
                    if post.identity.is_external || post.identity.feed_id.is_none() {
                        return Err("A site's post isn't external, and has its feed ID".into());
                    }
                    if post.title.is_none() || post.content.is_none() || post.site_name.is_none() {
                        return Err("The post is missing its title, content or site name".into());
                    }
                    if post.site_locale.is_none() {
                        return Err("`meta=site` should send the site's locale".into());
                    }

                    // Signed out, the same post answers.
                    let post = signed_out_client(&ctx)
                        .reader_post()
                        .site_post(&SITE, &POST, &ReaderPostParams::default())
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if post.identity.id != POST.0 as u64 || post.title.is_none() {
                        return Err("Signed out, the post should be the same".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_post::site_post_fields", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let params = ReaderPostParams {
                        fields: Some("ID,site_ID,date".to_string()),
                        ..Default::default()
                    };
                    let post = ctx
                        .client
                        .reader_post()
                        .site_post(&SITE, &POST, &params)
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if post.identity.site_id != Some(SITE) || post.date.is_none() {
                        return Err("The post is missing a field that was asked for".into());
                    }
                    if post.title.is_some() || post.content.is_some() {
                        return Err("The post has a field that wasn't asked for".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_post::site_post_that_is_not_there", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let missing_post = ctx
                        .client
                        .reader_post()
                        .site_post(&SITE, &PostId(999999999), &ReaderPostParams::default())
                        .await;
                    expect_error_code(missing_post, "unknown_post")?;

                    let missing_site = ctx
                        .client
                        .reader_post()
                        .site_post(
                            &WpComSiteId(99999999999),
                            &PostId(1),
                            &ReaderPostParams::default(),
                        )
                        .await;
                    expect_error_code(missing_site, "unknown_blog")
                })
            }
        }),
        Trial::test("reader_post::feed_post", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    // Feed items are pruned over time, so take one the feed has now.
                    let params = ReaderStreamParams {
                        number: Some(1),
                        ..Default::default()
                    };
                    let page = ctx
                        .client
                        .reader_posts()
                        .feed_posts(&FEED, &params)
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    let in_stream = page.posts.first().ok_or("The feed has no posts")?;
                    let item = in_stream
                        .identity
                        .feed_item_id
                        .ok_or("A feed's post should have a feed item ID")?;

                    for client in [&ctx.client, &signed_out_client(&ctx)] {
                        let post = client
                            .reader_post()
                            .feed_post(&FEED, &item, &ReaderPostParams::default())
                            .await
                            .map_err(|e| e.to_string())?
                            .data;
                        if post.identity != in_stream.identity {
                            return Err("It should be the post the stream sent".into());
                        }
                        if !post.identity.is_external || post.title.is_none() {
                            return Err("An RSS post is external, and has a title".into());
                        }
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_post::feed_post_of_a_site", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    // The site's newest post that is in its feed. A post published in the last
                    // few hours can come without a feed item ID, until the feed has picked it up.
                    let params = ReaderStreamParams {
                        number: Some(5),
                        ..Default::default()
                    };
                    let page = ctx
                        .client
                        .reader_posts()
                        .site_posts(&SITE, &params)
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if page.posts.is_empty() {
                        return Err("The site has no posts".into());
                    }
                    let Some((in_stream, feed, item)) = page.posts.iter().find_map(|post| {
                        Some((post, post.identity.feed_id?, post.identity.feed_item_id?))
                    }) else {
                        return Err("None of the site's newest posts has its feed IDs".into());
                    };

                    let post = ctx
                        .client
                        .reader_post()
                        .feed_post(&feed, &item, &ReaderPostParams::default())
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if post.identity.is_external
                        || post.identity.id != in_stream.identity.id
                        || post.identity.site_id != Some(SITE)
                        || post.identity.feed_item_id != Some(item)
                    {
                        return Err("It should be the site's post, with its feed item ID".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_post::jetpack_post", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime
                    .block_on(check_jetpack_site_post(&ctx, JETPACK_SITE, false))
                    .map_err(Failed::from)
            }
        }),
        Trial::test("reader_post::vip_post", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime
                    .block_on(check_jetpack_site_post(&ctx, VIP_SITE, true))
                    .map_err(Failed::from)
            }
        }),
        Trial::test("reader_post::feed_post_that_is_not_there", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let missing_item = ctx
                        .client
                        .reader_post()
                        .feed_post(
                            &FEED,
                            &wp_api::wp_com::reader_saved::ReaderFeedItemId(1),
                            &ReaderPostParams::default(),
                        )
                        .await;
                    expect_error_code(missing_item, "no_such_item")
                })
            }
        }),
        Trial::test("reader_post::post_by_slug", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let slug = ReaderPostSlug(POST_SLUG.to_string());
                    let by_domain = WpComSiteIdentifier::Slug {
                        value: SITE_DOMAIN.to_string(),
                    };
                    let post = ctx
                        .client
                        .reader_post()
                        .post_by_slug(&by_domain, &slug, &ReaderPostBySlugParams::default())
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if post.site_id != SITE || post.post_id != POST {
                        return Err("The slug should find the post".into());
                    }
                    if post.title.is_none() || post.status.as_deref() != Some("publish") {
                        return Err("The post is missing its title or status".into());
                    }

                    // Signed out, by site ID, and with only its IDs.
                    let by_id = WpComSiteIdentifier::Id { value: SITE.0 };
                    let params = ReaderPostBySlugParams {
                        fields: Some("ID,site_ID".to_string()),
                    };
                    let post = signed_out_client(&ctx)
                        .reader_post()
                        .post_by_slug(&by_id, &slug, &params)
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if post.site_id != SITE || post.post_id != POST || post.title.is_some() {
                        return Err("Asked for its IDs, the post should have only those".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_post::post_by_slug_that_is_not_there", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let site = WpComSiteIdentifier::Id { value: SITE.0 };
                    let missing = ctx
                        .client
                        .reader_post()
                        .post_by_slug(
                            &site,
                            &ReaderPostSlug("no-such-slug-xyzzy-12345".to_string()),
                            &ReaderPostBySlugParams::default(),
                        )
                        .await;
                    expect_error_code(missing, "unknown_post")
                })
            }
        }),
        Trial::test("reader_post::related_posts", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let params = ReaderRelatedPostsParams {
                        size_local: Some(2),
                        size_global: Some(2),
                        ..Default::default()
                    };
                    for client in [&ctx.client, &signed_out_client(&ctx)] {
                        let related = client
                            .reader_post()
                            .related_posts(&SITE, &POST, &params)
                            .await
                            .map_err(|e| e.to_string())?
                            .data;
                        if related.posts.is_empty() || related.posts.len() > 4 {
                            return Err("There should be up to four related posts".into());
                        }
                        if related.posts.iter().any(|post| post.railcar.is_none()) {
                            return Err("A related post should say how it was picked".into());
                        }
                        if !related
                            .posts
                            .iter()
                            .any(|post| post.identity.site_id == Some(SITE))
                        {
                            return Err("Some related posts should be from the same site".into());
                        }
                    }

                    // Without a size, the API returns none.
                    let none = ctx
                        .client
                        .reader_post()
                        .related_posts(&SITE, &POST, &ReaderRelatedPostsParams::default())
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if !none.posts.is_empty() {
                        return Err("Without a size, there should be no related posts".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_post::resolve_url", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    // The address as it is, and with what a shared link can carry.
                    for url in [
                        POST_URL.to_string(),
                        format!("{POST_URL}?utm_source=example#comments"),
                    ] {
                        let resolved = ctx
                            .client
                            .reader_post()
                            .resolve_url(&ReaderResolveUrlParams { url })
                            .await
                            .map_err(|e| e.to_string())?
                            .data;
                        if resolved.site_id != SITE || resolved.post_id != POST {
                            return Err("The address should resolve to the post".into());
                        }
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_post::resolve_url_that_is_not_a_post", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let resolve = |url: &str| {
                        let params = ReaderResolveUrlParams {
                            url: url.to_string(),
                        };
                        let ctx = Arc::clone(&ctx);
                        async move { ctx.client.reader_post().resolve_url(&params).await }
                    };

                    // A post that isn't there, and a Reader screen.
                    let missing = "https://en.blog.wordpress.com/2014/12/29/no-such-post-xyzzy/";
                    expect_error_code(resolve(missing).await, "post_not_found")?;
                    let reader = "https://wordpress.com/reader/blogs/3584907/posts/29614";
                    expect_error_code(resolve(reader).await, "post_not_found")?;

                    // A site's address has no path to find a post with.
                    match resolve("https://en.blog.wordpress.com/").await {
                        Err(WpApiError::WpError {
                            error_code: WpErrorCode::InvalidParam,
                            ..
                        }) => Ok(()),
                        other => Err(format!("Expected an invalid parameter: {other:?}").into()),
                    }
                })
            }
        }),
        Trial::test("reader_post::resolve_url_signed_out", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let params = ReaderResolveUrlParams {
                        url: POST_URL.to_string(),
                    };
                    match signed_out_client(&ctx)
                        .reader_post()
                        .resolve_url(&params)
                        .await
                    {
                        Err(WpApiError::WpError {
                            error_code: WpErrorCode::Forbidden,
                            status_code: 401,
                            ..
                        }) => Ok(()),
                        other => Err(format!("Expected a 401: {other:?}").into()),
                    }
                })
            }
        }),
    ]
}

/// Checks a Jetpack site's post, or a VIP site's, through the site's endpoint and its feed's.
///
/// The site has to still be that kind of site: a Jetpack site, owned by the VIP account when `vip`
/// and by any other account when not. One of the site's newest posts that is in its feed is taken,
/// so the post never ages. Through the site's endpoint it's the site's post. Through the feed's it's
/// the same post, merged with its feed item and marked as a Jetpack post. A feed item the server
/// hasn't merged with the site's post yet is external, so the next post is tried.
async fn check_jetpack_site_post(
    ctx: &TestContext,
    site_id: WpComSiteId,
    vip: bool,
) -> Result<(), String> {
    let site = ctx
        .client
        .reader_sites()
        .site(
            &WpComSiteIdentifier::Id { value: site_id.0 },
            &ReaderSiteParams::default(),
        )
        .await
        .map_err(|e| e.to_string())?
        .data;
    let name = site.name.clone().unwrap_or_else(|| site_id.0.to_string());
    if site.is_jetpack != Some(true) {
        return Err(format!(
            "{name} is no longer a Jetpack site, so the trial needs another site"
        ));
    }
    let owner = site.owner.as_ref().and_then(|owner| owner.login.as_deref());
    if vip != (owner == Some(VIP_OWNER)) {
        return Err(format!(
            "{name} is owned by {owner:?}, so it's no longer a {} site: the trial needs another \
             site",
            if vip { "VIP" } else { "non-VIP Jetpack" }
        ));
    }

    let params = ReaderStreamParams {
        number: Some(10),
        ..Default::default()
    };
    let page = ctx
        .client
        .reader_posts()
        .site_posts(&site_id, &params)
        .await
        .map_err(|e| e.to_string())?
        .data;
    let in_feed: Vec<_> = page
        .posts
        .iter()
        .filter_map(|post| Some((post, post.identity.feed_id?, post.identity.feed_item_id?)))
        .collect();
    if in_feed.is_empty() {
        return Err(format!("None of {name}'s newest posts is in its feed"));
    }

    let mut not_merged = 0;
    for (in_stream, feed, item) in in_feed {
        let post_id = PostId(in_stream.identity.id as i64);
        let post = ctx
            .client
            .reader_post()
            .site_post(&site_id, &post_id, &ReaderPostParams::default())
            .await
            .map_err(|e| e.to_string())?
            .data;
        if post.identity.is_external
            || post.identity.id != in_stream.identity.id
            || post.identity.site_id != Some(site_id)
            || post.identity.feed_id != Some(feed)
        {
            return Err(format!(
                "Through the site's endpoint, {name}'s post should be the site's post, with its \
                 feed ID: {:?}",
                post.identity
            ));
        }
        if post.is_jetpack.is_some() || post.identity.feed_item_id.is_some() {
            return Err(format!(
                "Through the site's endpoint, {name}'s post should leave out `is_jetpack` and its \
                 feed item ID, as the `reader_post` module says"
            ));
        }

        let copy = ctx
            .client
            .reader_post()
            .feed_post(&feed, &item, &ReaderPostParams::default())
            .await
            .map_err(|e| e.to_string())?
            .data;
        if copy.is_jetpack != Some(true) {
            return Err(format!(
                "Through the feed's endpoint, {name}'s post should be a Jetpack post"
            ));
        }
        if copy.identity.is_external {
            // The feed item, not yet merged with the site's post: its ID is the feed item's.
            if copy.identity.id != item.0 {
                return Err(format!(
                    "An external Jetpack post should have its feed item ID as its ID: {:?}",
                    copy.identity
                ));
            }
            not_merged += 1;
            continue;
        }
        if copy.identity.id != in_stream.identity.id
            || copy.identity.site_id != Some(site_id)
            || copy.identity.feed_item_id != Some(item)
        {
            return Err(format!(
                "Through the feed's endpoint, {name}'s post should be the site's post, with its \
                 feed item ID: {:?}",
                copy.identity
            ));
        }
        return Ok(());
    }
    Err(format!(
        "None of {name}'s {not_merged} newest posts in its feed is merged with the site's post"
    ))
}
