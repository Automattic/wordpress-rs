use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::wp_com::sites::SitesListParams;

use crate::context::TestContext;

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut trials = vec![];

    // Pre-fetch sites during test collection
    let sites_result = ctx
        .runtime
        .block_on(async { ctx.client.sites().get(&SitesListParams::default()).await });

    trials.push(Trial::test("sites::list", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                ctx.client
                    .sites()
                    .get(&SitesListParams::default())
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(())
            })
        }
    }));

    if let Ok(response) = sites_result {
        let sites = response.data.sites;

        for site in &sites {
            let site_id = site.id;
            trials.push(Trial::test(format!("sites::get_by_id::{}", site_id), {
                let ctx = Arc::clone(&ctx);
                move || {
                    ctx.runtime.block_on(async {
                        ctx.client
                            .sites()
                            .get_site_by_id(&site_id)
                            .await
                            .map_err(|e| e.to_string())?;
                        Ok(())
                    })
                }
            }));
        }

        for site in &sites {
            if let Some(slug) = &site.slug {
                let slug = slug.clone();
                trials.push(Trial::test(format!("sites::get_by_slug::{}", slug), {
                    let ctx = Arc::clone(&ctx);
                    let slug = slug.clone();
                    move || {
                        ctx.runtime.block_on(async {
                            ctx.client
                                .sites()
                                .get_site_by_slug(&slug)
                                .await
                                .map_err(|e| e.to_string())?;
                            Ok(())
                        })
                    }
                }));
            }
        }
    }

    trials
}
