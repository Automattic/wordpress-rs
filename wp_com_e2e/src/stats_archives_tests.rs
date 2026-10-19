use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::wp_com::{sites::SitesListParams, stats_archives::StatsArchivesParams};

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
                format!("archives::get_stats_archives::{}", site_id),
                {
                    let ctx = Arc::clone(&ctx);
                    move || {
                        ctx.runtime.block_on(async {
                            ctx.client
                                .stats_archives()
                                .get_stats_archives(&site_id, &StatsArchivesParams::default())
                                .await
                                .map_err(|e| e.to_string())?;
                            Ok(())
                        })
                    }
                },
            ));

            // The per-day breakdown has a different response shape from the summary, so exercise
            // both.
            trials.push(Trial::test(
                format!("archives::get_stats_archives_by_day::{}", site_id),
                {
                    let ctx = Arc::clone(&ctx);
                    move || {
                        ctx.runtime.block_on(async {
                            ctx.client
                                .stats_archives()
                                .get_stats_archives(
                                    &site_id,
                                    &StatsArchivesParams {
                                        summarize: false,
                                        num: Some(7),
                                        ..Default::default()
                                    },
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
