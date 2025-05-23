use anyhow::{Ok, Result};
use async_trait::async_trait;
use wp_api::wp_com::{
    client::WpComApiClient,
    support_tickets::{AddMessageToSupportConversationParams, CreateSupportTicketParams},
};

use crate::Testable;

pub struct SupportTicketsTest<'a> {
    pub client: &'a WpComApiClient,
}

#[async_trait]
impl Testable for SupportTicketsTest<'_> {
    async fn test(&self) -> Result<(), anyhow::Error> {
        println!("== Support Tickets Test ==");
        let new_conversation = self
            .client
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

        self.client
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

        self.client
            .support_tickets()
            .get_support_conversation(&new_conversation.id)
            .await?;
        println!("✅ Fetch Conversation");

        self.client
            .support_tickets()
            .get_support_conversation_list()
            .await?;
        println!("✅ Fetch Conversation List");

        Ok(())
    }
}
