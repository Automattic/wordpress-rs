use std::collections::HashMap;

use wp_api::wp_com::{
    client::WpComApiClient,
    support_bots::{
        AddMessageToBotConversationParams, BotConversation, BotId, ChatId,
        CreateBotConversationParams, GetBotConversationParams,
    },
};

pub async fn support_bots_test(client: &WpComApiClient, allow_writes: bool) -> anyhow::Result<()> {
    let bot_id = BotId("jetpack-chat-mobile".to_string());
    println!("== Support Bots Test ==");

    let conversations = client
        .support_bots()
        .get_bot_converation_list(&bot_id)
        .await?
        .data;

    println!(
        "✅ Fetch Conversation List: Found {} conversations",
        conversations.len()
    );

    for conversation in conversations {
        let chat_id = ChatId(conversation.chat_id);

        if let Err(e) = client
            .support_bots()
            .get_bot_conversation(&bot_id, &chat_id, &GetBotConversationParams::default())
            .await
        {
            println!(
                "❌ Fetch Conversation: {} Error: {}",
                conversation.chat_id, e
            );
            return Err(e.into());
        } else {
            println!("✅ Fetch Conversation: {}", conversation.chat_id);
        }
    }

    if allow_writes {
        let new_conversation = create_conversation(client, &bot_id).await?;
        let chat_id = ChatId(new_conversation.chat_id);
        add_message_to_conversation(client, &bot_id, chat_id).await?;
    }

    Ok(())
}

async fn create_conversation(
    client: &WpComApiClient,
    bot_id: &BotId,
) -> anyhow::Result<BotConversation> {
    let new_conversation = client
        .support_bots()
        .create_bot_conversation(
            bot_id,
            &CreateBotConversationParams {
                message: "This is a test – it can be deleted without replying.".to_string(),
                user_id: None,
            },
        )
        .await?
        .data;

    println!("✅ Create Conversation");

    Ok(new_conversation)
}

async fn add_message_to_conversation(
    client: &WpComApiClient,
    bot_id: &BotId,
    conversation_id: ChatId,
) -> anyhow::Result<()> {
    client
        .support_bots()
        .add_message_to_bot_conversation(
            bot_id,
            &conversation_id,
            &AddMessageToBotConversationParams {
                message: "Test Message".to_string(),
                context: HashMap::new(),
            },
        )
        .await?;

    println!("✅ Add Message to Conversation");
    Ok(())
}
