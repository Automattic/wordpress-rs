use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::wp_com::{sites::SitesListParams, stats_summary::StatsSummaryParams};

use crate::context::TestContext;

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut trials = vec![];

    let sites_result = ctx
        .runtime
        .block_on(async { ctx.client.sites().get(&SitesListParams::default()).await });

    if let Ok(response) = sites_result {
        let sites = response.data.sites;

        for site in &sites {
            let site_id = site.id;
            trials.push(Trial::test(
                format!("summary::get_stats_summary::{}", site_id),
                {
                    let ctx = Arc::clone(&ctx);
                    move || {
                        ctx.runtime.block_on(async {
                            ctx.client
                                .stats_summary()
                                .get_stats_summary(&site_id, &StatsSummaryParams::default())
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
