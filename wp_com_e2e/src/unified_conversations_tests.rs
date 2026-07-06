use libtest_mimic::Trial;
use std::sync::Arc;

use wp_api::wp_com::unified_conversations::ReplyToUnifiedConversationParams;

use crate::context::TestContext;

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut trials = vec![];

    // Pre-fetch the conversation list during test collection so we can fan out a
    // `get_conversation` round-trip per conversation. These hit the live API and
    // therefore verify the real response shape (bare array for the list, bare
    // object for a single conversation) rather than only the hand-written
    // fixtures.
    let conversations_result = ctx.runtime.block_on(async {
        ctx.client
            .unified_conversations()
            .get_unified_conversation_list()
            .await
    });

    trials.push(Trial::test("unified_conversations::list_conversations", {
        let ctx = Arc::clone(&ctx);
        move || {
            ctx.runtime.block_on(async {
                ctx.client
                    .unified_conversations()
                    .get_unified_conversation_list()
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(())
            })
        }
    }));

    if let Ok(response) = &conversations_result {
        for conversation in &response.data {
            let conversation_id = conversation.id;
            trials.push(Trial::test(
                format!("unified_conversations::get_conversation::{conversation_id}"),
                {
                    let ctx = Arc::clone(&ctx);
                    move || {
                        ctx.runtime.block_on(async {
                            ctx.client
                                .unified_conversations()
                                .get_unified_conversation(&conversation_id)
                                .await
                                .map_err(|e| e.to_string())?;
                            Ok(())
                        })
                    }
                },
            ));
        }
    }

    // Write test - marked as ignored (run with --ignored or --include-ignored).
    // Replies to the first conversation that still accepts replies and asserts
    // that the POST echoes back the full conversation with our new message as
    // the last entry (verifying the `output = UnifiedConversation` assumption).
    let replyable_id = conversations_result.ok().and_then(|response| {
        response
            .data
            .into_iter()
            .find(|conversation| conversation.can_accept_reply)
            .map(|conversation| conversation.id)
    });

    trials.push(
        Trial::test("unified_conversations::reply_to_conversation", {
            let ctx = Arc::clone(&ctx);
            move || {
                ctx.runtime.block_on(async {
                    let Some(conversation_id) = replyable_id else {
                        // No conversation currently accepts replies; nothing to exercise.
                        return Ok(());
                    };

                    let message =
                        "This is a test reply – it can be deleted without replying.".to_string();
                    let conversation = ctx
                        .client
                        .unified_conversations()
                        .reply_to_unified_conversation(
                            &conversation_id,
                            &ReplyToUnifiedConversationParams {
                                message: message.clone(),
                                encrypted_log_ids: vec![],
                                attachments: vec![],
                            },
                        )
                        .await
                        .map_err(|e| e.to_string())?;

                    // The reply endpoint returns the full conversation; our newly
                    // posted message should be the last entry.
                    let reply = conversation
                        .data
                        .messages
                        .last()
                        .ok_or_else(|| "reply response contained no messages".to_string())?;
                    if reply.message != message {
                        return Err(format!(
                            "expected last message to be the posted reply, got: {:?}",
                            reply.message
                        )
                        .into());
                    }
                    Ok(())
                })
            }
        })
        .with_ignored_flag(true),
    );

    trials
}
