use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::{
    posts::PostId,
    wp_com::{
        WpComSiteId,
        sites::SitesListParams,
        stats_top_posts::{StatsTopPostsParams, StatsTopPostsPeriod},
    },
};

use crate::context::TestContext;

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut trials = vec![];

    // Pre-fetch sites during test collection
    let sites_result = ctx
        .runtime
        .block_on(async { ctx.client.sites().get(&SitesListParams::default()).await });

    if let Ok(response) = sites_result {
        let sites = response.data.sites;

        for site in &sites {
            let site_id = site.id;

            // The endpoint needs a real post, so borrow one from the site's top
            // posts. Sites without a viewed post are skipped.
            let Some(post_id) = most_viewed_post_id(&ctx, &site_id) else {
                continue;
            };

            trials.push(Trial::test(
                format!("post_stats::get_stats_post::{}", site_id),
                {
                    let ctx = Arc::clone(&ctx);
                    move || {
                        ctx.runtime.block_on(async {
                            ctx.client
                                .stats_post()
                                .get_stats_post(&site_id, &post_id)
                                .await
                                .map_err(|e| e.to_string())?;
                            Ok(())
                        })
                    }
                },
            ));
        }
    }

    trials
}

fn most_viewed_post_id(ctx: &Arc<TestContext>, site_id: &WpComSiteId) -> Option<PostId> {
    // Look back over several years rather than the default single day, so quiet
    // test sites still yield a post.
    let params = StatsTopPostsParams {
        period: Some(StatsTopPostsPeriod::Year),
        num: Some(10),
        ..Default::default()
    };

    let response = ctx.runtime.block_on(async {
        ctx.client
            .stats_top_posts()
            .get_stats_top_posts(site_id, &params)
            .await
    });

    response
        .ok()?
        .data
        .summary?
        .postviews
        .iter()
        // Id 0 is the homepage pseudo-entry, which isn't a real post.
        .find(|post_view| post_view.id != 0)
        .map(|post_view| PostId(post_view.id as i64))
}
