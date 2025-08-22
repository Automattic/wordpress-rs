use wp_api::wp_com::{
    client::WpComApiClient,
    support_tickets::{AddMessageToSupportConversationParams, CreateSupportTicketParams},
};

pub async fn support_tickets_test(client: &WpComApiClient) -> anyhow::Result<()> {
    println!("== Support Tickets Test ==");
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

    client
        .support_tickets()
        .add_message_to_support_conversation(
            &new_conversation.id,
            &AddMessageToSupportConversationParams {
                message: "Test Message".to_string(),
                attachments: vec![],
            },
        )
        .await?;
    println!("✅ Add Message to Conversation");

    client
        .support_tickets()
        .get_support_conversation(&new_conversation.id)
        .await?;
    println!("✅ Fetch Conversation");

    client
        .support_tickets()
        .get_support_conversation_list()
        .await?;
    println!("✅ Fetch Conversation List");

    Ok(())
}
