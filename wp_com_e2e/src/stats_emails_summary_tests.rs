use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::wp_com::{sites::SitesListParams, stats_emails_summary::StatsEmailsSummaryParams};

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
            trials.push(Trial::test(
                format!("emails_summary::get_stats_emails_summary::{}", site_id),
                {
                    let ctx = Arc::clone(&ctx);
                    move || {
                        ctx.runtime.block_on(async {
                            ctx.client
                                .stats_emails_summary()
                                .get_stats_emails_summary(
                                    &site_id,
                                    &StatsEmailsSummaryParams::default(),
                                )
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
