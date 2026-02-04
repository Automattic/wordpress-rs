use libtest_mimic::Trial;
use std::sync::Arc;
use wp_api::wp_com::oauth2::TokenValidationParameters;

use crate::context::TestContext;

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut trials = vec![];

    // Pre-fetch user info to determine if we can run OAuth2 test
    let user_info = ctx.runtime.block_on(async { ctx.client.me().get().await });

    trials.push(Trial::test("me::get_user_info", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                ctx.client.me().get().await.map_err(|e| e.to_string())?;
                Ok(())
            })
        }
    }));

    // Only add OAuth2 test if we have a client_id
    if let Ok(response) = user_info
        && let Some(client_id) = response.data.token_client_id
    {
        trials.push(Trial::test("me::oauth2_token_info", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    ctx.client
                        .oauth2()
                        .fetch_info(&TokenValidationParameters {
                            client_id,
                            token: ctx.token.clone(),
                        })
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(())
                })
            }
        }));
    }

    trials
}
