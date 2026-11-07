use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::reader_saved::{ReaderSavedPostsParams, ReaderSavedSource};
use wp_mobile::service::reader_saved::{ReaderSavedService, ReaderSavedStatus};
use wp_mobile_cache::WpApiCache;

use crate::context::TestContext;

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    vec![
        Trial::test("reader_saved::list_posts", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    ctx.client
                        .reader_saved()
                        .list_posts(&ReaderSavedPostsParams::default())
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(())
                })
            }
        }),
        Trial::test("reader_saved::service_refresh_and_list", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let service = saved_service(&ctx, migrated_cache()?)?;
                    service.refresh().await.map_err(|e| e.to_string())?;
                    service
                        .list_posts(ReaderSavedPostsParams::default())
                        .await
                        .map_err(|e| e.to_string())?;
                    assert!(service.saved_count().is_some());
                    Ok(())
                })
            }
        }),
        Trial::test("reader_saved::saved_list_is_kept_in_the_cache", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let cache = migrated_cache()?;
                    let service = Arc::new(saved_service(&ctx, cache.clone())?);
                    let collection = Arc::clone(&service).saved_posts();
                    let page = collection.refresh().await.map_err(|e| e.to_string())?;
                    let items = collection.load_items().await.map_err(|e| e.to_string())?;
                    let count = service.saved_count().ok_or("No count after a refresh")?;
                    if count > 0 && items.is_empty() {
                        return Err(format!("{count} saves, but the list is empty").into());
                    }
                    if items.len() as u64 > page.post_count {
                        return Err("The list has posts the page didn't".into());
                    }
                    let first = items.first().cloned();
                    drop((collection, service));

                    // A new service on the same cache, as after a relaunch, knows the same
                    // without a request.
                    let service = Arc::new(saved_service(&ctx, cache)?);
                    if service.saved_count() != Some(count) {
                        return Err("The count wasn't kept".into());
                    }
                    if let Some(post) = first.and_then(|item| item.post)
                        && service.status(post.identity) != ReaderSavedStatus::Saved
                    {
                        return Err("A listed post isn't saved after a relaunch".into());
                    }
                    let reloaded = Arc::clone(&service)
                        .saved_posts()
                        .load_items()
                        .await
                        .map_err(|e| e.to_string())?;
                    if reloaded.len() != items.len() {
                        return Err("The list wasn't kept".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_saved::list_keys", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    ctx.client
                        .reader_saved()
                        .list_keys()
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(())
                })
            }
        }),
    ]
}

fn migrated_cache() -> Result<Arc<WpApiCache>, libtest_mimic::Failed> {
    let cache = WpApiCache::new(None).map_err(|e| e.to_string())?;
    cache.perform_migrations().map_err(|e| e.to_string())?;
    Ok(Arc::new(cache))
}

fn saved_service(
    ctx: &TestContext,
    cache: Arc<WpApiCache>,
) -> Result<ReaderSavedService, libtest_mimic::Failed> {
    // The cache is new for each test, so any user ID keeps them apart.
    Ok(ReaderSavedService::new(
        ctx.delegate.clone(),
        ReaderSavedSource::Ios,
        cache,
        WpComUserId(1),
    )
    .map_err(|e| e.to_string())?)
}
