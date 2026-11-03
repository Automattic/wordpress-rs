use crate::context::TestContext;
use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::wp_com::site_plans::SitePlansParams;

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut trials = vec![];

    // GET /sites/{siteId}/plans

    let site_id = ctx.site_id;

    trials.push(Trial::test("site_plans::list", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let plans = ctx
                    .client
                    .site_plans()
                    .list(&site_id, &SitePlansParams::default())
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                if plans.is_empty() {
                    return Err("expected at least one plan for the test site".into());
                }

                // The response is keyed by product ID, and the key repeats the
                // entry's own `product_id`.
                for (product_id, plan) in &plans {
                    if *product_id != plan.product_id {
                        return Err(format!(
                            "plan keyed as {product_id} reports product_id {}",
                            plan.product_id
                        )
                        .into());
                    }
                }

                // A site is always on exactly one plan, even if it's the free one.
                let current: Vec<_> = plans
                    .values()
                    .filter(|plan| plan.current_plan.is_some())
                    .collect();
                if current.len() != 1 {
                    return Err(format!(
                        "expected exactly one current plan, got {}",
                        current.len()
                    )
                    .into());
                }

                // The current plan is the only entry without upgrade/downgrade
                // availability, since there's nothing to transition from.
                if current[0].transition.is_some() {
                    return Err("the current plan should not report a transition".into());
                }

                Ok(())
            })
        }
    }));

    trials
}
