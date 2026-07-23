use crate::context::TestContext;
use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::wp_com::purchases::{ExpiryStatus, PaymentType, PurchaseSubscriptionStatus};

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut trials = vec![];

    let site_id = ctx.site_id;

    trials.push(Trial::test("purchases::site_purchases", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                // Fetching and deserializing the live response is the main assertion
                // here: a test site may legitimately have zero purchases, so we don't
                // require a non-empty list.
                let purchases = ctx
                    .client
                    .purchases()
                    .site_purchases(&site_id)
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                // Surface any live enum values the model doesn't yet know about, so
                // gaps show up as failures rather than silently landing in `Other`.
                for purchase in &purchases {
                    if let PurchaseSubscriptionStatus::Other(s) = &purchase.subscription_status {
                        return Err(format!("unexpected subscription_status '{s}'").into());
                    }
                    if let ExpiryStatus::Other(s) = &purchase.expiry_status {
                        return Err(format!("unexpected expiry_status '{s}'").into());
                    }
                    if let Some(PaymentType::Other(s)) = &purchase.payment_type {
                        return Err(format!("unexpected payment_type '{s}'").into());
                    }
                }

                Ok(())
            })
        }
    }));

    trials
}
