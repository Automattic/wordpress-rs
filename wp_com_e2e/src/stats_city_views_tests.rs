use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::{
    api_error::WpApiError,
    wp_com::{sites::SitesListParams, stats_city_views::StatsCityViewsParams},
};

use crate::context::TestContext;

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut trials = vec![];

    let sites_result = ctx
        .runtime
        .block_on(async { ctx.client.sites().get(&SitesListParams::default()).await });

    if let Ok(response) = sites_result {
        let sites = response.data.sites;
        let site_ids: Vec<_> = sites.iter().map(|s| s.id).collect();

        // City-level location views require a WP.com plan with access to
        // region-specific stats. The test account contains a mix of sites:
        // some with premium plans (where the endpoint returns city view data)
        // and some without (where the API returns an "unauthorized" error).
        //
        // Instead of testing each site individually (which would fail for
        // non-premium sites), we validate that the full set of sites produces
        // at least one successful response AND at least one "unauthorized"
        // error, confirming both code paths are exercised.
        trials.push(Trial::test(
            "city_views::get_stats_city_views".to_string(),
            {
                let ctx = Arc::clone(&ctx);
                move || {
                    let mut has_success = false;
                    let mut has_unauthorized = false;

                    for site_id in &site_ids {
                        let result = ctx.runtime.block_on(async {
                            ctx.client
                                .stats_city_views()
                                .get_stats_city_views(site_id, &StatsCityViewsParams::default())
                                .await
                        });

                        match &result {
                            Ok(_) => has_success = true,
                            Err(WpApiError::UnknownError { response, .. })
                                if response.contains("unauthorized") =>
                            {
                                has_unauthorized = true;
                            }
                            Err(e) => {
                                return Err(
                                    format!("Unexpected error for site {site_id}: {e:?}").into()
                                );
                            }
                        }
                    }

                    if !has_success {
                        return Err(
                            "Expected at least one site with premium access to return city views"
                                .into(),
                        );
                    }
                    if !has_unauthorized {
                        return Err(
                            "Expected at least one non-premium site to return unauthorized".into(),
                        );
                    }

                    Ok(())
                }
            },
        ));
    }

    trials
}
