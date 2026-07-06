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

    trials.push(Trial::test("me::transactions_supported_countries", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let response = ctx
                    .client
                    .me()
                    .transactions_supported_countries()
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                if response.all.is_empty() {
                    return Err("expected non-empty countries list".into());
                }

                // No separator entries should leak through.
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

    trials.push(Trial::test("me::domain_contact_information", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                let contact = ctx
                    .client
                    .me()
                    .domain_contact_information()
                    .await
                    .map_err(|e| e.to_string())?
                    .data;

                // The test account should have at least an email.
                if contact.email.is_none() {
                    return Err("expected non-null email in domain contact info".into());
                }

                Ok(())
            })
        }
    }));

    trials
}
