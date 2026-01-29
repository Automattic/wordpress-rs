use libtest_mimic::Trial;
use std::sync::Arc;
use uuid::Uuid;

use wp_api::wp_com::support_tickets::{
    AddMessageToSupportConversationParams, CreateSupportTicketParams,
};

use crate::context::TestContext;

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut trials = vec![];

    // Pre-fetch conversations during test collection
    let conversations_result = ctx.runtime.block_on(async {
        ctx.client
            .support_tickets()
            .get_support_conversation_list()
            .await
    });

    trials.push(Trial::test("support_tickets::list_conversations", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                ctx.client
                    .support_tickets()
                    .get_support_conversation_list()
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(())
            })
        }
    }));

    if let Ok(response) = conversations_result {
        for conversation in response.data {
            let conversation_id = conversation.id;
            trials.push(Trial::test(
                format!("support_tickets::get_conversation::{}", conversation_id),
                {
                    let ctx = Arc::clone(&ctx);
                    move || {
                        ctx.runtime.block_on(async {
                            ctx.client
                                .support_tickets()
                                .get_support_conversation(&conversation_id)
                                .await
                                .map_err(|e| e.to_string())?;
                            Ok(())
                        })
                    }
                },
            ));
        }
    }

    // Write tests - marked as ignored (run with --ignored or --include-ignored)
    trials.push(
        Trial::test("support_tickets::create_ticket", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    ctx.client
                        .support_tickets()
                        .create_support_ticket(&CreateSupportTicketParams {
                            subject: "Mobile Support Test Message".to_string(),
                            message: "This is a test – it can be deleted without replying."
                                .to_string(),
                            application: "jetpack".to_string(),
                            wpcom_site_id: None,
                            tags: vec!["jetpack_mobile".to_string(), "test".to_string()],
                            attachments: vec![],
                            encrypted_log_ids: vec![Uuid::new_v4().to_string()],
                        })
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(())
                })
            }
        })
        .with_ignored_flag(true),
    );

    trials.push(
        Trial::test("support_tickets::create_and_add_message", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    // Create a new ticket
                    let new_conversation = ctx
                        .client
                        .support_tickets()
                        .create_support_ticket(&CreateSupportTicketParams {
                            subject: "Mobile Support Test Message".to_string(),
                            message: "This is a test – it can be deleted without replying."
                                .to_string(),
                            application: "jetpack".to_string(),
                            wpcom_site_id: None,
                            tags: vec!["jetpack_mobile".to_string(), "test".to_string()],
                            attachments: vec![],
                            encrypted_log_ids: vec![Uuid::new_v4().to_string()],
                        })
                        .await
                        .map_err(|e| e.to_string())?;

                    // Add a message to the conversation
                    ctx.client
                        .support_tickets()
                        .add_message_to_support_conversation(
                            &new_conversation.data.id,
                            &AddMessageToSupportConversationParams {
                                message: "Test Message".to_string(),
                                attachments: vec![],
                            },
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(())
                })
            }
        })
        .with_ignored_flag(true),
    );

    trials
}
