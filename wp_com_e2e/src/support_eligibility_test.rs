use libtest_mimic::Trial;
use std::sync::Arc;

use crate::context::TestContext;

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    vec![Trial::test("support_eligibility::get", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                ctx.client
                    .support_eligibility()
                    .get_support_eligibility()
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(())
            })
        }
    })]
}
