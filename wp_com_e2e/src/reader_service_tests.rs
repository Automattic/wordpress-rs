//! `ReaderService` against the live API: streams fetched, kept in a cache, and paged.

use libtest_mimic::{Failed, Trial};
use std::collections::HashSet;
use std::sync::Arc;
use wp_api::prelude::*;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::reader_posts::ReaderTagSlug;
use wp_mobile::service::reader::{
    ReaderError, ReaderService, ReaderStream, ReaderStreamCollection, ReaderStreamItem,
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
                    // The cache is new for each test, so any user ID keeps them apart.
                    let owner = ReaderOwner::Account {
                        user_id: WpComUserId(1),
                    };
                    let service = service(ctx.delegate.clone(), owner)?;
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
                    let service = service(signed_out(&ctx.delegate), ReaderOwner::SignedOut)?;
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
                    let service = service(signed_out(&ctx.delegate), ReaderOwner::SignedOut)?;
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
        Trial::test("reader_service::following_needs_an_account", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let service = service(signed_out(&ctx.delegate), ReaderOwner::SignedOut)?;
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

fn service(delegate: WpApiClientDelegate, owner: ReaderOwner) -> Result<ReaderService, Failed> {
    let cache = WpApiCache::new(None).map_err(|e| e.to_string())?;
    cache.perform_migrations().map_err(|e| e.to_string())?;
    Ok(ReaderService::new(delegate, Arc::new(cache), owner).map_err(|e| e.to_string())?)
}

/// The same delegate, sending no authentication.
fn signed_out(delegate: &WpApiClientDelegate) -> WpApiClientDelegate {
    WpApiClientDelegate {
        auth_provider: Arc::new(WpAuthenticationProvider::none()),
        ..delegate.clone()
    }
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
