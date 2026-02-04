use libtest_mimic::Trial;
use std::collections::HashMap;
use std::sync::Arc;

use wp_api::wp_com::support_bots::{
    AddMessageToBotConversationParams, BotId, ChatId, CreateBotConversationParams,
    GetBotConversationParams, ListBotConversationsParams,
};

use crate::context::TestContext;

const BOT_ID: &str = "jetpack-chat-mobile";

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    let mut trials = vec![];

    let bot_id = BotId(BOT_ID.to_string());

    // Pre-fetch conversations during test collection
    let conversations_result = ctx.runtime.block_on(async {
        ctx.client
            .support_bots()
            .get_bot_conversation_list(&bot_id, &ListBotConversationsParams::default())
            .await
    });

    trials.push(Trial::test("support_bots::list_conversations", {
        let ctx = Arc::clone(&ctx);
        let bot_id = bot_id.clone();
        move || {
            ctx.runtime.block_on(async {
                ctx.client
                    .support_bots()
                    .get_bot_conversation_list(&bot_id, &ListBotConversationsParams::default())
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(())
            })
        }
    }));

    if let Ok(response) = conversations_result {
        for conversation in response.data {
            let chat_id = ChatId(conversation.chat_id);
            trials.push(Trial::test(
                format!("support_bots::get_conversation::{}", conversation.chat_id),
                {
                    let ctx = Arc::clone(&ctx);
                    let bot_id = bot_id.clone();
                    move || {
                        ctx.runtime.block_on(async {
                            ctx.client
                                .support_bots()
                                .get_bot_conversation(
                                    &bot_id,
                                    &chat_id,
                                    &GetBotConversationParams::default(),
                                )
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
        Trial::test("support_bots::create_conversation", {
            let ctx = Arc::clone(&ctx);
            let bot_id = bot_id.clone();
            move || {
                ctx.runtime.block_on(async {
                    ctx.client
                        .support_bots()
                        .create_bot_conversation(
                            &bot_id,
                            &CreateBotConversationParams {
                                message: "This is a test – it can be deleted without replying."
                                    .to_string(),
                                user_id: None,
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

    trials.push(
        Trial::test("support_bots::create_and_add_message", {
            let ctx = Arc::clone(&ctx);
            let bot_id = bot_id.clone();
            move || {
                ctx.runtime.block_on(async {
                    // Create a new conversation
                    let new_conversation = ctx
                        .client
                        .support_bots()
                        .create_bot_conversation(
                            &bot_id,
                            &CreateBotConversationParams {
                                message: "This is a test – it can be deleted without replying."
                                    .to_string(),
                                user_id: None,
                            },
                        )
                        .await
                        .map_err(|e| e.to_string())?;

                    let chat_id = ChatId(new_conversation.data.chat_id);

                    // Add a message to the conversation
                    ctx.client
                        .support_bots()
                        .add_message_to_bot_conversation(
                            &bot_id,
                            &chat_id,
                            &AddMessageToBotConversationParams {
                                message: "Test Message".to_string(),
                                context: HashMap::new(),
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
