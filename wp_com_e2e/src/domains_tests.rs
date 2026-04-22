use crate::context::TestContext;
use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::wp_com::domains::CountryCode;

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut trials = vec![];

    trials.push(Trial::test("domains::supported_countries", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let response = ctx
                    .client
                    .domains()
                    .supported_countries()
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                if response.all.is_empty() {
                    return Err("expected non-empty `all` countries list".into());
                }

                // Verify no separator entries leaked through.
                let has_separator = response
                    .featured
                    .iter()
                    .chain(response.all.iter())
                    .any(|c| c.code.0.is_empty());
                if has_separator {
                    return Err("separator entry should be filtered out".into());
                }

                Ok(())
            })
        }
    }));

    trials.push(Trial::test("domains::supported_states_us", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let states = ctx
                    .client
                    .domains()
                    .supported_states(&CountryCode::from("US"))
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                if states.is_empty() {
                    return Err("expected non-empty states list for US".into());
                }

                Ok(())
            })
        }
    }));

    trials.push(Trial::test("domains::supported_states_empty", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let states = ctx
                    .client
                    .domains()
                    .supported_states(&CountryCode::from("DE"))
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                if !states.is_empty() {
                    return Err(format!(
                        "expected empty states list for DE, got {} entries",
                        states.len()
                    )
                    .into());
                }

                Ok(())
            })
        }
    }));

    trials
}
