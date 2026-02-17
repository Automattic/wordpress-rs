use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::{
    api_error::{RequestExecutionErrorReason, WpApiError},
    wp_com::{sites::SitesListParams, stats_city_views::StatsCityViewsParams},
};

use crate::context::TestContext;

fn is_not_authorized(err: &WpApiError) -> bool {
    matches!(
        err,
        WpApiError::UnknownError { response, .. } if response.contains("unauthorized")
    ) || matches!(
        err,
        WpApiError::RequestExecutionFailed {
            reason: RequestExecutionErrorReason::HttpForbiddenError { .. }
                | RequestExecutionErrorReason::HttpAuthenticationRejectedError { .. },
            ..
        }
    )
}

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut trials = vec![];

    let sites_result = ctx
        .runtime
        .block_on(async { ctx.client.sites().get(&SitesListParams::default()).await });

    if let Ok(response) = sites_result {
        let sites = response.data.sites;

        // City-level location views require a WP.com plan with access to
        // region-specific stats. The test account contains a mix of sites:
        // some with premium plans (where the endpoint returns city view data)
        // and some without (where the API returns an "unauthorized" error).
        //
        // We preflight each site to determine authorization, marking
        // unauthorized sites as ignored so they appear in test output
        // without causing failures.
        let mut has_authorized_site = false;
        let mut has_unauthorized_site = false;

        for site in &sites {
            let site_id = site.id;

            // Preflight the request to determine if this site is authorized.
            let preflight = ctx.runtime.block_on(async {
                ctx.client
                    .stats_city_views()
                    .get_stats_city_views(&site_id, &StatsCityViewsParams::default())
                    .await
            });

            let is_ignored = preflight.as_ref().is_err_and(is_not_authorized);
            if is_ignored {
                has_unauthorized_site = true;
            } else {
                has_authorized_site = true;
            }

            trials.push(
                Trial::test(
                    format!("city_views::get_stats_city_views::{site_id}"),
                    {
                        move || match preflight {
                            Ok(_) => Ok(()),
                            Err(e) if is_not_authorized(&e) => Ok(()),
                            Err(e) => {
                                Err(format!("Unexpected error for site {site_id}: {e:?}").into())
                            }
                        }
                    },
                )
                .with_ignored_flag(is_ignored),
            );
        }

        trials.push(Trial::test(
            "city_views::at_least_one_site_authorized".to_string(),
            move || {
                if has_authorized_site {
                    Ok(())
                } else {
                    Err("Expected at least one site with premium access to return city views"
                        .into())
                }
            },
        ));

        trials.push(Trial::test(
            "city_views::at_least_one_site_unauthorized".to_string(),
            move || {
                if has_unauthorized_site {
                    Ok(())
                } else {
                    Err("Expected at least one non-premium site to return unauthorized".into())
                }
            },
        ));
    }

    trials
}
