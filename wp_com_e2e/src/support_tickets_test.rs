use wp_api::wp_com::{
    client::WpComApiClient,
    support_tickets::{
        AddMessageToSupportConversationParams, ConversationId, CreateSupportTicketParams,
        SupportConversation,
    },
};

pub async fn support_tickets_test(
    client: &WpComApiClient,
    allow_writes: bool,
) -> anyhow::Result<()> {
    println!("== Support Tickets Test ==");

    let conversations = client
        .support_tickets()
        .get_support_conversation_list()
        .await?
        .data;

    println!(
        "✅ Fetch Conversation List: Found {} conversations",
        conversations.len()
    );

    for conversation in conversations {
        if let Err(e) = client
            .support_tickets()
            .get_support_conversation(&conversation.id)
            .await
        {
            println!("❌ Fetch Conversation: {} Error: {}", conversation.id, e);
            return Err(e.into());
        } else {
            println!("✅ Fetch Conversation: {}", conversation.id);
        }
    }

    if allow_writes {
        let new_conversation = create_conversation(client).await?;
        add_message_to_conversation(client, new_conversation.id).await?;
    }

    Ok(())
}

async fn create_conversation(client: &WpComApiClient) -> anyhow::Result<SupportConversation> {
    let new_conversation = client
        .support_tickets()
        .create_support_ticket(&CreateSupportTicketParams {
            subject: "Mobile Support Test Message".to_string(),
            message: "This is a test – it can be deleted without replying.".to_string(),
            application: "jetpack".to_string(),
            wpcom_site_id: None,
            tags: vec!["jetpack_mobile".to_string(), "test".to_string()],
            attachments: vec![],
        })
        .await?
        .data;
    println!("✅ Create Conversation");

    Ok(new_conversation)
}

async fn add_message_to_conversation(
    client: &WpComApiClient,
    conversation_id: ConversationId,
) -> anyhow::Result<()> {
    client
        .support_tickets()
        .add_message_to_support_conversation(
            &conversation_id,
            &AddMessageToSupportConversationParams {
                message: "Test Message".to_string(),
                attachments: vec![],
            },
        )
        .await?;
    println!("✅ Add Message to Conversation");
    Ok(())
}
