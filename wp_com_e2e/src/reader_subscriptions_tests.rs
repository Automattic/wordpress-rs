use libtest_mimic::Trial;
use std::collections::HashSet;
use std::sync::Arc;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_subscriptions::{
    ReaderSubscriptionsParams, ReaderSubscriptionsResponse,
};

use crate::context::{TestContext, reading};
use crate::reader_tags_tests::{expect_error_code, signed_out_client};

/// The WordPress.com Blog, which is public.
const SITE: WpComSiteId = WpComSiteId(3584907);

async fn page(
    ctx: &TestContext,
    page: u32,
    number: u32,
) -> Result<ReaderSubscriptionsResponse, String> {
    let params = ReaderSubscriptionsParams {
        page: Some(page),
        number: Some(number),
        ..Default::default()
    };
    let response = ctx
        .client
        .reader_subscriptions()
        .list(&params)
        .await
        .map_err(|e| e.to_string())?
        .data;
    if let Some(unreadable) = response.unreadable_subscriptions.first() {
        return Err(format!(
            "A subscription couldn't be read: {}",
            unreadable.error
        ));
    }
    Ok(response)
}

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    vec![
        Trial::test("reader_subscriptions::list", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _follows = reading(&ctx.locks.follows);
                ctx.runtime.block_on(async {
                    let whole = page(&ctx, 1, 100).await?;
                    if whole
                        .subscriptions
                        .iter()
                        .any(|subscription| subscription.id.is_none())
                    {
                        return Err("A subscription has no ID".into());
                    }
                    if whole.has_next_page(100) {
                        // An account with more than a page of subscriptions: the rest of this
                        // trial compares against the whole list, so it has nothing to check.
                        return Ok(());
                    }

                    // Paging by page number: small pages list what one large page does, in the
                    // same order, and a short page isn't the end.
                    let mut paged = Vec::new();
                    let mut number = 1;
                    loop {
                        let next = page(&ctx, number, 5).await?;
                        paged.extend(
                            next.subscriptions
                                .iter()
                                .map(|subscription| subscription.id),
                        );
                        if !next.has_next_page(5) {
                            break;
                        }
                        number += 1;
                        if number > 40 {
                            return Err("The list never ended".into());
                        }
                    }
                    let listed: Vec<_> = whole.subscriptions.iter().map(|s| s.id).collect();
                    if paged != listed {
                        return Err(format!(
                            "Pages of 5 listed {} subscriptions, and one page {}",
                            paged.len(),
                            listed.len()
                        )
                        .into());
                    }
                    if paged.iter().collect::<HashSet<_>>().len() != paged.len() {
                        return Err("A subscription was listed twice".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_subscriptions::list_signed_out", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let result = signed_out_client(&ctx)
                        .reader_subscriptions()
                        .list(&ReaderSubscriptionsParams::default())
                        .await;
                    expect_error_code(result, "authorization_required")
                })
            }
        }),
        Trial::test("reader_subscriptions::is_following_site", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _follows = reading(&ctx.locks.follows);
                ctx.runtime.block_on(async {
                    // Either answer is fine for a site the account may or may not follow.
                    ctx.client
                        .reader_subscriptions()
                        .is_following_site(&SITE)
                        .await
                        .map_err(|e| e.to_string())?;

                    // A site the account is subscribed to is one it follows.
                    let subscriptions = page(&ctx, 1, 100).await?.subscriptions;
                    let Some(site) = subscriptions.iter().find_map(|s| s.blog_id) else {
                        return Ok(());
                    };
                    let status = ctx
                        .client
                        .reader_subscriptions()
                        .is_following_site(&site)
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if !status.is_following {
                        return Err("A subscribed site should be followed".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("reader_subscriptions::is_following_site_signed_out", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let result = signed_out_client(&ctx)
                        .reader_subscriptions()
                        .is_following_site(&SITE)
                        .await;
                    expect_error_code(result, "authorization_required")
                })
            }
        }),
    ]
}
